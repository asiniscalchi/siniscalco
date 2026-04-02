mod mutation;
mod query;
mod types;

use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    Router,
    http::{Method, header::CONTENT_TYPE},
    routing::{get, post, put},
};
use sqlx::SqlitePool;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use mutation::MutationRoot;
use query::QueryRoot;

use crate::{
    AssetPriceRefreshConfig, SharedFxRefreshStatus,
    assistant::{SharedAssistantChatSemaphore, SharedAssistantModelRegistry},
    mcp::SharedMcpClient,
};

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn schema_sdl() -> String {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .finish()
        .sdl()
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub fx_refresh_status: SharedFxRefreshStatus,
    pub asset_price_refresh_config: AssetPriceRefreshConfig,
    pub http_client: reqwest::Client,
    pub openai_api_key: Option<String>,
    pub assistant_models: SharedAssistantModelRegistry,
    pub assistant_chat_semaphore: SharedAssistantChatSemaphore,
    pub openai_chat_url: String,
    pub openai_models_url: String,
    pub mcp_client: Option<SharedMcpClient>,
}

pub fn build_schema(
    pool: SqlitePool,
    fx_refresh_status: SharedFxRefreshStatus,
    asset_price_refresh_config: AssetPriceRefreshConfig,
    http_client: reqwest::Client,
) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(pool)
        .data(fx_refresh_status)
        .data(asset_price_refresh_config)
        .data(http_client)
        .finish()
}

pub fn build_router(pool: SqlitePool) -> Router {
    use clap::Parser;
    let config = crate::Config::parse_from(["siniscalco"]);
    build_router_with_state(AppState {
        pool,
        fx_refresh_status: crate::new_shared_fx_refresh_status(),
        asset_price_refresh_config: config.asset_price_refresh_config(),
        http_client: reqwest::Client::new(),
        openai_api_key: config.openai_api_key.clone(),
        assistant_models: crate::assistant::new_shared_assistant_model_registry(
            config.openai_api_key.as_deref(),
            None,
        ),
        assistant_chat_semaphore: crate::assistant::new_assistant_chat_semaphore(),
        openai_chat_url: crate::assistant::openai_chat_url().to_string(),
        openai_models_url: crate::assistant::openai_models_url().to_string(),
        mcp_client: None,
    })
}

pub fn build_router_with_state(state: AppState) -> Router {
    let schema = build_schema(
        state.pool.clone(),
        state.fx_refresh_status.clone(),
        state.asset_price_refresh_config.clone(),
        state.http_client.clone(),
    );

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health))
        .route("/assistant/chat", post(crate::assistant::chat))
        .route("/assistant/models", get(crate::assistant::models))
        .route(
            "/assistant/models/selected",
            put(crate::assistant::select_model),
        )
        .route(
            "/assistant/system-prompt",
            get(crate::assistant::get_system_prompt)
                .put(crate::assistant::update_system_prompt)
                .delete(crate::assistant::delete_system_prompt),
        )
        .route(
            "/assistant/threads",
            get(crate::chat_threads::list_threads).post(crate::chat_threads::create_thread),
        )
        .route(
            "/assistant/threads/{thread_id}",
            get(crate::chat_threads::get_thread).delete(crate::chat_threads::delete_thread),
        )
        .route(
            "/assistant/threads/{thread_id}/title",
            put(crate::chat_threads::rename_thread),
        )
        .route(
            "/assistant/threads/{thread_id}/status",
            put(crate::chat_threads::update_thread_status),
        )
        .route(
            "/assistant/threads/{thread_id}/messages",
            get(crate::chat_threads::get_thread_messages).post(crate::chat_threads::append_message),
        )
        .route_service("/graphql", GraphQL::new(schema))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests;
