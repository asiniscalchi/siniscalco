mod handlers;
mod models;

use handlers::*;
pub use models::*;

use axum::{
    Router,
    http::{Method, header::CONTENT_TYPE},
    routing::{get, put},
};
use sqlx::SqlitePool;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

pub fn build_router(pool: SqlitePool) -> Router {
    let state = AppState {
        pool,
        fx_refresh_status: crate::new_shared_fx_refresh_status(),
    };
    build_router_with_state(state)
}

pub fn build_router_with_state(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health))
        .route(
            "/assets",
            get(list_assets_handler).post(create_asset_handler),
        )
        .route(
            "/asset-transactions",
            get(list_asset_transactions_handler).post(create_asset_transaction_handler),
        )
        .route("/currencies", get(list_currencies_handler))
        .route("/fx-rates", get(get_fx_rate_summary_handler))
        .route(
            "/fx-rates/{from_currency}/{to_currency}",
            get(get_fx_rate_handler),
        )
        .route("/portfolio", get(get_portfolio_summary_handler))
        .route(
            "/accounts",
            get(list_accounts_handler).post(create_account_handler),
        )
        .route(
            "/accounts/{account_id}",
            get(get_account_handler).delete(delete_account_handler),
        )
        .route(
            "/accounts/{account_id}/positions",
            get(list_account_positions_handler),
        )
        .route(
            "/accounts/{account_id}/balances/{currency}",
            put(upsert_account_balance_handler).delete(delete_account_balance_handler),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests;
