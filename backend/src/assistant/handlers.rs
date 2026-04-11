use std::convert::Infallible;

use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{
        IntoResponse,
        sse::{Event, Sse},
    },
};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

use crate::AppState;

use super::mock::{build_mock_reply, latest_user_prompt_from};
use super::model_registry::SETTING_SYSTEM_PROMPT;
use super::model_registry::{MOCK_BACKEND_MODEL, SETTING_REASONING_EFFORT, SETTING_SELECTED_MODEL};
use super::openai_client::DEFAULT_SYSTEM_PROMPT;
use super::openai_client::{openai_responses_streaming, send_sse_event};
use super::types::{
    AssistantChatErrorResponse, AssistantChatMessageRequest, AssistantChatRequest,
    AssistantModelSelectionRequest, AssistantModelsResponse, ReasoningEffort,
    ReasoningEffortRequest, SystemPromptResponse, UpdateSystemPromptRequest,
};

pub async fn models(State(state): State<AppState>) -> impl IntoResponse {
    let response = state.assistant_models.read().await.to_response();
    (
        [(header::CACHE_CONTROL, "public, max-age=300")],
        Json(response),
    )
}

pub async fn select_model(
    State(state): State<AppState>,
    Json(request): Json<AssistantModelSelectionRequest>,
) -> Result<Json<AssistantModelsResponse>, (StatusCode, Json<AssistantChatErrorResponse>)> {
    let requested_model = request.model.trim();
    if requested_model.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AssistantChatErrorResponse {
                error: "assistant model cannot be empty".to_string(),
            }),
        ));
    }

    let mut registry = state.assistant_models.write().await;
    if !registry.models.iter().any(|model| model == requested_model) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AssistantChatErrorResponse {
                error: format!("assistant model is not available: {requested_model}"),
            }),
        ));
    }

    registry.selected_model = requested_model.to_string();
    let response = registry.to_response();
    drop(registry);

    if let Err(error) = crate::storage::settings::set_app_setting(
        &state.pool,
        SETTING_SELECTED_MODEL,
        requested_model,
    )
    .await
    {
        tracing::warn!(error = %error, "failed to persist selected assistant model");
    }

    Ok(Json(response))
}

pub async fn set_reasoning_effort(
    State(state): State<AppState>,
    Json(request): Json<ReasoningEffortRequest>,
) -> Result<Json<AssistantModelsResponse>, (StatusCode, Json<AssistantChatErrorResponse>)> {
    let effort: ReasoningEffort = request.effort.parse().map_err(|error: String| {
        (
            StatusCode::BAD_REQUEST,
            Json(AssistantChatErrorResponse { error }),
        )
    })?;

    let mut registry = state.assistant_models.write().await;
    registry.reasoning_effort = effort;
    let response = registry.to_response();
    drop(registry);

    if let Err(error) = crate::storage::settings::set_app_setting(
        &state.pool,
        SETTING_REASONING_EFFORT,
        effort.as_str(),
    )
    .await
    {
        tracing::warn!(error = %error, "failed to persist reasoning effort");
    }

    Ok(Json(response))
}

pub async fn chat(
    State(state): State<AppState>,
    Json(request): Json<AssistantChatRequest>,
) -> impl IntoResponse {
    let permit = match state.assistant_chat_semaphore.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(AssistantChatErrorResponse {
                    error: "too many concurrent assistant requests".to_string(),
                }),
            )
                .into_response();
        }
    };

    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(async move {
        let _permit = permit;
        run_chat_streaming(state, request.messages, tx).await;
    });

    Sse::new(ReceiverStream::new(rx)).into_response()
}

pub async fn get_system_prompt(
    State(state): State<AppState>,
) -> Result<Json<SystemPromptResponse>, (StatusCode, Json<AssistantChatErrorResponse>)> {
    match crate::storage::settings::get_app_setting(&state.pool, SETTING_SYSTEM_PROMPT).await {
        Ok(Some(prompt)) => Ok(Json(SystemPromptResponse {
            prompt,
            is_default: false,
        })),
        Ok(None) => Ok(Json(SystemPromptResponse {
            prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            is_default: true,
        })),
        Err(error) => {
            tracing::warn!(error = %error, "failed to load system prompt");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AssistantChatErrorResponse {
                    error: "failed to load system prompt".to_string(),
                }),
            ))
        }
    }
}

pub async fn update_system_prompt(
    State(state): State<AppState>,
    Json(request): Json<UpdateSystemPromptRequest>,
) -> Result<Json<SystemPromptResponse>, (StatusCode, Json<AssistantChatErrorResponse>)> {
    let prompt = request.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AssistantChatErrorResponse {
                error: "system prompt cannot be empty".to_string(),
            }),
        ));
    }

    if let Err(error) =
        crate::storage::settings::set_app_setting(&state.pool, SETTING_SYSTEM_PROMPT, &prompt).await
    {
        tracing::warn!(error = %error, "failed to persist system prompt");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AssistantChatErrorResponse {
                error: "failed to save system prompt".to_string(),
            }),
        ));
    }

    Ok(Json(SystemPromptResponse {
        prompt,
        is_default: false,
    }))
}

pub async fn delete_system_prompt(
    State(state): State<AppState>,
) -> Result<Json<SystemPromptResponse>, (StatusCode, Json<AssistantChatErrorResponse>)> {
    if let Err(error) =
        crate::storage::settings::delete_app_setting(&state.pool, SETTING_SYSTEM_PROMPT).await
    {
        tracing::warn!(error = %error, "failed to delete system prompt");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AssistantChatErrorResponse {
                error: "failed to reset system prompt".to_string(),
            }),
        ));
    }

    Ok(Json(SystemPromptResponse {
        prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
        is_default: true,
    }))
}

async fn run_chat_streaming(
    state: AppState,
    messages: Vec<AssistantChatMessageRequest>,
    tx: mpsc::Sender<Result<Event, Infallible>>,
) {
    let registry = state.assistant_models.read().await;
    let selected_model = registry.selected_model.clone();
    let reasoning_effort = registry.reasoning_effort;
    drop(registry);
    let openai_api_key = state
        .openai_api_key
        .as_deref()
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string);

    match (openai_api_key.as_deref(), selected_model.as_str()) {
        (Some(api_key), model) if model != MOCK_BACKEND_MODEL => {
            info!(
                message_count = messages.len(),
                model, "dispatching to OpenAI"
            );
            openai_responses_streaming(
                &state,
                &messages,
                api_key,
                model,
                reasoning_effort.as_str(),
                &tx,
            )
            .await;
        }
        _ => {
            if openai_api_key.is_some() {
                info!(
                    message_count = messages.len(),
                    model = %selected_model,
                    "OPENAI_API_KEY is configured but the assistant is using the in-memory mock model"
                );
            } else {
                info!("OPENAI_API_KEY not set — using mock reply");
            }
            let prompt = latest_user_prompt_from(&messages).unwrap_or_default();
            match build_mock_reply(&state, prompt).await {
                Ok(text) => {
                    send_sse_event(
                        &tx,
                        json!({"type": "text", "text": text, "model": selected_model}),
                    )
                    .await;
                }
                Err(e) => {
                    error!(error = %e, "mock reply failed");
                    send_sse_event(&tx, json!({"type": "error", "error": e.to_string()})).await;
                }
            }
        }
    }
}
