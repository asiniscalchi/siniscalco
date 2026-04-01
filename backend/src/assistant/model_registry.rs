use std::{collections::BTreeSet, sync::Arc, time::Duration};

use sqlx::SqlitePool;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::current_utc_timestamp;
use crate::storage::StorageError;

use super::types::{AssistantModelRefreshError, AssistantModelsResponse, OpenAiModelsListResponse};

// ── Constants ─────────────────────────────────────────────────────────────────

pub const SETTING_SELECTED_MODEL: &str = "assistant.selected_model";
pub const SETTING_SYSTEM_PROMPT: &str = "assistant.system_prompt";
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
pub const MOCK_BACKEND_MODEL: &str = "mock-backend";
pub const MAX_CONCURRENT_CHAT_REQUESTS: usize = 5;
const MODEL_REFRESH_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

const OPENAI_MODELS_URL: &str = "https://api.openai.com/v1/models";

pub const fn openai_models_url() -> &'static str {
    OPENAI_MODELS_URL
}

// ── Registry ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssistantModelRegistry {
    pub models: Vec<String>,
    pub selected_model: String,
    pub openai_enabled: bool,
    pub last_refreshed_at: Option<String>,
    pub refresh_error: Option<String>,
}

impl AssistantModelRegistry {
    pub fn mock_backend() -> Self {
        Self {
            models: vec![MOCK_BACKEND_MODEL.to_string()],
            selected_model: MOCK_BACKEND_MODEL.to_string(),
            openai_enabled: false,
            last_refreshed_at: None,
            refresh_error: None,
        }
    }

    pub fn openai_defaults() -> Self {
        Self {
            selected_model: DEFAULT_OPENAI_MODEL.to_string(),
            models: vec![DEFAULT_OPENAI_MODEL.to_string()],
            openai_enabled: true,
            last_refreshed_at: None,
            refresh_error: None,
        }
    }

    pub fn to_response(&self) -> AssistantModelsResponse {
        AssistantModelsResponse {
            models: self.models.clone(),
            selected_model: self.selected_model.clone(),
            openai_enabled: self.openai_enabled,
            last_refreshed_at: self.last_refreshed_at.clone(),
            refresh_error: self.refresh_error.clone(),
        }
    }
}

pub type SharedAssistantModelRegistry = Arc<RwLock<AssistantModelRegistry>>;
pub type SharedAssistantChatSemaphore = Arc<Semaphore>;

pub fn new_assistant_chat_semaphore() -> SharedAssistantChatSemaphore {
    Arc::new(Semaphore::new(MAX_CONCURRENT_CHAT_REQUESTS))
}

pub fn new_shared_assistant_model_registry(
    openai_api_key: Option<&str>,
    persisted_model: Option<&str>,
) -> SharedAssistantModelRegistry {
    Arc::new(RwLock::new(
        if openai_api_key
            .map(str::trim)
            .is_some_and(|api_key| !api_key.is_empty())
        {
            let mut registry = AssistantModelRegistry::openai_defaults();
            if let Some(model) = persisted_model.map(str::trim).filter(|m| !m.is_empty()) {
                registry.selected_model = model.to_string();
            }
            registry
        } else {
            AssistantModelRegistry::mock_backend()
        },
    ))
}

pub async fn load_selected_model_setting(
    pool: &SqlitePool,
) -> Result<Option<String>, StorageError> {
    crate::storage::settings::get_app_setting(pool, SETTING_SELECTED_MODEL).await
}

pub async fn spawn_assistant_model_refresh_task(
    assistant_models: SharedAssistantModelRegistry,
    http_client: reqwest::Client,
    openai_api_key: Option<String>,
    openai_models_url: String,
) {
    if openai_api_key
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
    {
        *assistant_models.write().await = AssistantModelRegistry::mock_backend();
        return;
    }

    tokio::spawn(async move {
        loop {
            match refresh_assistant_model_registry(
                &assistant_models,
                &http_client,
                openai_api_key.as_deref(),
                &openai_models_url,
            )
            .await
            {
                Ok(()) => {
                    let registry = assistant_models.read().await;
                    info!(
                        model_count = registry.models.len(),
                        selected_model = %registry.selected_model,
                        "assistant model refresh succeeded"
                    );
                }
                Err(error) => {
                    warn!(error = %error, "assistant model refresh failed");
                    assistant_models.write().await.refresh_error = Some(error.to_string());
                }
            }

            sleep(MODEL_REFRESH_INTERVAL).await;
        }
    });
}

pub async fn refresh_assistant_model_registry(
    assistant_models: &SharedAssistantModelRegistry,
    http_client: &reqwest::Client,
    openai_api_key: Option<&str>,
    openai_models_url: &str,
) -> Result<(), AssistantModelRefreshError> {
    let Some(openai_api_key) = openai_api_key
        .map(str::trim)
        .filter(|api_key| !api_key.is_empty())
    else {
        *assistant_models.write().await = AssistantModelRegistry::mock_backend();
        return Ok(());
    };

    let fetched_model_ids =
        fetch_openai_model_ids(http_client, openai_api_key, openai_models_url).await?;
    let available_models = fetched_model_ids
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    if available_models.is_empty() {
        return Err(AssistantModelRefreshError::Provider(
            "OpenAI model refresh failed: no models are currently available".to_string(),
        ));
    }

    let current_registry = assistant_models.read().await.clone();
    let selected_model = if available_models.contains(&current_registry.selected_model) {
        current_registry.selected_model
    } else {
        available_models
            .iter()
            .find(|model| model.as_str() == DEFAULT_OPENAI_MODEL)
            .cloned()
            .unwrap_or_else(|| available_models[0].clone())
    };

    let refreshed_at = current_utc_timestamp().map_err(|_| {
        AssistantModelRefreshError::Config("assistant model refresh failed: invalid timestamp")
    })?;

    *assistant_models.write().await = AssistantModelRegistry {
        models: available_models,
        selected_model,
        openai_enabled: true,
        last_refreshed_at: Some(refreshed_at),
        refresh_error: None,
    };

    Ok(())
}

async fn fetch_openai_model_ids(
    http_client: &reqwest::Client,
    openai_api_key: &str,
    openai_models_url: &str,
) -> Result<Vec<String>, AssistantModelRefreshError> {
    let response = http_client
        .get(openai_models_url)
        .bearer_auth(openai_api_key)
        .send()
        .await
        .map_err(|error| {
            AssistantModelRefreshError::Provider(format!("OpenAI model refresh failed: {error}"))
        })?;

    let http_status = response.status();
    if !http_status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AssistantModelRefreshError::Provider(format!(
            "OpenAI model refresh failed with {http_status}: {body}"
        )));
    }

    let payload = response
        .json::<OpenAiModelsListResponse>()
        .await
        .map_err(|error| {
            AssistantModelRefreshError::Provider(format!("OpenAI model refresh failed: {error}"))
        })?;

    Ok(payload.data.into_iter().map(|model| model.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_backend_registry_defaults() {
        let r = AssistantModelRegistry::mock_backend();
        assert_eq!(r.selected_model, MOCK_BACKEND_MODEL);
        assert!(!r.openai_enabled);
        assert_eq!(r.models, vec![MOCK_BACKEND_MODEL]);
    }

    #[test]
    fn openai_defaults_registry() {
        let r = AssistantModelRegistry::openai_defaults();
        assert_eq!(r.selected_model, DEFAULT_OPENAI_MODEL);
        assert!(r.openai_enabled);
    }

    #[test]
    fn to_response_round_trips_fields() {
        let r = AssistantModelRegistry {
            models: vec!["gpt-4o".to_string()],
            selected_model: "gpt-4o".to_string(),
            openai_enabled: true,
            last_refreshed_at: Some("2024-01-01T00:00:00Z".to_string()),
            refresh_error: None,
        };
        let resp = r.to_response();
        assert_eq!(resp.models, r.models);
        assert_eq!(resp.selected_model, r.selected_model);
        assert_eq!(resp.openai_enabled, r.openai_enabled);
        assert_eq!(resp.last_refreshed_at, r.last_refreshed_at);
        assert_eq!(resp.refresh_error, r.refresh_error);
    }

    #[test]
    fn new_shared_registry_with_no_api_key_is_mock() {
        let registry = new_shared_assistant_model_registry(None, None);
        let r = registry.try_read().expect("no contention");
        assert_eq!(r.selected_model, MOCK_BACKEND_MODEL);
        assert!(!r.openai_enabled);
    }

    #[test]
    fn new_shared_registry_with_api_key_is_openai() {
        let registry = new_shared_assistant_model_registry(Some("sk-test"), None);
        let r = registry.try_read().expect("no contention");
        assert_eq!(r.selected_model, DEFAULT_OPENAI_MODEL);
        assert!(r.openai_enabled);
    }

    #[test]
    fn new_shared_registry_persisted_model_overrides_default() {
        let registry = new_shared_assistant_model_registry(Some("sk-test"), Some("gpt-4o"));
        let r = registry.try_read().expect("no contention");
        assert_eq!(r.selected_model, "gpt-4o");
    }

    #[tokio::test]
    async fn refresh_with_no_api_key_switches_to_mock() {
        let registry = new_shared_assistant_model_registry(Some("sk-test"), None);
        let http_client = reqwest::Client::new();
        // passing no api key should immediately flip to mock
        let result =
            refresh_assistant_model_registry(&registry, &http_client, None, OPENAI_MODELS_URL)
                .await;
        assert!(result.is_ok());
        let r = registry.read().await;
        assert_eq!(r.selected_model, MOCK_BACKEND_MODEL);
        assert!(!r.openai_enabled);
    }
}
