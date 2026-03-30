use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::graphql::AppState;
use crate::storage::chat_threads::{
    append_chat_message, create_chat_thread, delete_chat_thread, get_chat_thread,
    list_chat_messages, list_chat_threads, rename_chat_thread, update_chat_thread_status,
};

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ThreadMetadataResponse {
    pub id: String,
    pub title: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub parent_id: Option<String>,
    pub content: Value,
    pub run_config: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct MessagesResponse {
    pub messages: Vec<MessageResponse>,
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameTitleRequest {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct AppendMessageRequest {
    pub id: String,
    pub parent_id: Option<String>,
    pub content: Value,
    pub run_config: Option<Value>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_threads(State(state): State<AppState>) -> impl IntoResponse {
    match list_chat_threads(&state.pool).await {
        Ok(threads) => {
            let response: Vec<ThreadMetadataResponse> = threads
                .into_iter()
                .map(|t| ThreadMetadataResponse {
                    id: t.id,
                    title: t.title,
                    status: t.status,
                    created_at: t.created_at,
                    updated_at: t.updated_at,
                })
                .collect();
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => {
            tracing::error!("list_threads error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_thread(
    State(state): State<AppState>,
    Json(body): Json<CreateThreadRequest>,
) -> impl IntoResponse {
    match create_chat_thread(&state.pool, &body.id).await {
        Ok(thread) => (
            StatusCode::CREATED,
            Json(ThreadMetadataResponse {
                id: thread.id,
                title: thread.title,
                status: thread.status,
                created_at: thread.created_at,
                updated_at: thread.updated_at,
            }),
        )
            .into_response(),
        Err(err) => {
            tracing::error!("create_thread error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_thread(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
) -> impl IntoResponse {
    match get_chat_thread(&state.pool, &thread_id).await {
        Ok(thread) => Json(ThreadMetadataResponse {
            id: thread.id,
            title: thread.title,
            status: thread.status,
            created_at: thread.created_at,
            updated_at: thread.updated_at,
        })
        .into_response(),
        Err(crate::storage::StorageError::Database(sqlx::Error::RowNotFound)) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(err) => {
            tracing::error!("get_thread error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn rename_thread(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
    Json(body): Json<RenameTitleRequest>,
) -> impl IntoResponse {
    match rename_chat_thread(&state.pool, &thread_id, &body.title).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => {
            tracing::error!("rename_thread error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_thread_status(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
    Json(body): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
    if body.status != "regular" && body.status != "archived" {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match update_chat_thread_status(&state.pool, &thread_id, &body.status).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => {
            tracing::error!("update_thread_status error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_thread(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
) -> impl IntoResponse {
    match delete_chat_thread(&state.pool, &thread_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => {
            tracing::error!("delete_thread error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_thread_messages(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
) -> impl IntoResponse {
    match list_chat_messages(&state.pool, &thread_id).await {
        Ok(messages) => {
            let response = MessagesResponse {
                messages: messages
                    .into_iter()
                    .map(|m| MessageResponse {
                        id: m.id,
                        parent_id: m.parent_id,
                        content: m.content_json,
                        run_config: m.run_config_json,
                    })
                    .collect(),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => {
            tracing::error!("get_thread_messages error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn append_message(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
    Json(body): Json<AppendMessageRequest>,
) -> impl IntoResponse {
    match append_chat_message(
        &state.pool,
        &thread_id,
        &body.id,
        body.parent_id.as_deref(),
        &body.content,
        body.run_config.as_ref(),
    )
    .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => {
            tracing::error!("append_message error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
