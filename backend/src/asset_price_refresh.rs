use std::{env, time::Duration};

use reqwest::Client;
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::{
    AssetUnitPrice, Currency, UpsertAssetPriceInput, current_utc_timestamp_iso8601, list_assets,
    upsert_asset_price,
};

const DEFAULT_TWELVE_DATA_BASE_URL: &str = "https://api.twelvedata.com";
const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 60 * 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetPriceRefreshConfig {
    pub refresh_interval: Duration,
    pub base_url: String,
    pub api_key: Option<String>,
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

        Self {
            refresh_interval: Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS),
            base_url,
            api_key,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.api_key.is_some()
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
        warn!("asset price refresh disabled: TWELVE_DATA_API_KEY is not set");
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
    let api_key = config
        .api_key
        .as_deref()
        .ok_or(AssetPriceRefreshError::Config(
            "asset price refresh disabled: TWELVE_DATA_API_KEY is not set",
        ))?;

    let assets = list_assets(pool).await?;
    let mut updated_count = 0usize;

    for asset in assets {
        let symbol = asset
            .quote_symbol
            .as_deref()
            .unwrap_or_else(|| asset.symbol.as_str());

        match fetch_twelve_data_quote(client, &config.base_url, api_key, symbol).await {
            Ok(quote) => {
                upsert_asset_price(
                    pool,
                    UpsertAssetPriceInput {
                        asset_id: asset.id,
                        price: quote.price,
                        currency: quote.currency,
                        as_of: quote.as_of,
                    },
                )
                .await?;
                updated_count += 1;
            }
            Err(error) => {
                warn!(
                    asset_id = asset.id.as_i64(),
                    symbol,
                    error = %error,
                    "skipping asset price refresh for symbol"
                );
            }
        }
    }

    Ok(updated_count)
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

fn normalize_provider_datetime(datetime: String) -> Result<String, AssetPriceRefreshError> {
    if datetime.ends_with('Z') {
        return Ok(datetime);
    }

    if datetime.len() == 19 && datetime.as_bytes()[10] == b' ' {
        return Ok(format!("{}Z", datetime.replace(' ', "T")));
    }

    Err(AssetPriceRefreshError::Provider(
        "asset price refresh failed: provider returned invalid datetime".into(),
    ))
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, time::Duration};

    use axum::{Json, Router, routing::get};
    use reqwest::Client;
    use serde_json::json;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tokio::net::TcpListener;

    use super::{AssetPriceRefreshConfig, fetch_twelve_data_quote, refresh_asset_prices};
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

    async fn start_test_server(payload: serde_json::Value) -> String {
        let app = Router::new().route(
            "/quote",
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

    #[tokio::test]
    async fn fetches_twelve_data_quote() {
        let base_url = start_test_server(json!({
            "close": "123.45",
            "currency": "USD",
            "datetime": "2026-03-24 10:15:00"
        }))
        .await;

        let quote = fetch_twelve_data_quote(&Client::new(), &base_url, "test-key", "AAPL")
            .await
            .expect("quote fetch should succeed");

        assert_eq!(quote.price.to_string(), "123.45");
        assert_eq!(quote.currency, Currency::Usd);
        assert_eq!(quote.as_of, "2026-03-24T10:15:00Z");
    }

    #[tokio::test]
    async fn refreshes_asset_prices_into_storage() {
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

        let base_url = start_test_server(json!({
            "close": "210.12",
            "currency": "USD",
            "datetime": "2026-03-24 14:30:00"
        }))
        .await;

        let updated_count = refresh_asset_prices(
            &pool,
            &Client::new(),
            &AssetPriceRefreshConfig {
                refresh_interval: Duration::from_secs(60),
                base_url,
                api_key: Some("test-key".to_string()),
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
}
