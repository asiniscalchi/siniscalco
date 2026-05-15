mod mutation;
mod query;
mod types;

use std::path::PathBuf;

use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{any, get},
};
use serde::Serialize;
use sqlx::SqlitePool;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::{info, warn};

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
    pub web_dir: Option<PathBuf>,
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
        web_dir: None,
    })
}

pub fn build_router_with_state(state: AppState) -> Router {
    let schema = build_schema(
        state.pool.clone(),
        state.fx_refresh_status.clone(),
        state.asset_price_refresh_config.clone(),
        state.http_client.clone(),
    );

    let api_router = Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/config", get(config_summary))
        .route_service("/graphql", GraphQL::new(schema))
        .with_state(state.clone());

    let mut router: Router<AppState> = Router::new().nest("/api", api_router).nest_service(
        "/mcp",
        crate::mcp_server::build_mcp_service(state.pool.clone()),
    );

    if let Some(web_dir) = state.web_dir.as_deref() {
        if web_dir.join("index.html").is_file() {
            info!(web_dir = %web_dir.display(), "serving frontend from disk");
        } else {
            warn!(
                web_dir = %web_dir.display(),
                "WEB_DIR does not contain index.html; SPA routes will 404"
            );
        }
        let spa_index = any(spa_fallback).with_state(state.clone());
        let serve_dir = ServeDir::new(web_dir).fallback(spa_index);
        router = router.fallback_service(serve_dir);
    } else {
        router = router.fallback(spa_fallback);
    }

    router.layer(TraceLayer::new_for_http()).with_state(state)
}

async fn spa_fallback(State(state): State<AppState>) -> impl IntoResponse {
    let Some(dir) = state.web_dir.as_deref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let index = dir.join("index.html");
    match tokio::fs::read(&index).await {
        Ok(bytes) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
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
