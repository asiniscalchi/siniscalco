mod handlers;
mod models;

pub use models::*;
use handlers::*;

use axum::{
    Router,
    http::{Method, header::CONTENT_TYPE},
    routing::{get, put},
};
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};

pub fn build_router(pool: SqlitePool) -> Router {
    let state = AppState { pool };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health))
        .route("/currencies", get(list_currencies_handler))
        .route("/accounts", get(list_accounts_handler).post(create_account_handler))
        .route(
            "/accounts/{account_id}",
            get(get_account_handler).delete(delete_account_handler),
        )
        .route(
            "/accounts/{account_id}/balances/{currency}",
            put(upsert_account_balance_handler).delete(delete_account_balance_handler),
        )
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests;
