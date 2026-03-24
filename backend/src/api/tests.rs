use std::str::FromStr;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::sync::RwLock;
use tower::ServiceExt;

use super::{ApiError, AppState, build_router, build_router_with_state};
use crate::{
    AccountId, AccountName, AccountType, Amount, AssetName, AssetSymbol, AssetTransactionType,
    AssetType, CreateAccountInput, CreateAssetInput, CreateAssetTransactionInput, Currency, FxRate,
    FxRefreshAvailability, FxRefreshStatus, TradeDate, UpsertAccountBalanceInput,
    UpsertFxRateInput, create_account, create_asset, create_asset_transaction, get_account,
    get_asset, init_db, list_account_balances, upsert_account_balance, upsert_fx_rate,
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

fn asset_symbol(value: &str) -> AssetSymbol {
    AssetSymbol::try_from(value).expect("asset symbol should parse")
}

fn asset_name(value: &str) -> AssetName {
    AssetName::try_from(value).expect("asset name should parse")
}

fn trade_date(value: &str) -> TradeDate {
    TradeDate::try_from(value).expect("trade date should parse")
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

fn build_router_with_fx_status(
    pool: sqlx::SqlitePool,
    availability: FxRefreshAvailability,
    last_error: Option<&str>,
) -> axum::Router {
    build_router_with_state(AppState {
        pool,
        fx_refresh_status: std::sync::Arc::new(RwLock::new(FxRefreshStatus {
            availability,
            last_error: last_error.map(str::to_string),
        })),
    })
}

#[tokio::test]
async fn serves_health_route() {
    let pool = test_pool().await;
    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);

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
    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);

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
async fn lists_assets_through_api() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/assets")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("assets request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"[{"id":1,"symbol":"AAPL","name":"Apple Inc.","asset_type":"STOCK","isin":null}]"#
    );
}

#[tokio::test]
async fn creates_assets_through_api_with_normalized_fields() {
    let pool = test_pool().await;
    let app = build_router_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/assets")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"symbol":"  vwce  ","name":"  Vanguard FTSE All-World UCITS ETF  ","asset_type":"ETF","isin":"   "}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("create asset request should succeed");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    let json: Value = serde_json::from_slice(&body).expect("asset body should parse");
    assert_eq!(json["symbol"], "VWCE");
    assert_eq!(json["name"], "Vanguard FTSE All-World UCITS ETF");
    assert_eq!(json["asset_type"], "ETF");
    assert_eq!(json["isin"], Value::Null);
    assert!(json["created_at"].as_str().is_some());
    assert!(json["updated_at"].as_str().is_some());

    let row = sqlx::query("SELECT symbol, name, asset_type, isin FROM assets WHERE id = 1")
        .fetch_one(&pool)
        .await
        .expect("asset row should exist");

    assert_eq!(row.get::<String, _>("symbol"), "VWCE");
    assert_eq!(
        row.get::<String, _>("name"),
        "Vanguard FTSE All-World UCITS ETF"
    );
    assert_eq!(row.get::<String, _>("asset_type"), "ETF");
    assert_eq!(row.get::<Option<String>, _>("isin"), None);
}

#[tokio::test]
async fn rejects_invalid_asset_creation_with_field_errors() {
    let pool = test_pool().await;
    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/assets")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"symbol":"   ","name":"   ","asset_type":"stock","isin":null}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("create asset request should succeed");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"field_errors":{"asset_type":["Asset type must be one of: STOCK, ETF, BOND, CRYPTO, CASH_EQUIVALENT, OTHER"],"name":["Name is required"],"symbol":["Symbol is required"]},"message":"Asset validation failed"}"#
    );
}

#[tokio::test]
async fn rejects_duplicate_asset_symbol_through_api() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            isin: Some("US0378331005".to_string()),
        },
    )
    .await
    .expect("first asset insert should succeed");

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/assets")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"symbol":"AAPL","name":"Apple Common Stock","asset_type":"STOCK","isin":"US0378331006"}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("create asset request should succeed");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"field_errors":{"symbol":["Symbol must be unique"]},"message":"Asset validation failed"}"#
    );
}

#[tokio::test]
async fn rejects_duplicate_asset_isin_through_api() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
            isin: Some("US9229087690".to_string()),
        },
    )
    .await
    .expect("first asset insert should succeed");

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/assets")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"symbol":"VWCE","name":"Vanguard FTSE All-World UCITS ETF","asset_type":"ETF","isin":"US9229087690"}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("create asset request should succeed");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"field_errors":{"isin":["ISIN must be unique"]},"message":"Asset validation failed"}"#
    );
}

#[tokio::test]
async fn lists_fx_rates_for_eur_through_api() {
    let pool = test_pool().await;

    for (from_currency, rate) in [
        (Currency::Usd, "0.920000"),
        (Currency::Gbp, "1.170000"),
        (Currency::Chf, "1.040000"),
    ] {
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

    for (from_currency, updated_at) in [
        ("USD", "2026-03-22 09:00:00"),
        ("GBP", "2026-03-22 10:00:00"),
        ("CHF", "2026-03-22 08:30:00"),
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

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);
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
        r#"{"target_currency":"EUR","rates":[{"currency":"CHF","rate":"1.04"},{"currency":"GBP","rate":"1.17"},{"currency":"USD","rate":"0.92"}],"last_updated":"2026-03-22 10:00:00","refresh_status":"available","refresh_error":null}"#
    );
}

#[tokio::test]
async fn returns_empty_fx_rates_payload_when_no_eur_rates_exist() {
    let pool = test_pool().await;
    let app = build_router_with_fx_status(
        pool,
        FxRefreshAvailability::Unavailable,
        Some("FX refresh unavailable: no successful refresh has completed"),
    );

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
        r#"{"target_currency":"EUR","rates":[],"last_updated":null,"refresh_status":"unavailable","refresh_error":"FX refresh unavailable: no successful refresh has completed"}"#
    );
}

#[tokio::test]
async fn gets_single_fx_rate_pair_through_api() {
    let pool = test_pool().await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.920000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    sqlx::query(
        "UPDATE fx_rates SET updated_at = '2026-03-22 10:00:00' WHERE from_currency = 'USD' AND to_currency = 'EUR'",
    )
    .execute(&pool)
    .await
    .expect("timestamp update should succeed");

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/fx-rates/USD/EUR")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("fx rate request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"from_currency":"USD","to_currency":"EUR","rate":"0.92","updated_at":"2026-03-22 10:00:00","refresh_status":"available","refresh_error":null}"#
    );
}

#[tokio::test]
async fn returns_portfolio_summary_through_api() {
    let pool = test_pool().await;

    let ibkr_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("broker account insert should succeed");

    let bank_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("bank account insert should succeed");

    for (from_currency, rate, updated_at) in [
        (Currency::Usd, "0.920000", "2026-03-22 10:00:00"),
        (Currency::Gbp, "1.170000", "2026-03-22 11:30:00"),
    ] {
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

        sqlx::query(
            "UPDATE fx_rates SET updated_at = ? WHERE from_currency = ? AND to_currency = 'EUR'",
        )
        .bind(updated_at)
        .bind(from_currency.as_str())
        .execute(&pool)
        .await
        .expect("fx timestamp update should succeed");
    }

    for (account_id, currency, amount) in [
        (ibkr_id, Currency::Usd, "100.00"),
        (ibkr_id, Currency::Gbp, "10.00"),
        (bank_id, Currency::Eur, "50.00"),
    ] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt(amount),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/portfolio")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("portfolio request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"display_currency":"EUR","total_value_status":"ok","total_value_amount":"153.700000","account_totals":[{"id":1,"name":"IBKR","account_type":"broker","summary_status":"ok","total_amount":"103.700000","total_currency":"EUR"},{"id":2,"name":"Main Bank","account_type":"bank","summary_status":"ok","total_amount":"50.000000","total_currency":"EUR"}],"cash_by_currency":[{"currency":"EUR","amount":"50.000000","converted_amount":"50.000000"},{"currency":"GBP","amount":"10.000000","converted_amount":"11.700000"},{"currency":"USD","amount":"100.000000","converted_amount":"92.000000"}],"fx_last_updated":"2026-03-22 11:30:00","fx_refresh_status":"available","fx_refresh_error":null}"#
    );
}

#[tokio::test]
async fn creates_asset_transaction_through_api() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let app = build_router_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/transactions")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"account_id":{},"asset_id":{},"transaction_type":"BUY","trade_date":"2026-03-20","quantity":"10","unit_price":"150.25","currency_code":"USD","notes":"initial buy"}}"#,
                    account_id.as_i64(),
                    asset_id.as_i64()
                )))
                .expect("request should build"),
        )
        .await
        .expect("create transaction request should succeed");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).expect("transaction response should parse");

    assert_eq!(json["account_id"], account_id.as_i64());
    assert_eq!(json["asset_id"], asset_id.as_i64());
    assert_eq!(json["transaction_type"], "BUY");
    assert_eq!(json["trade_date"], "2026-03-20");
    assert_eq!(json["quantity"], "10.000000");
    assert_eq!(json["unit_price"], "150.250000");
    assert_eq!(json["currency_code"], "USD");
    assert_eq!(json["notes"], "initial buy");
    assert_eq!(json["created_at"], json["updated_at"]);
}

#[tokio::test]
async fn lists_asset_transactions_through_api_in_trade_date_order() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: crate::AssetQuantity::try_from("2").unwrap(),
            unit_price: crate::AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::Usd,
            notes: Some("older".to_string()),
        },
    )
    .await
    .expect("first transaction insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Sell,
            trade_date: trade_date("2026-03-21"),
            quantity: crate::AssetQuantity::try_from("1").unwrap(),
            unit_price: crate::AssetUnitPrice::try_from("101").unwrap(),
            currency_code: Currency::Usd,
            notes: Some("newer".to_string()),
        },
    )
    .await
    .expect("second transaction insert should succeed");

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/transactions?account_id={}", account_id.as_i64()))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("list transactions request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).expect("transaction list should parse");

    assert_eq!(json[0]["transaction_type"], "SELL");
    assert_eq!(json[0]["trade_date"], "2026-03-21");
    assert_eq!(json[1]["transaction_type"], "BUY");
    assert_eq!(json[1]["trade_date"], "2026-03-20");
}

#[tokio::test]
async fn lists_all_transactions_through_api_without_filter() {
    let pool = test_pool().await;
    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: crate::AssetQuantity::try_from("2").unwrap(),
            unit_price: crate::AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/transactions")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("list transactions request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    let json: Value = serde_json::from_slice(&body).expect("transaction list should parse");
    assert_eq!(
        json.as_array().expect("response should be an array").len(),
        1
    );
}

#[tokio::test]
async fn updates_asset_transaction_through_api() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let transaction = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: crate::AssetQuantity::try_from("10").unwrap(),
            unit_price: crate::AssetUnitPrice::try_from("150.25").unwrap(),
            currency_code: Currency::Usd,
            notes: Some("initial buy".to_string()),
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_router_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/transactions/{}", transaction.id))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"account_id":{},"asset_id":{},"transaction_type":"BUY","trade_date":"2026-03-22","quantity":"7","unit_price":"155","currency_code":"USD","notes":"updated buy"}}"#,
                    account_id.as_i64(),
                    asset_id.as_i64()
                )))
                .expect("request should build"),
        )
        .await
        .expect("update transaction request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).expect("transaction response should parse");

    assert_eq!(json["id"], transaction.id);
    assert_eq!(json["trade_date"], "2026-03-22");
    assert_eq!(json["quantity"], "7.000000");
    assert_eq!(json["unit_price"], "155.000000");
    assert_eq!(json["notes"], "updated buy");
    assert_eq!(json["created_at"], transaction.created_at);
    assert!(json["updated_at"].is_string());
}

#[tokio::test]
async fn rejects_asset_transaction_update_that_would_make_position_negative() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("BTC"),
            name: asset_name("Bitcoin"),
            asset_type: AssetType::Crypto,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: crate::AssetQuantity::try_from("5").unwrap(),
            unit_price: crate::AssetUnitPrice::try_from("80000").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("buy insert should succeed");
    let sell_transaction = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Sell,
            trade_date: trade_date("2026-03-21"),
            quantity: crate::AssetQuantity::try_from("2").unwrap(),
            unit_price: crate::AssetUnitPrice::try_from("81000").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("sell insert should succeed");

    let app = build_router_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/transactions/{}", sell_transaction.id))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"account_id":{},"asset_id":{},"transaction_type":"SELL","trade_date":"2026-03-21","quantity":"6","unit_price":"81000","currency_code":"USD","notes":null}}"#,
                    account_id.as_i64(),
                    asset_id.as_i64()
                )))
                .expect("request should build"),
        )
        .await
        .expect("update transaction request should succeed");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"error":"validation_error","message":"sell transaction would make position negative"}"#
    );
}

#[tokio::test]
async fn deletes_asset_transaction_through_api() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let transaction = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: crate::AssetQuantity::try_from("2").unwrap(),
            unit_price: crate::AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::Usd,
            notes: Some("to be deleted".to_string()),
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_router_with_fx_status(pool.clone(), FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/transactions/{}", transaction.id))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("delete transaction request should succeed");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let list_response = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/transactions?account_id={}", account_id.as_i64()))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("list transactions request should succeed");

    let body = list_response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).expect("transaction list should parse");

    assert!(
        json.as_array()
            .expect("response should be an array")
            .is_empty()
    );
}

#[tokio::test]
async fn returns_not_found_when_deleting_missing_asset_transaction_through_api() {
    let pool = test_pool().await;
    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/transactions/999")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("delete transaction request should succeed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"error":"not_found","message":"Asset transaction not found"}"#
    );
}

#[tokio::test]
async fn lists_active_positions_through_api() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("BTC"),
            name: asset_name("Bitcoin"),
            asset_type: AssetType::Crypto,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    for transaction in [
        (AssetTransactionType::Buy, "3"),
        (AssetTransactionType::Sell, "3"),
        (AssetTransactionType::Buy, "5"),
    ] {
        create_asset_transaction(
            &pool,
            CreateAssetTransactionInput {
                account_id,
                asset_id,
                transaction_type: transaction.0,
                trade_date: trade_date("2026-03-20"),
                quantity: crate::AssetQuantity::try_from(transaction.1).unwrap(),
                unit_price: crate::AssetUnitPrice::try_from("90000").unwrap(),
                currency_code: Currency::Usd,
                notes: None,
            },
        )
        .await
        .expect("transaction insert should succeed");
    }

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/accounts/{}/positions", account_id.as_i64()))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("positions request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        format!(
            r#"[{{"account_id":{},"asset_id":{},"quantity":"5.000000"}}]"#,
            account_id.as_i64(),
            asset_id.as_i64()
        )
    );
}

#[tokio::test]
async fn returns_empty_portfolio_summary_when_no_cash_exists() {
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

    let app = build_router_with_fx_status(
        pool,
        FxRefreshAvailability::Unavailable,
        Some("FX refresh unavailable: no successful refresh has completed"),
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/portfolio")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("portfolio request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"display_currency":"EUR","total_value_status":"ok","total_value_amount":"0.000000","account_totals":[{"id":1,"name":"IBKR","account_type":"broker","summary_status":"ok","total_amount":"0.000000","total_currency":"EUR"}],"cash_by_currency":[],"fx_last_updated":null,"fx_refresh_status":"unavailable","fx_refresh_error":"FX refresh unavailable: no successful refresh has completed"}"#
    );
}

#[tokio::test]
async fn returns_conversion_unavailable_portfolio_summary_when_fx_is_missing() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("100.00"),
        },
    )
    .await
    .expect("usd balance insert should succeed");

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Gbp,
            amount: amt("10.00"),
        },
    )
    .await
    .expect("gbp balance insert should succeed");

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.920000"),
        },
    )
    .await
    .expect("usd fx rate insert should succeed");

    sqlx::query("UPDATE fx_rates SET updated_at = '2026-03-22 10:00:00' WHERE from_currency = 'USD' AND to_currency = 'EUR'")
        .execute(&pool)
        .await
        .expect("fx timestamp update should succeed");

    let app = build_router_with_fx_status(pool, FxRefreshAvailability::Available, None);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/portfolio")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("portfolio request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"display_currency":"EUR","total_value_status":"conversion_unavailable","total_value_amount":null,"account_totals":[{"id":1,"name":"IBKR","account_type":"broker","summary_status":"conversion_unavailable","total_amount":null,"total_currency":"EUR"}],"cash_by_currency":[{"currency":"GBP","amount":"10.000000","converted_amount":null},{"currency":"USD","amount":"100.000000","converted_amount":"92.000000"}],"fx_last_updated":"2026-03-22 10:00:00","fx_refresh_status":"available","fx_refresh_error":null}"#
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

#[tokio::test]
async fn gets_asset_detail_through_api() {
    let pool = test_pool().await;
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            isin: Some("US0378331005".to_string()),
        },
    )
    .await
    .expect("asset insert should succeed");

    let app = build_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/assets/{asset_id}"))
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
        serde_json::from_slice(&body).expect("asset detail body should be valid json");

    assert_eq!(json["symbol"], "AAPL");
    assert_eq!(json["name"], "Apple Inc.");
    assert_eq!(json["asset_type"], "STOCK");
    assert_eq!(json["isin"], "US0378331005");
    assert!(json["created_at"].is_string());
    assert!(json["updated_at"].is_string());
}

#[tokio::test]
async fn updates_asset_through_api() {
    let pool = test_pool().await;
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let app = build_router(pool.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/assets/{asset_id}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"symbol":"msft","name":"Microsoft","asset_type":"STOCK","isin":"US5949181045"}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("update request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let updated = get_asset(&pool, asset_id)
        .await
        .expect("asset should exist");
    assert_eq!(updated.symbol.as_str(), "MSFT");
    assert_eq!(updated.name.as_str(), "Microsoft");
    assert_eq!(updated.isin.as_deref(), Some("US5949181045"));
}

#[tokio::test]
async fn deletes_asset_through_api() {
    let pool = test_pool().await;
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let app = build_router(pool.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/assets/{asset_id}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("delete request should succeed");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let error = get_asset(&pool, asset_id)
        .await
        .expect_err("asset should be deleted");
    match error {
        crate::storage::StorageError::Database(sqlx::Error::RowNotFound) => {}
        other => panic!("expected RowNotFound, got {other}"),
    }
}

#[tokio::test]
async fn rejects_deleting_asset_with_transactions_through_api() {
    let pool = test_pool().await;
    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: crate::AssetQuantity::try_from("1").unwrap(),
            unit_price: crate::AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/assets/{asset_id}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("delete request should succeed");

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(
        std::str::from_utf8(&body).expect("json body should be utf8"),
        r#"{"error":"conflict","message":"Asset has transactions and cannot be deleted"}"#
    );
}

#[tokio::test]
async fn gets_transaction_detail_through_api() {
    let pool = test_pool().await;
    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");
    let transaction = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: crate::AssetQuantity::try_from("1").unwrap(),
            unit_price: crate::AssetUnitPrice::try_from("100").unwrap(),
            currency_code: Currency::Usd,
            notes: Some("first buy".to_string()),
        },
    )
    .await
    .expect("transaction insert should succeed");

    let app = build_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/transactions/{}", transaction.id))
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
        serde_json::from_slice(&body).expect("transaction detail body should be valid json");

    assert_eq!(json["id"], transaction.id);
    assert_eq!(json["notes"], "first buy");
}

#[tokio::test]
async fn lists_account_balances_through_api() {
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

    for (currency, amount) in [(Currency::Usd, "12.3"), (Currency::Eur, "4.5")] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt(amount),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    let app = build_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/accounts/{account_id}/balances"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("balances request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();
    let json: Value =
        serde_json::from_slice(&body).expect("balance list body should be valid json");

    assert_eq!(
        json.as_array().expect("response should be an array").len(),
        2
    );
}

#[tokio::test]
async fn updates_account_through_api() {
    let pool = test_pool().await;
    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Old"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let app = build_router(pool.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/accounts/{account_id}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Updated","account_type":"bank","base_currency":"EUR"}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("update request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let account = get_account(&pool, account_id)
        .await
        .expect("account should exist");
    assert_eq!(account.name.as_str(), "Updated");
    assert_eq!(account.account_type.as_str(), "bank");
    assert_eq!(account.base_currency, Currency::Eur);
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
    assert_eq!(json["amount"], "12.300000");
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
                .body(Body::from(r#"{"amount":"1.1234567"}"#))
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
        r#"{"error":"validation_error","message":"amount must match a signed 6-decimal value"}"#
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

    assert_eq!(json["amount"], "12.000000");
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
    assert_eq!(json["total_amount"], "0.000000");
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
            amount: amt("12000.000000"),
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
    assert_eq!(json[0]["total_amount"], "12000.000000");
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
            amount: amt("12.300000"),
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
            amount: amt("12.300000"),
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
                amount: amt("1.000000"),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    for (from_currency, rate) in [(Currency::Usd, "0.333333"), (Currency::Gbp, "0.333333")] {
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
    assert_eq!(json[0]["total_amount"], "0.666666");
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
                .uri(format!("/accounts/{account_id}"))
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
    assert_eq!(json["balances"][0]["amount"], "12.300000");
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
