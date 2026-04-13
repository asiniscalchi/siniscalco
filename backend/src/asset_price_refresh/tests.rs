use std::{str::FromStr, time::Duration};

use axum::{Json, Router, routing::any};
use clap::Parser;
use reqwest::Client;
use serde_json::json;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::net::TcpListener;

use super::{
    AssetPriceRefreshConfig,
    providers::{
        fetch_alpha_vantage_quote, fetch_coingecko_quote, fetch_eodhd_quote, fetch_finnhub_quote,
        fetch_fmp_quote, fetch_marketstack_quote, fetch_openfigi_tickers, fetch_polygon_quote,
        fetch_tiingo_quote, fetch_twelve_data_quote, yahoo::fetch_yahoo_quote,
    },
    refresh_asset_prices,
};
use crate::{
    AssetType, Config, CreateAssetInput, Currency, UpsertAssetQuoteSourceInput, get_asset, init_db,
    upsert_asset_quote_source,
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

    init_db(&pool).await.expect("database should initialize");
    pool
}

async fn start_test_server_at(route: &'static str, payload: serde_json::Value) -> String {
    let app = Router::new().route(
        route,
        any(move || {
            let payload = payload.clone();
            async move { Json(payload) }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("listener should expose addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server should run");
    });

    format!("http://{address}")
}

// CoinGecko tests

#[tokio::test]
async fn fetches_coingecko_quote() {
    let base_url = start_test_server_at(
        "/simple/price",
        json!({
            "bitcoin": {
                "usd": 65432.10,
                "last_updated_at": 1742817600
            }
        }),
    )
    .await;

    let quote = fetch_coingecko_quote(&Client::new(), &base_url, "bitcoin")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "65432.1");
    assert_eq!(quote.currency, Currency::Usd);
    assert_eq!(quote.as_of, "2025-03-24T12:00:00Z");
}

#[tokio::test]
async fn coingecko_quote_fails_on_unknown_coin() {
    let base_url = start_test_server_at("/simple/price", json!({})).await;

    let result = fetch_coingecko_quote(&Client::new(), &base_url, "notacoin").await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no data for coin"));
}

#[tokio::test]
async fn refreshes_crypto_asset_price_via_coingecko() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "BTC".try_into().unwrap(),
            name: "Bitcoin".try_into().unwrap(),
            asset_type: AssetType::Crypto,
            quote_symbol: Some("bitcoin".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let coingecko_base_url = start_test_server_at(
        "/simple/price",
        json!({
            "bitcoin": {
                "usd": 65432.10,
                "last_updated_at": 1742817600
            }
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url,
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "65432.1");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(
        asset.current_price_as_of,
        Some("2025-03-24T12:00:00Z".to_string())
    );
}

// Twelve Data tests

#[tokio::test]
async fn fetches_twelve_data_quote() {
    let base_url = start_test_server_at(
        "/quote",
        json!({
            "close": "123.45",
            "currency": "USD",
            "datetime": "2026-03-24 10:15:00"
        }),
    )
    .await;

    let quote = fetch_twelve_data_quote(&Client::new(), &base_url, "test-key", "AAPL")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "123.45");
    assert_eq!(quote.currency, Currency::Usd);
    assert_eq!(quote.as_of, "2026-03-24T10:15:00Z");
}

#[tokio::test]
async fn fetches_twelve_data_quote_with_rfc3339_datetime() {
    let base_url = start_test_server_at(
        "/quote",
        json!({
            "close": "123.45",
            "currency": "USD",
            "datetime": "2026-03-24T10:15:00+00:00"
        }),
    )
    .await;

    let quote = fetch_twelve_data_quote(&Client::new(), &base_url, "test-key", "AAPL")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.as_of, "2026-03-24T10:15:00Z");
}

#[tokio::test]
async fn fetches_twelve_data_quote_with_date_only_datetime() {
    let base_url = start_test_server_at(
        "/quote",
        json!({
            "close": "123.45",
            "currency": "USD",
            "datetime": "2026-03-24"
        }),
    )
    .await;

    let quote = fetch_twelve_data_quote(&Client::new(), &base_url, "test-key", "AAPL")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.as_of, "2026-03-24T00:00:00Z");
}

#[tokio::test]
async fn refreshes_asset_prices_via_twelve_data() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("AAPL".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let base_url = start_test_server_at(
        "/quote",
        json!({
            "close": "210.12",
            "currency": "USD",
            "datetime": "2026-03-24 14:30:00"
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: base_url,
            twelve_data_api_key: Some("test-key".to_string()),
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "210.12");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(
        asset.current_price_as_of,
        Some("2026-03-24T14:30:00Z".to_string())
    );
}

// Finnhub tests

#[tokio::test]
async fn fetches_finnhub_quote() {
    let base_url = start_test_server_at(
        "/api/v1/quote",
        json!({
            "c": 173.57,
            "t": 1742817600
        }),
    )
    .await;

    let quote = fetch_finnhub_quote(&Client::new(), &base_url, "test-key", "AAPL")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "173.57");
    assert_eq!(quote.currency, Currency::Usd);
    assert_eq!(quote.as_of, "2025-03-24T12:00:00Z");
}

#[tokio::test]
async fn finnhub_quote_infers_eur_for_italian_exchange_symbol() {
    let base_url = start_test_server_at(
        "/api/v1/quote",
        json!({
            "c": 51.19,
            "t": 1742817600
        }),
    )
    .await;

    let quote = fetch_finnhub_quote(&Client::new(), &base_url, "test-key", "GRID.MI")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.currency, Currency::Eur);
}

#[tokio::test]
async fn finnhub_quote_fails_on_error_field() {
    let base_url = start_test_server_at(
        "/api/v1/quote",
        json!({ "error": "API limit reached. Please upgrade your plan" }),
    )
    .await;

    let result = fetch_finnhub_quote(&Client::new(), &base_url, "test-key", "AAPL").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("API limit reached")
    );
}

#[tokio::test]
async fn finnhub_quote_fails_on_unknown_symbol() {
    let base_url = start_test_server_at(
        "/api/v1/quote",
        json!({ "c": 0, "d": null, "dp": null, "h": 0, "l": 0, "o": 0, "pc": 0, "t": 0 }),
    )
    .await;

    let result = fetch_finnhub_quote(&Client::new(), &base_url, "test-key", "INVALID").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no data for symbol")
    );
}

#[tokio::test]
async fn refreshes_asset_prices_via_finnhub() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("AAPL".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let finnhub_base_url = start_test_server_at(
        "/api/v1/quote",
        json!({
            "c": 210.99,
            "t": 1742817600
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url,
            finnhub_api_key: Some("test-key".to_string()),
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "210.99");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(
        asset.current_price_as_of,
        Some("2025-03-24T12:00:00Z".to_string())
    );
}

// Alpha Vantage tests

#[tokio::test]
async fn fetches_alpha_vantage_quote() {
    let base_url = start_test_server_at(
        "/query",
        json!({
            "Global Quote": {
                "01. symbol": "AAPL",
                "05. price": "173.57",
                "07. latest trading day": "2026-03-24"
            }
        }),
    )
    .await;

    let quote = fetch_alpha_vantage_quote(&Client::new(), &base_url, "test-key", "AAPL")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "173.57");
    assert_eq!(quote.currency, Currency::Usd);
    assert_eq!(quote.as_of, "2026-03-24T00:00:00Z");
}

#[tokio::test]
async fn alpha_vantage_quote_fails_on_rate_limit_message() {
    let base_url = start_test_server_at(
        "/query",
        json!({
            "Information": "Thank you for using Alpha Vantage! Our standard API rate limit is 25 requests per day."
        }),
    )
    .await;

    let result = fetch_alpha_vantage_quote(&Client::new(), &base_url, "test-key", "AAPL").await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Alpha Vantage"));
}

#[tokio::test]
async fn alpha_vantage_quote_fails_on_empty_global_quote() {
    let base_url = start_test_server_at("/query", json!({ "Global Quote": {} })).await;

    let result = fetch_alpha_vantage_quote(&Client::new(), &base_url, "test-key", "INVALID").await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing price"));
}

// OpenFIGI tests

#[tokio::test]
async fn fetches_openfigi_tickers() {
    let base_url = start_test_server_at(
        "/v3/mapping",
        json!([{
            "data": [
                {"ticker": "EMIM", "name": "ISHS CORE MSCI EM IMI", "exchCode": "AEB"},
                {"ticker": "EIMI", "name": "ISHS CORE MSCI EM IMI", "exchCode": "LSE"},
                {"ticker": "IS3N", "name": "ISHS CORE MSCI EM IMI", "exchCode": "XETRA"}
            ]
        }]),
    )
    .await;

    let tickers = fetch_openfigi_tickers(&Client::new(), &base_url, None, "IE00BKM4GZ66")
        .await
        .expect("tickers fetch should succeed");

    assert_eq!(tickers, vec!["EMIM", "EIMI", "IS3N"]);
}

#[tokio::test]
async fn openfigi_tickers_fails_on_unknown_isin() {
    let base_url =
        start_test_server_at("/v3/mapping", json!([{"error": "No identifier found."}])).await;

    let result = fetch_openfigi_tickers(&Client::new(), &base_url, None, "XX0000000000").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No identifier found")
    );
}

#[tokio::test]
async fn tries_all_openfigi_tickers_until_success() {
    use axum::extract::Query;
    use std::collections::HashMap;

    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "EIMI".try_into().unwrap(),
            name: "iShares Core MSCI EM IMI UCITS ETF".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: Some("IE00BKM4GZ66".to_string()),
        },
    )
    .await
    .expect("asset should be created");

    // OpenFIGI returns two tickers: EMIM (fails) and EIMI (succeeds)
    let openfigi_base_url = start_test_server_at(
        "/v3/mapping",
        json!([{
            "data": [
                {"ticker": "EMIM", "exchCode": "AEB"},
                {"ticker": "EIMI", "exchCode": "LSE"}
            ]
        }]),
    )
    .await;

    // twelve_data server: succeeds only for EIMI, returns error for EMIM
    let twelve_data_app = Router::new().route(
        "/quote",
        any(|Query(params): Query<HashMap<String, String>>| async move {
            if params.get("symbol").is_some_and(|s| s == "EIMI") {
                Json(json!({"close": "22.50", "currency": "USD", "datetime": "2026-03-25"}))
            } else {
                Json(json!({"status": "error", "code": 404, "message": "symbol not found"}))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let twelve_data_base_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, twelve_data_app).await.unwrap();
    });

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url,
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url,
            twelve_data_api_key: Some("test-key".to_string()),
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "22.5");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(asset.quote_source_symbol.as_deref(), Some("EIMI"));
    assert_eq!(asset.quote_source_provider.as_deref(), Some("twelve_data"));
    assert_eq!(
        asset.quote_source_last_success_at.as_deref(),
        Some("2026-03-25T00:00:00Z")
    );
}

#[tokio::test]
async fn resolves_isin_via_openfigi_and_fetches_price() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: Some("US0378331005".to_string()),
        },
    )
    .await
    .expect("asset should be created");

    let openfigi_base_url = start_test_server_at(
        "/v3/mapping",
        json!([{
            "data": [{"ticker": "AAPL", "name": "APPLE INC", "exchCode": "US"}]
        }]),
    )
    .await;

    let finnhub_base_url = start_test_server_at(
        "/api/v1/quote",
        json!({
            "c": 188.50,
            "t": 1742817600
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url,
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url,
            finnhub_api_key: Some("test-key".to_string()),
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "188.5");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(asset.quote_source_symbol.as_deref(), Some("AAPL"));
    assert_eq!(asset.quote_source_provider.as_deref(), Some("finnhub"));
}

#[tokio::test]
async fn uses_cached_quote_source_before_openfigi_resolution() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "Apple".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: Some("US0378331005".to_string()),
        },
    )
    .await
    .expect("asset should be created");

    upsert_asset_quote_source(
        &pool,
        UpsertAssetQuoteSourceInput {
            asset_id,
            quote_symbol: "AAPL".to_string(),
            provider: "finnhub".to_string(),
            last_success_at: "2026-03-24T12:00:00Z".to_string(),
        },
    )
    .await
    .expect("quote source should be cached");

    let finnhub_base_url = start_test_server_at(
        "/api/v1/quote",
        json!({
            "c": 189.75,
            "t": 1742817600
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url,
            finnhub_api_key: Some("test-key".to_string()),
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should use cached source without OpenFIGI");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "189.75");
    assert_eq!(asset.quote_source_symbol.as_deref(), Some("AAPL"));
    assert_eq!(asset.quote_source_provider.as_deref(), Some("finnhub"));
}

#[tokio::test]
async fn falls_back_to_asset_symbol_when_openfigi_fails() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: Some("US0378331005".to_string()),
        },
    )
    .await
    .expect("asset should be created");

    // OpenFIGI is unreachable — should fall back to asset symbol "AAPL"
    let finnhub_base_url = start_test_server_at(
        "/api/v1/quote",
        json!({
            "c": 177.00,
            "t": 1742817600
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url,
            finnhub_api_key: Some("test-key".to_string()),
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed using asset symbol as fallback");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "177");
}

#[tokio::test]
async fn falls_back_to_next_provider_on_failure() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("AAPL".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    // twelve_data is configured but points to an unreachable address (will fail)
    // finnhub is configured with a working test server (should be used as fallback)
    let finnhub_base_url = start_test_server_at(
        "/api/v1/quote",
        json!({
            "c": 199.99,
            "t": 1742817600
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: Some("test-key".to_string()),
            finnhub_base_url,
            finnhub_api_key: Some("test-key".to_string()),
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed with fallback");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "199.99");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
}

#[tokio::test]
async fn refreshes_asset_prices_via_alpha_vantage() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("AAPL".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let alpha_vantage_base_url = start_test_server_at(
        "/query",
        json!({
            "Global Quote": {
                "01. symbol": "AAPL",
                "05. price": "175.42",
                "07. latest trading day": "2026-03-24"
            }
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url,
            alpha_vantage_api_key: Some("test-key".to_string()),
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "175.42");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(
        asset.current_price_as_of,
        Some("2026-03-24T00:00:00Z".to_string())
    );
}

// Polygon tests

#[tokio::test]
async fn fetches_polygon_quote() {
    let base_url = start_test_server_at(
        "/v2/aggs/ticker/AAPL/prev",
        json!({
            "status": "OK",
            "results": [{"c": 173.57, "t": 1742817600000_i64}]
        }),
    )
    .await;

    let quote = fetch_polygon_quote(&Client::new(), &base_url, "test-key", "AAPL")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "173.57");
    assert_eq!(quote.currency, Currency::Usd);
    assert_eq!(quote.as_of, "2025-03-24T12:00:00Z");
}

#[tokio::test]
async fn polygon_quote_fails_on_empty_results() {
    let base_url = start_test_server_at(
        "/v2/aggs/ticker/INVALID/prev",
        json!({"status": "OK", "results": []}),
    )
    .await;

    let result = fetch_polygon_quote(&Client::new(), &base_url, "test-key", "INVALID").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no data for symbol")
    );
}

#[tokio::test]
async fn refreshes_asset_prices_via_polygon() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("AAPL".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let polygon_base_url = start_test_server_at(
        "/v2/aggs/ticker/AAPL/prev",
        json!({
            "status": "OK",
            "results": [{"c": 182.00, "t": 1742817600000_i64}]
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url,
            polygon_api_key: Some("test-key".to_string()),
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "182");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(
        asset.current_price_as_of,
        Some("2025-03-24T12:00:00Z".to_string())
    );
}

// FMP tests

#[tokio::test]
async fn fetches_fmp_quote() {
    let base_url = start_test_server_at(
        "/stable/quote",
        json!([{"price": 173.57, "timestamp": 1742817600}]),
    )
    .await;

    let quote = fetch_fmp_quote(&Client::new(), &base_url, "test-key", "AAPL")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "173.57");
    assert_eq!(quote.currency, Currency::Usd);
    assert_eq!(quote.as_of, "2025-03-24T12:00:00Z");
}

#[tokio::test]
async fn fmp_quote_fails_on_empty_response() {
    let base_url = start_test_server_at("/stable/quote", json!([])).await;

    let result = fetch_fmp_quote(&Client::new(), &base_url, "test-key", "INVALID").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no data for symbol")
    );
}

#[tokio::test]
async fn refreshes_asset_prices_via_fmp() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("AAPL".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let fmp_base_url = start_test_server_at(
        "/stable/quote",
        json!([{"price": 195.50, "timestamp": 1742817600}]),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url,
            fmp_api_key: Some("test-key".to_string()),
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "195.5");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(
        asset.current_price_as_of,
        Some("2025-03-24T12:00:00Z".to_string())
    );
}

// EODHD tests

#[tokio::test]
async fn fetches_eodhd_quote() {
    let base_url = start_test_server_at(
        "/api/real-time/AAPL.US",
        json!({"close": 173.57, "timestamp": 1742817600}),
    )
    .await;

    let quote = fetch_eodhd_quote(&Client::new(), &base_url, "test-key", "AAPL.US")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "173.57");
    assert_eq!(quote.currency, Currency::Usd);
    assert_eq!(quote.as_of, "2025-03-24T12:00:00Z");
}

#[tokio::test]
async fn eodhd_quote_fails_on_missing_close() {
    let base_url = start_test_server_at(
        "/api/real-time/INVALID.US",
        json!({"timestamp": 1742817600}),
    )
    .await;

    let result = fetch_eodhd_quote(&Client::new(), &base_url, "test-key", "INVALID.US").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("missing close price")
    );
}

#[tokio::test]
async fn refreshes_asset_prices_via_eodhd() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("AAPL.US".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let eodhd_base_url = start_test_server_at(
        "/api/real-time/AAPL.US",
        json!({"close": 201.75, "timestamp": 1742817600}),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url,
            eodhd_api_key: Some("test-key".to_string()),
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "201.75");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(
        asset.current_price_as_of,
        Some("2025-03-24T12:00:00Z".to_string())
    );
}

// Tiingo tests

#[tokio::test]
async fn fetches_tiingo_quote() {
    let base_url = start_test_server_at(
        "/tiingo/daily/AAPL/prices",
        json!([{"close": 173.57, "date": "2026-03-24T00:00:00+00:00"}]),
    )
    .await;

    let quote = fetch_tiingo_quote(&Client::new(), &base_url, "test-key", "AAPL")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "173.57");
    assert_eq!(quote.currency, Currency::Usd);
    assert_eq!(quote.as_of, "2026-03-24T00:00:00Z");
}

#[tokio::test]
async fn tiingo_quote_fails_on_empty_response() {
    let base_url = start_test_server_at("/tiingo/daily/INVALID/prices", json!([])).await;

    let result = fetch_tiingo_quote(&Client::new(), &base_url, "test-key", "INVALID").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no data for symbol")
    );
}

#[tokio::test]
async fn refreshes_asset_prices_via_tiingo() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("AAPL".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let tiingo_base_url = start_test_server_at(
        "/tiingo/daily/AAPL/prices",
        json!([{"close": 178.90, "date": "2026-03-24T00:00:00+00:00"}]),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url,
            tiingo_api_key: Some("test-key".to_string()),
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "178.9");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(
        asset.current_price_as_of,
        Some("2026-03-24T00:00:00Z".to_string())
    );
}

// Marketstack tests

#[tokio::test]
async fn fetches_marketstack_quote() {
    let base_url = start_test_server_at(
        "/v1/eod/latest",
        json!({"data": [{"close": 173.57, "date": "2026-03-24T00:00:00+0000"}]}),
    )
    .await;

    let quote = fetch_marketstack_quote(&Client::new(), &base_url, "test-key", "AAPL")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "173.57");
    assert_eq!(quote.currency, Currency::Usd);
    assert_eq!(quote.as_of, "2026-03-24T00:00:00Z");
}

#[tokio::test]
async fn marketstack_quote_fails_on_empty_data() {
    let base_url = start_test_server_at("/v1/eod/latest", json!({"data": []})).await;

    let result = fetch_marketstack_quote(&Client::new(), &base_url, "test-key", "INVALID").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no data for symbol")
    );
}

#[tokio::test]
async fn refreshes_asset_prices_via_marketstack() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "AAPL".try_into().unwrap(),
            name: "Apple".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("AAPL".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let marketstack_base_url = start_test_server_at(
        "/v1/eod/latest",
        json!({"data": [{"close": 165.33, "date": "2026-03-24T00:00:00+0000"}]}),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: "http://127.0.0.1:1".to_string(),
            yahoo_finance_enabled: false,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url,
            marketstack_api_key: Some("test-key".to_string()),
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "165.33");
    assert_eq!(asset.current_price_currency, Some(Currency::Usd));
    assert_eq!(
        asset.current_price_as_of,
        Some("2026-03-24T00:00:00Z".to_string())
    );
}

// Yahoo Finance tests

#[tokio::test]
async fn fetches_yahoo_finance_quote() {
    use axum::http::{HeaderMap, header::USER_AGENT};

    let app = Router::new().route(
        "/v8/finance/chart/GRID.MI",
        any(|headers: HeaderMap| async move {
            let user_agent = headers
                .get(USER_AGENT)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();

            if !user_agent.contains("Mozilla/5.0") {
                return Json(json!({
                    "chart": {
                        "result": null,
                        "error": {
                            "description": format!("unexpected User-Agent: {user_agent}")
                        }
                    }
                }));
            }

            Json(json!({
                "chart": {
                    "result": [{
                        "meta": {
                            "regularMarketPrice": 51.19,
                            "regularMarketTime": 1742817600,
                            "currency": "EUR"
                        }
                    }],
                    "error": null
                }
            }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let quote = fetch_yahoo_quote(&Client::new(), &base_url, "GRID.MI")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "51.19");
    assert_eq!(quote.currency, Currency::Eur);
    assert_eq!(quote.as_of, "2025-03-24T12:00:00Z");
}

#[tokio::test]
async fn yahoo_finance_quote_defaults_to_usd_for_unknown_currency() {
    let base_url = start_test_server_at(
        "/v8/finance/chart/TEST",
        json!({
            "chart": {
                "result": [{
                    "meta": {
                        "regularMarketPrice": 100.0,
                        "regularMarketTime": 1742817600,
                        "currency": "JPY"
                    }
                }],
                "error": null
            }
        }),
    )
    .await;

    let quote = fetch_yahoo_quote(&Client::new(), &base_url, "TEST")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.currency, Currency::Usd);
}

#[tokio::test]
async fn yahoo_finance_quote_fails_on_error_response() {
    let base_url = start_test_server_at(
        "/v8/finance/chart/INVALID",
        json!({
            "chart": {
                "result": null,
                "error": {
                    "code": "Not Found",
                    "description": "No data found for symbol INVALID"
                }
            }
        }),
    )
    .await;

    let result = fetch_yahoo_quote(&Client::new(), &base_url, "INVALID").await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No data found"));
}

#[tokio::test]
async fn refreshes_asset_prices_via_yahoo_finance() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "GRID".try_into().unwrap(),
            name: "Smart Grid ETF".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("GRID.MI".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let base_url = start_test_server_at(
        "/v8/finance/chart/GRID.MI",
        json!({
            "chart": {
                "result": [{
                    "meta": {
                        "regularMarketPrice": 51.19,
                        "regularMarketTime": 1742817600,
                        "currency": "EUR"
                    }
                }],
                "error": null
            }
        }),
    )
    .await;

    let updated_count = refresh_asset_prices(
        &pool,
        &Client::new(),
        &AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            coingecko_base_url: "http://127.0.0.1:1".to_string(),
            coincap_base_url: "http://127.0.0.1:1".to_string(),
            coincap_api_key: None,
            openfigi_base_url: "http://127.0.0.1:1".to_string(),
            openfigi_api_key: None,
            yahoo_finance_base_url: base_url,
            yahoo_finance_enabled: true,
            twelve_data_base_url: "http://127.0.0.1:1".to_string(),
            twelve_data_api_key: None,
            finnhub_base_url: "http://127.0.0.1:1".to_string(),
            finnhub_api_key: None,
            alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
            alpha_vantage_api_key: None,
            polygon_base_url: "http://127.0.0.1:1".to_string(),
            polygon_api_key: None,
            fmp_base_url: "http://127.0.0.1:1".to_string(),
            fmp_api_key: None,
            eodhd_base_url: "http://127.0.0.1:1".to_string(),
            eodhd_api_key: None,
            tiingo_base_url: "http://127.0.0.1:1".to_string(),
            tiingo_api_key: None,
            marketstack_base_url: "http://127.0.0.1:1".to_string(),
            marketstack_api_key: None,
            fcsapi_base_url: "http://127.0.0.1:1".to_string(),
            fcsapi_api_key: None,
            itick_base_url: "http://127.0.0.1:1".to_string(),
            itick_api_key: None,
        },
    )
    .await
    .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "51.19");
    assert_eq!(asset.current_price_currency, Some(Currency::Eur));
    assert_eq!(
        asset.current_price_as_of,
        Some("2025-03-24T12:00:00Z".to_string())
    );
}

#[tokio::test]
async fn refreshes_grid_mi_via_default_yahoo_finance_provider() {
    let pool = test_pool().await;
    let asset_id = crate::create_asset(
        &pool,
        CreateAssetInput {
            symbol: "GRID".try_into().unwrap(),
            name: "Smart Grid ETF".try_into().unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: Some("GRID.MI".to_string()),
            isin: None,
        },
    )
    .await
    .expect("asset should be created");

    let base_url = start_test_server_at(
        "/v8/finance/chart/GRID.MI",
        json!({
            "chart": {
                "result": [{
                    "meta": {
                        "regularMarketPrice": 51.19,
                        "regularMarketTime": 1742817600,
                        "currency": "EUR"
                    }
                }],
                "error": null
            }
        }),
    )
    .await;

    let config = Config::parse_from([
        "siniscalco",
        "--yahoo-finance-base-url",
        base_url.as_str(),
        "--twelve-data-api-key",
        "",
        "--finnhub-api-key",
        "",
        "--alpha-vantage-api-key",
        "",
        "--polygon-api-key",
        "",
        "--fmp-api-key",
        "",
        "--eodhd-api-key",
        "",
        "--tiingo-api-key",
        "",
        "--marketstack-api-key",
        "",
        "--fcsapi-api-key",
        "",
        "--itick-api-key",
        "",
    ]);
    let price_config = config.asset_price_refresh_config();
    assert!(price_config.yahoo_finance_enabled);

    let updated_count = refresh_asset_prices(&pool, &Client::new(), &price_config)
        .await
        .expect("refresh should succeed");

    let asset = get_asset(&pool, asset_id)
        .await
        .expect("asset should load with price");

    assert_eq!(updated_count, 1);
    assert_eq!(asset.current_price.unwrap().to_string(), "51.19");
    assert_eq!(asset.current_price_currency, Some(Currency::Eur));
    assert_eq!(asset.quote_source_symbol.as_deref(), Some("GRID.MI"));
    assert_eq!(asset.quote_source_provider.as_deref(), Some("yahoo"));
}

#[tokio::test]
async fn fetches_twelve_data_quote_for_european_symbol() {
    let base_url = start_test_server_at(
        "/quote",
        json!({
            "close": "51.19",
            "currency": "EUR",
            "datetime": "2026-03-24 16:30:00"
        }),
    )
    .await;

    let quote = fetch_twelve_data_quote(&Client::new(), &base_url, "test-key", "GRID.MI")
        .await
        .expect("quote fetch should succeed");

    assert_eq!(quote.price.to_string(), "51.19");
    assert_eq!(quote.currency, Currency::Eur);
    assert_eq!(quote.as_of, "2026-03-24T16:30:00Z");
}
