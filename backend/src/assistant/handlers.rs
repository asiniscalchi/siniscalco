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
use super::model_registry::{MOCK_BACKEND_MODEL, SETTING_SELECTED_MODEL};
use super::openai_client::{openai_chat_streaming, send_sse_event};
use super::types::{
    AssistantChatErrorResponse, AssistantChatMessageRequest, AssistantChatRequest,
    AssistantModelSelectionRequest, AssistantModelsResponse,
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

async fn run_chat_streaming(
    state: AppState,
    messages: Vec<AssistantChatMessageRequest>,
    tx: mpsc::Sender<Result<Event, Infallible>>,
) {
    let selected_model = state.assistant_models.read().await.selected_model.clone();
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
            openai_chat_streaming(&state, &messages, api_key, model, &selected_model, &tx).await;
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
