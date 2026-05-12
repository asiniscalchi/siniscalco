mod mutation;
mod query;
mod types;

use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    Json, Router,
    http::{Method, header::CONTENT_TYPE},
    routing::get,
};
use serde::Serialize;
use sqlx::SqlitePool;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use mutation::MutationRoot;
use query::QueryRoot;

use crate::{AssetPriceRefreshConfig, SharedFxRefreshStatus};

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
    pub config_markdown: String,
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
        config_markdown: config.to_markdown(),
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
        .route("/version", get(version))
        .route("/config", get(config_summary))
        .route_service("/graphql", GraphQL::new(schema))
        .nest_service(
            "/mcp",
            crate::mcp_server::build_mcp_service(state.pool.clone()),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

#[derive(Serialize)]
struct ConfigResponse {
    markdown: String,
}

async fn config_summary(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        markdown: state.config_markdown.clone(),
    })
}

#[derive(Serialize)]
struct VersionResponse {
    version: &'static str,
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: option_env!("GIT_VERSION").unwrap_or("unknown"),
    })
}

#[cfg(test)]
mod tests;
