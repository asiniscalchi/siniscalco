use std::{error::Error, fmt, fs, path::Path, str::FromStr};

use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

const UTC_TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountType {
    Bank,
    Broker,
}

impl AccountType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bank => "bank",
            Self::Broker => "broker",
        }
    }
}

impl TryFrom<&str> for AccountType {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "bank" => Ok(Self::Bank),
            "broker" => Ok(Self::Broker),
            _ => Err(StorageError::Validation(
                "account_type must be one of: bank, broker",
            )),
        }
    }
}

#[derive(Debug)]
pub enum StorageError {
    Validation(&'static str),
    Database(sqlx::Error),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => f.write_str(message),
            Self::Database(error) => write!(f, "{error}"),
        }
    }
}

impl Error for StorageError {}

impl From<sqlx::Error> for StorageError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

pub struct CreateAccountInput<'a> {
    pub name: &'a str,
    pub account_type: AccountType,
    pub base_currency: &'a str,
}

pub struct UpsertAccountBalanceInput<'a> {
    pub account_id: i64,
    pub currency: &'a str,
    pub amount: &'a str,
}

#[derive(Debug, Eq, PartialEq)]
pub enum UpsertOutcome {
    Created,
    Updated,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountRecord {
    pub id: i64,
    pub name: String,
    pub account_type: AccountType,
    pub base_currency: String,
    pub created_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountBalanceRecord {
    pub account_id: i64,
    pub currency: String,
    pub amount: String,
    pub updated_at: String,
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

    fn not_implemented() -> Self {
        Self {
            status: StatusCode::NOT_IMPLEMENTED,
            body: ApiErrorResponse {
                error: "not_implemented",
                message: "Endpoint not implemented yet",
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
        .with_state(AppState { pool })
}

/// SQLite stores DECIMAL values with numeric affinity, so it does not preserve
/// input formatting like trailing zeroes. We keep DECIMAL(20,8) in the schema
/// for clarity and validate writes in the application, but read values back via
/// `CAST(amount AS TEXT)` and treat exact display formatting as an application concern.
pub async fn connect_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    init_db(&pool).await?;
    Ok(pool)
}

pub async fn connect_db_file(path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::Error> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| sqlx::Error::Configuration(Box::new(error)))?;
        }
    }

    let url = format!("sqlite://{}", path.as_ref().display());
    connect_db(&url).await
}

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

pub async fn create_account(
    pool: &SqlitePool,
    input: CreateAccountInput<'_>,
) -> Result<i64, StorageError> {
    validate_name(input.name)?;
    validate_currency(input.base_currency)?;

    let result =
        sqlx::query("INSERT INTO accounts (name, account_type, base_currency) VALUES (?, ?, ?)")
            .bind(input.name)
            .bind(input.account_type.as_str())
            .bind(input.base_currency)
            .execute(pool)
            .await?;

    Ok(result.last_insert_rowid())
}

pub async fn upsert_account_balance(
    pool: &SqlitePool,
    input: UpsertAccountBalanceInput<'_>,
) -> Result<UpsertOutcome, StorageError> {
    validate_currency(input.currency)?;
    validate_decimal_20_8(input.amount)?;

    let updated_at = current_utc_timestamp()?;
    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM account_balances WHERE account_id = ? AND currency = ?)",
    )
    .bind(input.account_id)
    .bind(input.currency)
    .fetch_one(&mut *transaction)
    .await?
        != 0;

    sqlx::query(
        r#"
        INSERT INTO account_balances (account_id, currency, amount, updated_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(account_id, currency) DO UPDATE SET
            amount = excluded.amount,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(input.account_id)
    .bind(input.currency)
    .bind(input.amount)
    .bind(updated_at)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    if existed {
        Ok(UpsertOutcome::Updated)
    } else {
        Ok(UpsertOutcome::Created)
    }
}

pub async fn list_accounts(pool: &SqlitePool) -> Result<Vec<AccountRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, account_type, base_currency, created_at
        FROM accounts
        ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AccountRecord {
                id: row.get("id"),
                name: row.get("name"),
                account_type: AccountType::try_from(row.get::<&str, _>("account_type"))?,
                base_currency: row.get("base_currency"),
                created_at: row.get("created_at"),
            })
        })
        .collect()
}

pub async fn get_account(
    pool: &SqlitePool,
    account_id: i64,
) -> Result<AccountRecord, StorageError> {
    let row = sqlx::query(
        r#"
        SELECT id, name, account_type, base_currency, created_at
        FROM accounts
        WHERE id = ?
        "#,
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;

    Ok(AccountRecord {
        id: row.get("id"),
        name: row.get("name"),
        account_type: AccountType::try_from(row.get::<&str, _>("account_type"))?,
        base_currency: row.get("base_currency"),
        created_at: row.get("created_at"),
    })
}

pub async fn list_account_balances(
    pool: &SqlitePool,
    account_id: i64,
) -> Result<Vec<AccountBalanceRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            account_id,
            currency,
            CAST(amount AS TEXT) AS amount,
            updated_at
        FROM account_balances
        WHERE account_id = ?
        ORDER BY currency
        "#,
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AccountBalanceRecord {
            account_id: row.get("account_id"),
            currency: row.get("currency"),
            amount: row.get("amount"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

pub async fn delete_account_balance(
    pool: &SqlitePool,
    account_id: i64,
    currency: &str,
) -> Result<(), StorageError> {
    validate_currency(currency)?;

    let result = sqlx::query("DELETE FROM account_balances WHERE account_id = ? AND currency = ?")
        .bind(account_id)
        .bind(currency)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}

pub async fn delete_account(pool: &SqlitePool, account_id: i64) -> Result<(), StorageError> {
    let result = sqlx::query("DELETE FROM accounts WHERE id = ?")
        .bind(account_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}

fn validate_name(name: &str) -> Result<(), StorageError> {
    if name.trim().is_empty() {
        return Err(StorageError::Validation("name must not be empty"));
    }

    Ok(())
}

fn validate_currency(currency: &str) -> Result<(), StorageError> {
    let is_valid = currency.len() == 3 && currency.bytes().all(|byte| byte.is_ascii_uppercase());

    if !is_valid {
        return Err(StorageError::Validation(
            "currency must be a 3-letter uppercase code",
        ));
    }

    Ok(())
}

fn validate_decimal_20_8(amount: &str) -> Result<(), StorageError> {
    let amount = amount.strip_prefix('-').unwrap_or(amount);

    if amount.is_empty() {
        return Err(StorageError::Validation("amount must not be empty"));
    }

    let (integer_part, fractional_part) = match amount.split_once('.') {
        Some((integer_part, fractional_part)) => (integer_part, Some(fractional_part)),
        None => (amount, None),
    };

    if integer_part.is_empty() || !integer_part.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
    }

    if let Some(fractional_part) = fractional_part {
        if fractional_part.is_empty()
            || fractional_part.len() > 8
            || !fractional_part.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
        }
    }

    let total_digits = integer_part.len() + fractional_part.map_or(0, str::len);
    if total_digits > 20 || integer_part.len() > 12 {
        return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
    }

    Ok(())
}

fn current_utc_timestamp() -> Result<String, StorageError> {
    OffsetDateTime::now_utc()
        .format(UTC_TIMESTAMP_FORMAT)
        .map_err(|_| StorageError::Validation("failed to generate UTC timestamp"))
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

    let account_id = create_account(
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
        .ok_or_else(|| ApiError::internal_server_error())?;

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
    AxumPath((_account_id,)): AxumPath<(i64,)>,
) -> Result<StatusCode, ApiError> {
    Err(ApiError::not_implemented())
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

fn normalize_amount_output(amount: &str) -> String {
    let (sign, unsigned) = match amount.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", amount),
    };

    let (integer_part, fractional_part) = match unsigned.split_once('.') {
        Some((integer_part, fractional_part)) => (integer_part, fractional_part),
        None => (unsigned, ""),
    };

    let mut normalized = String::with_capacity(sign.len() + integer_part.len() + 9);
    normalized.push_str(sign);
    normalized.push_str(integer_part);
    normalized.push('.');
    normalized.push_str(fractional_part);

    for _ in fractional_part.len()..8 {
        normalized.push('0');
    }

    normalized
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
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::NamedTempFile;
    use tower::ServiceExt;

    use super::{
        AccountBalanceRecord, AccountRecord, AccountType, ApiError, CreateAccountInput,
        StorageError, UpsertAccountBalanceInput, UpsertOutcome, build_router, connect_db_file,
        create_account, delete_account, delete_account_balance, get_account, init_db,
        list_account_balances, list_accounts, upsert_account_balance,
    };

    async fn test_pool() -> sqlx::SqlitePool {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")
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
    async fn applies_migrations_and_creates_tables() {
        let pool = test_pool().await;

        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'account_balances', '_sqlx_migrations')",
        )
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed");

        assert_eq!(tables, 3);
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
    async fn serves_account_route_skeletons() {
        let pool = test_pool().await;
        let app = build_router(pool);

        for (method, uri) in [("DELETE", "/accounts/1")] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
                        .expect("request should build"),
                )
                .await
                .expect("route request should succeed");

            assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        }
    }

    #[tokio::test]
    async fn returns_standard_json_error_shape_for_placeholder_routes() {
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

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should collect")
            .to_bytes();

        assert_eq!(
            std::str::from_utf8(&body).expect("json body should be utf8"),
            r#"{"error":"not_implemented","message":"Endpoint not implemented yet"}"#
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

    #[test]
    fn maps_validation_storage_errors_to_bad_request() {
        let error = ApiError::from(StorageError::Validation("Invalid currency format"));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn maps_row_not_found_to_not_found() {
        let error = ApiError::from(StorageError::Database(sqlx::Error::RowNotFound));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn maps_database_errors_to_internal_server_error() {
        let error = ApiError::from(StorageError::Database(sqlx::Error::PoolTimedOut));
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
    async fn bootstraps_file_database_and_runs_migrations() {
        let file = NamedTempFile::new().expect("temp db file should be created");
        let pool = connect_db_file(file.path())
            .await
            .expect("file database should initialize");

        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'account_balances')",
        )
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed");

        assert_eq!(tables, 2);
    }

    #[tokio::test]
    async fn creates_account_without_balance() {
        let pool = test_pool().await;

        create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("account insert should succeed");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&pool)
            .await
            .expect("account count query should succeed");

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn reads_accounts_in_insert_order() {
        let pool = test_pool().await;

        create_account(
            &pool,
            CreateAccountInput {
                name: "Main Bank",
                account_type: AccountType::Bank,
                base_currency: "USD",
            },
        )
        .await
        .expect("first account insert should succeed");

        create_account(
            &pool,
            CreateAccountInput {
                name: "IBKR",
                account_type: AccountType::Broker,
                base_currency: "EUR",
            },
        )
        .await
        .expect("second account insert should succeed");

        let accounts = list_accounts(&pool)
            .await
            .expect("account list should succeed");

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].name, "Main Bank");
        assert_eq!(accounts[0].account_type, AccountType::Bank);
        assert_eq!(accounts[1].name, "IBKR");
        assert_eq!(accounts[1].account_type, AccountType::Broker);
    }

    #[tokio::test]
    async fn gets_single_account_by_id() {
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

        let account = get_account(&pool, account_id)
            .await
            .expect("single account fetch should succeed");

        assert_eq!(account.id, account_id);
        assert_eq!(account.name, "IBKR");
        assert_eq!(account.account_type, AccountType::Broker);
        assert_eq!(account.base_currency, "EUR");
    }

    #[tokio::test]
    async fn allows_multiple_currencies_per_account() {
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

        for (currency, amount) in [("EUR", "12000.00000000"), ("USD", "3500.00000000")] {
            upsert_account_balance(
                &pool,
                UpsertAccountBalanceInput {
                    account_id,
                    currency,
                    amount,
                },
            )
            .await
            .expect("balance insert should succeed");
        }

        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should succeed");

        assert_eq!(
            balances,
            vec![
                AccountBalanceRecord {
                    account_id,
                    currency: "EUR".to_string(),
                    amount: "12000".to_string(),
                    updated_at: balances[0].updated_at.clone(),
                },
                AccountBalanceRecord {
                    account_id,
                    currency: "USD".to_string(),
                    amount: "3500".to_string(),
                    updated_at: balances[1].updated_at.clone(),
                }
            ]
        );
        assert_eq!(balances[0].updated_at.len(), 19);
        assert_eq!(balances[1].updated_at.len(), 19);
    }

    #[tokio::test]
    async fn upsert_updates_existing_balance() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "Main Bank",
                account_type: AccountType::Bank,
                base_currency: "USD",
            },
        )
        .await
        .expect("account insert should succeed");

        let first_outcome = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "10.00000000",
            },
        )
        .await
        .expect("first balance insert should succeed");
        assert_eq!(first_outcome, UpsertOutcome::Created);

        let second_outcome = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "USD",
                amount: "12.00000000",
            },
        )
        .await
        .expect("upsert should update the existing balance");
        assert_eq!(second_outcome, UpsertOutcome::Updated);

        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should succeed");

        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].amount, "12");
        assert_eq!(balances[0].updated_at.len(), 19);
    }

    #[tokio::test]
    async fn deletes_single_balance() {
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

        delete_account_balance(&pool, account_id, "EUR")
            .await
            .expect("balance delete should succeed");

        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should succeed");

        assert!(balances.is_empty());
    }

    #[tokio::test]
    async fn deleting_missing_balance_returns_not_found() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "Main Bank",
                account_type: AccountType::Bank,
                base_currency: "USD",
            },
        )
        .await
        .expect("account insert should succeed");

        let error = delete_account_balance(&pool, account_id, "USD")
            .await
            .expect_err("missing balance delete should fail");

        match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {}
            other => panic!("expected RowNotFound, got {other}"),
        }
    }

    #[tokio::test]
    async fn deletes_account_and_cascades_balances() {
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

        delete_account(&pool, account_id)
            .await
            .expect("account delete should succeed");

        let account_error = get_account(&pool, account_id)
            .await
            .expect_err("deleted account should not exist");
        let balances = list_account_balances(&pool, account_id)
            .await
            .expect("balance list should still succeed");

        match account_error {
            StorageError::Database(sqlx::Error::RowNotFound) => {}
            other => panic!("expected RowNotFound, got {other}"),
        }
        assert!(balances.is_empty());
    }

    #[tokio::test]
    async fn deleting_missing_account_returns_not_found() {
        let pool = test_pool().await;

        let error = delete_account(&pool, 999)
            .await
            .expect_err("missing account delete should fail");

        match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {}
            other => panic!("expected RowNotFound, got {other}"),
        }
    }

    #[tokio::test]
    async fn rejects_invalid_account_type_input() {
        let error =
            AccountType::try_from("cash").expect_err("unsupported account type should fail");

        assert_eq!(
            error.to_string(),
            "account_type must be one of: bank, broker"
        );
    }

    #[tokio::test]
    async fn rejects_invalid_account_currency_input() {
        let pool = test_pool().await;

        let error = create_account(
            &pool,
            CreateAccountInput {
                name: "Main Bank",
                account_type: AccountType::Bank,
                base_currency: "usd",
            },
        )
        .await
        .expect_err("lowercase currency should fail");

        assert_eq!(
            error.to_string(),
            "currency must be a 3-letter uppercase code"
        );
    }

    #[tokio::test]
    async fn rejects_invalid_balance_currency_input() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "Main Bank",
                account_type: AccountType::Bank,
                base_currency: "USD",
            },
        )
        .await
        .expect("account insert should succeed");

        let error = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "us",
                amount: "10.00000000",
            },
        )
        .await
        .expect_err("invalid currency should fail");

        assert_eq!(
            error.to_string(),
            "currency must be a 3-letter uppercase code"
        );
    }

    #[tokio::test]
    async fn rejects_invalid_amount_input() {
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

        let error = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: "EUR",
                amount: "1.123456789",
            },
        )
        .await
        .expect_err("amount with more than 8 decimals should fail");

        assert_eq!(error.to_string(), "amount must match DECIMAL(20,8)");
    }

    #[tokio::test]
    async fn rejects_balance_for_missing_account() {
        let pool = test_pool().await;

        let error = upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id: 999_i64,
                currency: "USD",
                amount: "10.00000000",
            },
        )
        .await
        .expect_err("missing parent account should fail");

        match error {
            StorageError::Database(error) => {
                assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
            }
            StorageError::Validation(_) => panic!("expected database error"),
        }
    }

    #[tokio::test]
    async fn preserves_created_account_fields() {
        let pool = test_pool().await;

        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: "Joint Bank",
                account_type: AccountType::Bank,
                base_currency: "GBP",
            },
        )
        .await
        .expect("account insert should succeed");

        let accounts = list_accounts(&pool)
            .await
            .expect("account list should succeed");

        assert_eq!(
            accounts,
            vec![AccountRecord {
                id: account_id,
                name: "Joint Bank".to_string(),
                account_type: AccountType::Bank,
                base_currency: "GBP".to_string(),
                created_at: accounts[0].created_at.clone(),
            }]
        );
        assert_eq!(accounts[0].created_at.len(), 19);
    }

    #[tokio::test]
    async fn migration_metadata_contains_the_initial_migration() {
        let pool = test_pool().await;

        let version: i64 = sqlx::query_scalar("SELECT version FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("migration metadata query should succeed");

        assert_eq!(version, 1);
    }
}
