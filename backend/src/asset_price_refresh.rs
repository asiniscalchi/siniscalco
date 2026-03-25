use std::{env, time::Duration};

use reqwest::Client;
use serde::Deserialize;
use sqlx::SqlitePool;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::{
    AssetId, AssetUnitPrice, Currency, UpsertAssetPriceInput, current_utc_timestamp_iso8601,
    get_asset, list_assets, upsert_asset_price,
};

const DEFAULT_TWELVE_DATA_BASE_URL: &str = "https://api.twelvedata.com";
const DEFAULT_ALPHA_VANTAGE_BASE_URL: &str = "https://www.alphavantage.co";
const DEFAULT_FINNHUB_BASE_URL: &str = "https://finnhub.io";
const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 60 * 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetPriceRefreshConfig {
    pub refresh_interval: Duration,
    pub base_url: String,
    pub api_key: Option<String>,
    pub finnhub_base_url: String,
    pub finnhub_api_key: Option<String>,
    pub alpha_vantage_base_url: String,
    pub alpha_vantage_api_key: Option<String>,
}

impl AssetPriceRefreshConfig {
    pub fn load() -> Self {
        let base_url = env::var("ASSET_PRICE_REFRESH_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_TWELVE_DATA_BASE_URL.to_string());

        let api_key = env::var("TWELVE_DATA_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let finnhub_base_url = env::var("FINNHUB_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_FINNHUB_BASE_URL.to_string());

        let finnhub_api_key = env::var("FINNHUB_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let alpha_vantage_base_url = env::var("ALPHA_VANTAGE_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_ALPHA_VANTAGE_BASE_URL.to_string());

        let alpha_vantage_api_key = env::var("ALPHA_VANTAGE_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Self {
            refresh_interval: Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS),
            base_url,
            api_key,
            finnhub_base_url,
            finnhub_api_key,
            alpha_vantage_base_url,
            alpha_vantage_api_key,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.api_key.is_some()
            || self.finnhub_api_key.is_some()
            || self.alpha_vantage_api_key.is_some()
    }
}

#[derive(Debug)]
pub enum AssetPriceRefreshError {
    Config(&'static str),
    Provider(String),
    Storage(crate::storage::StorageError),
}

impl std::fmt::Display for AssetPriceRefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(message) => f.write_str(message),
            Self::Provider(message) => f.write_str(message),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl From<crate::storage::StorageError> for AssetPriceRefreshError {
    fn from(value: crate::storage::StorageError) -> Self {
        Self::Storage(value)
    }
}

#[derive(Debug, Deserialize)]
struct TwelveDataQuoteResponse {
    close: Option<String>,
    currency: Option<String>,
    datetime: Option<String>,
    timestamp: Option<i64>,
    code: Option<i64>,
    message: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AssetQuote {
    pub price: AssetUnitPrice,
    pub currency: Currency,
    pub as_of: String,
}

pub async fn spawn_asset_price_refresh_task(pool: SqlitePool, config: AssetPriceRefreshConfig) {
    if !config.is_enabled() {
        warn!(
            "asset price refresh disabled: none of TWELVE_DATA_API_KEY, FINNHUB_API_KEY, or ALPHA_VANTAGE_API_KEY is set"
        );
        return;
    }

    tokio::spawn(async move {
        let client = Client::new();

        loop {
            info!(
                endpoint = %config.base_url,
                refresh_interval_seconds = config.refresh_interval.as_secs(),
                "starting asset price refresh"
            );

            match refresh_asset_prices(&pool, &client, &config).await {
                Ok(updated_count) => info!(updated_count, "asset price refresh succeeded"),
                Err(error) => warn!(error = %error, "asset price refresh failed"),
            }

            sleep(config.refresh_interval).await;
        }
    });
}

pub async fn refresh_asset_prices(
    pool: &SqlitePool,
    client: &Client,
    config: &AssetPriceRefreshConfig,
) -> Result<usize, AssetPriceRefreshError> {
    if !config.is_enabled() {
        return Err(AssetPriceRefreshError::Config(
            "asset price refresh disabled: none of TWELVE_DATA_API_KEY, FINNHUB_API_KEY, or ALPHA_VANTAGE_API_KEY is set",
        ));
    }

    let assets = list_assets(pool).await?;
    let mut updated_count = 0usize;

    for asset in assets {
        match refresh_single_asset_price(pool, client, config, asset.id).await {
            Ok(true) => {
                updated_count += 1;
            }
            Ok(false) => {}
            Err(error) => {
                warn!(
                    asset_id = asset.id.as_i64(),
                    error = %error,
                    "asset price refresh failed for stored asset"
                );
            }
        }
    }

    Ok(updated_count)
}

pub async fn refresh_single_asset_price(
    pool: &SqlitePool,
    client: &Client,
    config: &AssetPriceRefreshConfig,
    asset_id: AssetId,
) -> Result<bool, AssetPriceRefreshError> {
    if !config.is_enabled() {
        return Ok(false);
    }

    let asset = get_asset(pool, asset_id).await?;
    let symbol = asset
        .quote_symbol
        .as_deref()
        .unwrap_or_else(|| asset.symbol.as_str());

    let quote = if let Some(api_key) = config.api_key.as_deref() {
        fetch_twelve_data_quote(client, &config.base_url, api_key, symbol).await?
    } else if let Some(api_key) = config.finnhub_api_key.as_deref() {
        fetch_finnhub_quote(client, &config.finnhub_base_url, api_key, symbol).await?
    } else if let Some(api_key) = config.alpha_vantage_api_key.as_deref() {
        fetch_alpha_vantage_quote(client, &config.alpha_vantage_base_url, api_key, symbol).await?
    } else {
        return Ok(false);
    };

    upsert_asset_price(
        pool,
        UpsertAssetPriceInput {
            asset_id,
            price: quote.price,
            currency: quote.currency,
            as_of: quote.as_of,
        },
    )
    .await?;

    Ok(true)
}

pub async fn fetch_twelve_data_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!("{}/quote", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .query(&[("symbol", symbol), ("apikey", api_key)])
        .send()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    if !response.status().is_success() {
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned status {}",
            response.status()
        )));
    }

    let payload = response
        .json::<TwelveDataQuoteResponse>()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    if payload.status.as_deref() == Some("error") || payload.code.is_some() {
        return Err(AssetPriceRefreshError::Provider(
            payload
                .message
                .unwrap_or_else(|| "asset price refresh failed: provider returned an error".into()),
        ));
    }

    let price = payload
        .close
        .as_deref()
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: provider response missing close price".into(),
            )
        })
        .and_then(|price| AssetUnitPrice::try_from(price).map_err(AssetPriceRefreshError::from))?;

    let currency = payload
        .currency
        .as_deref()
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: provider response missing currency".into(),
            )
        })
        .and_then(|currency| Currency::try_from(currency).map_err(AssetPriceRefreshError::from))?;

    let as_of = match payload.datetime {
        Some(datetime) => normalize_provider_datetime(datetime)?,
        None if payload.timestamp.is_some() => current_utc_timestamp_iso8601()?,
        None => current_utc_timestamp_iso8601()?,
    };

    Ok(AssetQuote {
        price,
        currency,
        as_of,
    })
}

#[derive(Debug, Deserialize)]
struct FinnhubQuoteResponse {
    c: Option<serde_json::Number>,
    t: Option<i64>,
    error: Option<String>,
}

/// Fetches a quote from the Finnhub `/api/v1/quote` endpoint.
/// Note: Finnhub does not return the currency in this endpoint.
/// Prices are returned in the currency of the exchange where the symbol trades (defaults to USD).
pub async fn fetch_finnhub_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!("{}/api/v1/quote", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .query(&[("symbol", symbol), ("token", api_key)])
        .send()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    if !response.status().is_success() {
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned status {}",
            response.status()
        )));
    }

    let payload = response
        .json::<FinnhubQuoteResponse>()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    if let Some(error) = payload.error {
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: {error}"
        )));
    }

    let timestamp = payload.t.unwrap_or(0);
    if timestamp == 0 {
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned no data for symbol {symbol}"
        )));
    }

    let price_str = payload.c.as_ref().map(|n| n.to_string()).ok_or_else(|| {
        AssetPriceRefreshError::Provider(
            "asset price refresh failed: provider response missing price".into(),
        )
    })?;

    let price =
        AssetUnitPrice::try_from(price_str.as_str()).map_err(AssetPriceRefreshError::from)?;

    let as_of = OffsetDateTime::from_unix_timestamp(timestamp)
        .map_err(|_| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: provider returned invalid timestamp".into(),
            )
        })?
        .format(&Rfc3339)
        .map_err(|_| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: failed to format timestamp".into(),
            )
        })?;

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}

#[derive(Debug, Deserialize)]
struct AlphaVantageGlobalQuote {
    #[serde(rename = "05. price")]
    price: Option<String>,
    #[serde(rename = "07. latest trading day")]
    latest_trading_day: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlphaVantageQuoteResponse {
    #[serde(rename = "Global Quote")]
    global_quote: Option<AlphaVantageGlobalQuote>,
    #[serde(rename = "Information")]
    information: Option<String>,
}

/// Fetches a quote from the Alpha Vantage GLOBAL_QUOTE endpoint.
/// Note: Alpha Vantage does not return the currency in this endpoint.
/// Prices are returned in the currency of the exchange where the symbol trades (defaults to USD).
pub async fn fetch_alpha_vantage_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!("{}/query", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .query(&[
            ("function", "GLOBAL_QUOTE"),
            ("symbol", symbol),
            ("apikey", api_key),
        ])
        .send()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    if !response.status().is_success() {
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned status {}",
            response.status()
        )));
    }

    let payload = response
        .json::<AlphaVantageQuoteResponse>()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    if let Some(information) = payload.information {
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: {information}"
        )));
    }

    let quote = payload
        .global_quote
        .as_ref()
        .and_then(|q| q.price.as_deref())
        .filter(|p| !p.is_empty())
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: provider response missing price".into(),
            )
        })?;

    let price = AssetUnitPrice::try_from(quote).map_err(AssetPriceRefreshError::from)?;

    let as_of = payload
        .global_quote
        .as_ref()
        .and_then(|q| q.latest_trading_day.clone())
        .map(normalize_provider_datetime)
        .transpose()?
        .unwrap_or_else(|| current_utc_timestamp_iso8601().unwrap_or_default());

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}

fn normalize_provider_datetime(datetime: String) -> Result<String, AssetPriceRefreshError> {
    if let Ok(value) = OffsetDateTime::parse(&datetime, &Rfc3339) {
        return value.format(&Rfc3339).map_err(|_| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: provider returned invalid datetime".into(),
            )
        });
    }

    const DATETIME_WITH_SPACE: &[time::format_description::FormatItem<'static>] =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    const DATETIME_WITH_T: &[time::format_description::FormatItem<'static>] =
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    const DATE_ONLY: &[time::format_description::FormatItem<'static>] =
        format_description!("[year]-[month]-[day]");

    if let Ok(value) = PrimitiveDateTime::parse(&datetime, DATETIME_WITH_SPACE) {
        return Ok(format!(
            "{}Z",
            value.format(DATETIME_WITH_T).map_err(|_| {
                AssetPriceRefreshError::Provider(
                    "asset price refresh failed: provider returned invalid datetime".into(),
                )
            })?
        ));
    }

    if let Ok(value) = PrimitiveDateTime::parse(&datetime, DATETIME_WITH_T) {
        return Ok(format!(
            "{}Z",
            value.format(DATETIME_WITH_T).map_err(|_| {
                AssetPriceRefreshError::Provider(
                    "asset price refresh failed: provider returned invalid datetime".into(),
                )
            })?
        ));
    }

    if let Ok(value) = Date::parse(&datetime, DATE_ONLY) {
        return Ok(format!(
            "{}T00:00:00Z",
            value.format(DATE_ONLY).map_err(|_| {
                AssetPriceRefreshError::Provider(
                    "asset price refresh failed: provider returned invalid datetime".into(),
                )
            })?
        ));
    }

    Err(AssetPriceRefreshError::Provider(format!(
        "asset price refresh failed: unsupported datetime format: {datetime}"
    )))
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, time::Duration};

    use axum::{Json, Router, routing::get};
    use reqwest::Client;
    use serde_json::json;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tokio::net::TcpListener;

    use super::{
        AssetPriceRefreshConfig, fetch_alpha_vantage_quote, fetch_finnhub_quote,
        fetch_twelve_data_quote, refresh_asset_prices,
    };
    use crate::{AssetType, CreateAssetInput, Currency, get_asset, init_db};

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
            get(move || {
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
                base_url,
                api_key: Some("test-key".to_string()),
                finnhub_base_url: "http://127.0.0.1:1".to_string(),
                finnhub_api_key: None,
                alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
                alpha_vantage_api_key: None,
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
                base_url: "http://127.0.0.1:1".to_string(),
                api_key: None,
                finnhub_base_url,
                finnhub_api_key: Some("test-key".to_string()),
                alpha_vantage_base_url: "http://127.0.0.1:1".to_string(),
                alpha_vantage_api_key: None,
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

        let result =
            fetch_alpha_vantage_quote(&Client::new(), &base_url, "test-key", "INVALID").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing price"));
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
                base_url: "http://127.0.0.1:1".to_string(),
                api_key: None,
                finnhub_base_url: "http://127.0.0.1:1".to_string(),
                finnhub_api_key: None,
                alpha_vantage_base_url,
                alpha_vantage_api_key: Some("test-key".to_string()),
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
}
