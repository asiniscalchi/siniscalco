mod accounts;
mod assets;
mod common;
mod currencies;
mod fx;
mod health;
mod models;
mod portfolio;
mod transactions;

use accounts::*;
use assets::*;
use currencies::*;
use fx::*;
use health::*;
pub use models::*;
use portfolio::*;
use transactions::*;

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
        asset_price_refresh_config: crate::AssetPriceRefreshConfig::load(),
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
            "/assets/{asset_id}",
            get(get_asset_handler)
                .put(update_asset_handler)
                .delete(delete_asset_handler),
        )
        .route(
            "/transactions",
            get(list_transactions_handler).post(create_transaction_handler),
        )
        .route(
            "/transactions/{transaction_id}",
            get(get_transaction_handler)
                .put(update_transaction_handler)
                .delete(delete_transaction_handler),
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
            get(get_account_handler)
                .put(update_account_handler)
                .delete(delete_account_handler),
        )
        .route(
            "/accounts/{account_id}/balances",
            get(list_account_balances_handler),
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
