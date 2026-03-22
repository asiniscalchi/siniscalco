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
    AccountId, AccountName, AccountType, Amount, CreateAccountInput, Currency, FxRate,
    UpsertAccountBalanceInput, UpsertFxRateInput, create_account, get_account, init_db,
    list_account_balances, upsert_account_balance, upsert_fx_rate,
};

fn amt(value: &str) -> Amount {
    Amount::try_from(value).expect("amount should parse")
}

fn fx_rate(value: &str) -> FxRate {
    FxRate::try_from(value).expect("rate should parse")
}

fn account_id(value: i64) -> AccountId {
    AccountId::try_from(value).expect("account id should parse")
}

fn account_name(value: &str) -> AccountName {
    AccountName::try_from(value).expect("account name should parse")
}

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
async fn lists_fx_rates_for_eur_through_api() {
    let pool = test_pool().await;

    for (from_currency, to_currency, rate) in [
        (Currency::Usd, Currency::Eur, "0.92000000"),
        (Currency::Gbp, Currency::Eur, "1.17000000"),
        (Currency::Chf, Currency::Eur, "1.04000000"),
        (Currency::Eur, Currency::Eur, "1.00000000"),
    ] {
        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency,
                to_currency,
                rate: fx_rate(rate),
            },
        )
        .await
        .expect("fx rate insert should succeed");
    }

    for (from_currency, updated_at) in [
        ("USD", "2026-03-22 09:00:00"),
        ("GBP", "2026-03-22 10:00:00"),
        ("CHF", "2026-03-22 08:30:00"),
        ("EUR", "2026-03-22 11:00:00"),
    ] {
        sqlx::query(
            "UPDATE fx_rates SET updated_at = ? WHERE from_currency = ? AND to_currency = 'EUR'",
        )
        .bind(updated_at)
        .bind(from_currency)
        .execute(&pool)
        .await
        .expect("timestamp update should succeed");
    }

    let app = build_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/fx-rates")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("fx rates request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"target_currency":"EUR","rates":[{"currency":"CHF","rate":"1.04"},{"currency":"GBP","rate":"1.17"},{"currency":"USD","rate":"0.92"}],"last_updated":"2026-03-22 10:00:00"}"#
    );
}

#[tokio::test]
async fn returns_empty_fx_rates_payload_when_no_eur_rates_exist() {
    let pool = test_pool().await;
    let app = build_router(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/fx-rates")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("fx rates request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"target_currency":"EUR","rates":[],"last_updated":null}"#
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("12.3"),
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("1.0"),
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("12.3"),
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
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
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
    assert_eq!(json["summary_status"], "ok");
    assert_eq!(json["total_amount"], "0.00000000");
    assert_eq!(json["total_currency"], "EUR");
    assert!(json["id"].is_i64());
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
async fn returns_standard_error_payload_for_malformed_json() {
    let pool = test_pool().await;
    let app = build_router(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/accounts")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"IBKR","account_type":"broker""#))
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
        r#"{"error":"validation_error","message":"Malformed JSON body"}"#
    );
}

#[tokio::test]
async fn lists_account_summaries_with_totals_through_api() {
    let pool = test_pool().await;
    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id: account_id(1),
            currency: Currency::Eur,
            amount: amt("12000.00000000"),
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
    assert_eq!(json[0]["summary_status"], "ok");
    assert_eq!(json[0]["total_amount"], "12000.00000000");
    assert_eq!(json[0]["total_currency"], "EUR");
    assert!(json[0].get("created_at").is_none());
}

#[tokio::test]
async fn returns_conversion_unavailable_through_api_when_direct_rate_is_missing() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("12.30000000"),
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

    assert_eq!(json[0]["summary_status"], "conversion_unavailable");
    assert!(json[0]["total_amount"].is_null());
    assert!(json[0]["total_currency"].is_null());
}

#[tokio::test]
async fn does_not_use_inverse_fx_rates_through_api() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("12.30000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Eur,
            to_currency: Currency::Usd,
            rate: fx_rate("1.10000000"),
        },
    )
    .await
    .expect("inverse fx rate insert should succeed");

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

    assert_eq!(json[0]["summary_status"], "conversion_unavailable");
}

#[tokio::test]
async fn rounds_converted_account_totals_through_api() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    for currency in [Currency::Usd, Currency::Gbp] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt("1.00000000"),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    for (from_currency, rate) in [(Currency::Usd, "0.33333333"), (Currency::Gbp, "0.33333333")] {
        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency,
                to_currency: Currency::Eur,
                rate: fx_rate(rate),
            },
        )
        .await
        .expect("fx rate insert should succeed");
    }

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

    assert_eq!(json[0]["summary_status"], "ok");
    assert_eq!(json[0]["total_amount"], "0.66666666");
}

#[tokio::test]
async fn gets_account_detail_with_balances_through_api() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("12.3"),
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
