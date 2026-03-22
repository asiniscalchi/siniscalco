use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::{Method, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    AccountBalanceRecord, AccountRecord, AccountType, CreateAccountInput, CurrencyRecord,
    UpsertAccountBalanceInput, UpsertOutcome, delete_account, delete_account_balance, get_account,
    list_account_balances, list_accounts, list_currencies, normalize_amount_output,
    storage::StorageError, upsert_account_balance,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct CreateAccountRequest {
    pub name: String,
    pub account_type: String,
    pub base_currency: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpsertBalanceRequest {
    pub amount: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AccountSummaryResponse {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub base_currency: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct BalanceResponse {
    pub currency: String,
    pub amount: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct CurrencyResponse {
    pub code: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AccountDetailResponse {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub base_currency: String,
    pub created_at: String,
    pub balances: Vec<BalanceResponse>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct ApiErrorResponse {
    pub error: &'static str,
    pub message: &'static str,
}

pub struct ApiError {
    status: StatusCode,
    body: ApiErrorResponse,
}

impl ApiError {
    fn validation(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ApiErrorResponse {
                error: "validation_error",
                message,
            },
        }
    }

    fn not_found(message: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            body: ApiErrorResponse {
                error: "not_found",
                message,
            },
        }
    }

    fn internal_server_error() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: ApiErrorResponse {
                error: "internal_error",
                message: "Internal server error",
            },
        }
    }
}

impl From<StorageError> for ApiError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::Validation(message) => Self::validation(message),
            StorageError::Database(sqlx::Error::RowNotFound) => {
                Self::not_found("Resource not found")
            }
            StorageError::Database(_) => Self::internal_server_error(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

pub fn build_router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/currencies", get(list_currencies_handler))
        .route(
            "/accounts",
            post(create_account_handler).get(list_accounts_handler),
        )
        .route(
            "/accounts/{account_id}",
            get(get_account_handler).delete(delete_account_handler),
        )
        .route(
            "/accounts/{account_id}/balances/{currency}",
            put(upsert_account_balance_handler).delete(delete_account_balance_handler),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([CONTENT_TYPE]),
        )
        .with_state(AppState { pool })
}

async fn health() -> &'static str {
    "ok"
}

async fn create_account_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<AccountSummaryResponse>), ApiError> {
    let account_type =
        AccountType::try_from(request.account_type.as_str()).map_err(ApiError::from)?;

    let account_id = crate::create_account(
        &state.pool,
        CreateAccountInput {
            name: &request.name,
            account_type,
            base_currency: &request.base_currency,
        },
    )
    .await
    .map_err(ApiError::from)?;

    let account = get_account(&state.pool, account_id)
        .await
        .map_err(ApiError::from)?;

    Ok((
        StatusCode::CREATED,
        Json(to_account_summary_response(account)),
    ))
}

async fn list_currencies_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<CurrencyResponse>>, ApiError> {
    let currencies = list_currencies(&state.pool).await.map_err(ApiError::from)?;

    Ok(Json(
        currencies.into_iter().map(to_currency_response).collect(),
    ))
}

async fn list_accounts_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<AccountSummaryResponse>>, ApiError> {
    let accounts = list_accounts(&state.pool).await.map_err(ApiError::from)?;

    Ok(Json(
        accounts
            .into_iter()
            .map(to_account_summary_response)
            .collect(),
    ))
}

async fn get_account_handler(
    State(state): State<AppState>,
    AxumPath((account_id,)): AxumPath<(i64,)>,
) -> Result<Json<AccountDetailResponse>, ApiError> {
    let account = get_account(&state.pool, account_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Account not found")
            }
            other => ApiError::from(other),
        })?;
    let balances = list_account_balances(&state.pool, account_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(to_account_detail_response(account, balances)))
}

async fn upsert_account_balance_handler(
    State(state): State<AppState>,
    AxumPath((account_id, currency)): AxumPath<(i64, String)>,
    Json(request): Json<UpsertBalanceRequest>,
) -> Result<(StatusCode, Json<BalanceResponse>), ApiError> {
    get_account(&state.pool, account_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Account not found")
            }
            other => ApiError::from(other),
        })?;

    let outcome = upsert_account_balance(
        &state.pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: &currency,
            amount: &request.amount,
        },
    )
    .await
    .map_err(ApiError::from)?;

    let balance = list_account_balances(&state.pool, account_id)
        .await
        .map_err(ApiError::from)?
        .into_iter()
        .find(|balance| balance.currency == currency)
        .ok_or_else(ApiError::internal_server_error)?;

    let status = match outcome {
        UpsertOutcome::Created => StatusCode::CREATED,
        UpsertOutcome::Updated => StatusCode::OK,
    };

    Ok((status, Json(to_balance_response(balance))))
}

async fn delete_account_balance_handler(
    State(state): State<AppState>,
    AxumPath((account_id, currency)): AxumPath<(i64, String)>,
) -> Result<StatusCode, ApiError> {
    get_account(&state.pool, account_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Account not found")
            }
            other => ApiError::from(other),
        })?;

    delete_account_balance(&state.pool, account_id, &currency)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Balance not found")
            }
            other => ApiError::from(other),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_account_handler(
    State(state): State<AppState>,
    AxumPath((account_id,)): AxumPath<(i64,)>,
) -> Result<StatusCode, ApiError> {
    delete_account(&state.pool, account_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Account not found")
            }
            other => ApiError::from(other),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

fn to_account_summary_response(account: AccountRecord) -> AccountSummaryResponse {
    AccountSummaryResponse {
        id: account.id,
        name: account.name,
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        created_at: account.created_at,
    }
}

fn to_account_detail_response(
    account: AccountRecord,
    balances: Vec<AccountBalanceRecord>,
) -> AccountDetailResponse {
    AccountDetailResponse {
        id: account.id,
        name: account.name,
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        created_at: account.created_at,
        balances: balances.into_iter().map(to_balance_response).collect(),
    }
}

fn to_balance_response(balance: AccountBalanceRecord) -> BalanceResponse {
    BalanceResponse {
        currency: balance.currency,
        amount: normalize_amount_output(&balance.amount),
        updated_at: balance.updated_at,
    }
}

fn to_currency_response(currency: CurrencyRecord) -> CurrencyResponse {
    CurrencyResponse {
        code: currency.code,
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::IntoResponse,
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tower::ServiceExt;

    use super::{ApiError, build_router};
    use crate::{
        AccountType, CreateAccountInput, UpsertAccountBalanceInput, create_account, get_account,
        init_db, list_account_balances, upsert_account_balance,
    };

    async fn test_pool() -> sqlx::SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .expect("in-memory sqlite connect options should parse")
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("in-memory sqlite pool should connect");

        init_db(&pool).await.expect("schema should initialize");
        pool
    }

    #[tokio::test]
    async fn serves_health_route() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("health request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn lists_allowed_currencies_through_api() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/currencies")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("currencies request should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"[{"code":"CHF"},{"code":"EUR"},{"code":"GBP"},{"code":"USD"}]"#
        );
    }

    #[tokio::test]
    async fn serves_account_route_skeletons() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/accounts/1")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("route request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn maps_validation_storage_errors_to_bad_request() {
        let error = ApiError::from(crate::storage::StorageError::Validation(
            "Invalid currency format",
        ));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn maps_row_not_found_to_not_found() {
        let error = ApiError::from(crate::storage::StorageError::Database(
            sqlx::Error::RowNotFound,
        ));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn maps_database_errors_to_internal_server_error() {
        let error = ApiError::from(crate::storage::StorageError::Database(
            sqlx::Error::PoolTimedOut,
        ));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn builds_standard_validation_error_payload() {
        let error = ApiError::validation("Invalid amount format");
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"validation_error","message":"Invalid amount format"}"#
        );
    }

    #[tokio::test]
    async fn deletes_account_through_api() {
        let pool = test_pool().await;
        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "12.3",
            },
        )
        .await
        .expect("balance insert should succeed");

        let app = build_router(pool.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/accounts/{account_id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("delete request should succeed");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let account_error = get_account(&pool, account_id)
            .await
            .expect_err("account should be deleted");
        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance lookup should succeed");

        match account_error {
            crate::storage::StorageError::Database(sqlx::Error::RowNotFound) => {}
            other => panic!("expected RowNotFound, got {other}"),
        }
        assert!(balances.is_empty());
    }

    #[tokio::test]
    async fn returns_not_found_when_deleting_missing_account_through_api() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/accounts/999")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("delete request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"not_found","message":"Account not found"}"#
        );
    }

    #[tokio::test]
    async fn creates_balance_through_api() {
        let pool = test_pool().await;
        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        let app = build_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/accounts/{account_id}/balances/USD"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":"12.3"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("upsert request should succeed");

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let json: Value = serde_json::from_slice(&body).expect("balance body should be valid json");

        assert_eq!(json["currency"], "USD");
        assert_eq!(json["amount"], "12.30000000");
        assert!(json["updated_at"].is_string());
    }

    #[tokio::test]
    async fn rejects_invalid_amount_through_balance_api() {
        let pool = test_pool().await;
        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        let app = build_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/accounts/{account_id}/balances/USD"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":"1.123456789"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("upsert request should succeed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"validation_error","message":"amount must match DECIMAL(20,8)"}"#
        );
    }

    #[tokio::test]
    async fn rejects_invalid_currency_through_balance_api() {
        let pool = test_pool().await;
        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        let app = build_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/accounts/{account_id}/balances/us"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":"12"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("upsert request should succeed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"validation_error","message":"currency must be one of: EUR, USD, GBP, CHF"}"#
        );
    }

    #[tokio::test]
    async fn updates_balance_through_api() {
        let pool = test_pool().await;
        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "1.0",
            },
        )
        .await
        .expect("initial balance insert should succeed");

        let app = build_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/accounts/{account_id}/balances/USD"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":"12"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("upsert request should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let json: Value = serde_json::from_slice(&body).expect("balance body should be valid json");

        assert_eq!(json["amount"], "12.00000000");
    }

    #[tokio::test]
    async fn returns_not_found_when_writing_balance_for_missing_account() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/accounts/999/balances/USD")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"amount":"12"}"#))
                    .expect("request should build"),
            )
            .await
            .expect("upsert request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"not_found","message":"Account not found"}"#
        );
    }

    #[tokio::test]
    async fn deletes_balance_through_api() {
        let pool = test_pool().await;
        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "12.3",
            },
        )
        .await
        .expect("balance insert should succeed");

        let app = build_router(pool.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/accounts/{account_id}/balances/USD"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("delete request should succeed");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should succeed");
        assert!(balances.is_empty());
    }

    #[tokio::test]
    async fn returns_not_found_when_deleting_missing_balance() {
        let pool = test_pool().await;
        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        let app = build_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/accounts/{account_id}/balances/USD"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("delete request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"not_found","message":"Balance not found"}"#
        );
    }

    #[tokio::test]
    async fn returns_not_found_when_deleting_balance_for_missing_account() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/accounts/999/balances/USD")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("delete request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"not_found","message":"Account not found"}"#
        );
    }

    #[tokio::test]
    async fn creates_account_through_api() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/accounts")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"IBKR","account_type":"broker","base_currency":"EUR"}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("create request should succeed");

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let json: Value =
            serde_json::from_slice(&body).expect("account create body should be valid json");

        assert_eq!(json["name"], "IBKR");
        assert_eq!(json["account_type"], "broker");
        assert_eq!(json["base_currency"], "EUR");
        assert!(json["id"].is_i64());
        assert!(json["created_at"].is_string());
    }

    #[tokio::test]
    async fn rejects_invalid_account_type_through_api() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/accounts")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"IBKR","account_type":"cash","base_currency":"EUR"}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("create request should succeed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"validation_error","message":"account_type must be one of: bank, broker"}"#
        );
    }

    #[tokio::test]
    async fn rejects_invalid_base_currency_through_api() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/accounts")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"IBKR","account_type":"broker","base_currency":"eur"}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("create request should succeed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"validation_error","message":"currency must be one of: EUR, USD, GBP, CHF"}"#
        );
    }

    #[tokio::test]
    async fn lists_account_summaries_without_balances_through_api() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "EUR",
                amount: "12000.00000000",
            },
        )
        .await
        .expect("balance insert should succeed");

        let app = build_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/accounts")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("list request should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let json: Value =
            serde_json::from_slice(&body).expect("account list body should be valid json");

        assert_eq!(
            json.as_array()
                .expect("list response should be an array")
                .len(),
            1
        );
        assert!(json[0].get("balances").is_none());
        assert_eq!(json[0]["name"], "IBKR");
    }

    #[tokio::test]
    async fn gets_account_detail_with_balances_through_api() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "12.3",
            },
        )
        .await
        .expect("balance insert should succeed");

        let app = build_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/accounts/{account_id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("detail request should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();
        let json: Value =
            serde_json::from_slice(&body).expect("account detail body should be valid json");

        assert_eq!(json["name"], "IBKR");
        assert_eq!(json["balances"][0]["currency"], "USD");
        assert_eq!(json["balances"][0]["amount"], "12.30000000");
    }

    #[tokio::test]
    async fn returns_not_found_for_missing_account_detail() {
        let pool = test_pool().await;
        let app = build_router(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/accounts/999")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("detail request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"not_found","message":"Account not found"}"#
        );
    }
}
