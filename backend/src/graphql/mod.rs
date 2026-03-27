mod mutation;
mod query;
mod types;

use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    Router,
    http::{Method, header::CONTENT_TYPE},
    routing::get,
};
use sqlx::SqlitePool;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use mutation::MutationRoot;
use query::QueryRoot;

use crate::{AssetPriceRefreshConfig, SharedFxRefreshStatus};

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub fx_refresh_status: SharedFxRefreshStatus,
    pub asset_price_refresh_config: AssetPriceRefreshConfig,
}

pub fn build_schema(
    pool: SqlitePool,
    fx_refresh_status: SharedFxRefreshStatus,
    asset_price_refresh_config: AssetPriceRefreshConfig,
) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(pool)
        .data(fx_refresh_status)
        .data(asset_price_refresh_config)
        .finish()
}

pub fn build_router(pool: SqlitePool) -> Router {
    use clap::Parser;
    let config = crate::Config::parse_from(["siniscalco"]);
    build_router_with_state(AppState {
        pool,
        fx_refresh_status: crate::new_shared_fx_refresh_status(),
        asset_price_refresh_config: config.asset_price_refresh_config(),
    })
}

pub fn build_router_with_state(state: AppState) -> Router {
    let schema = build_schema(
        state.pool,
        state.fx_refresh_status,
        state.asset_price_refresh_config,
    );

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health))
        .route_service("/graphql", GraphQL::new(schema))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests;
