use std::{env, str::FromStr, sync::Arc, time::Duration};

use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::{sync::RwLock, time::sleep};
use tracing::{info, warn};

use crate::storage::StorageError;
use crate::{
    Currency, FxRate, UpsertFxRateInput, current_utc_timestamp, format_decimal_amount,
    replace_fx_rates,
};

pub const PRODUCT_BASE_CURRENCY: Currency = Currency::Eur;
const DEFAULT_FRANKFURTER_BASE_URL: &str = "https://api.frankfurter.dev/v1";
const REFRESH_INTERVAL_SECS: u64 = 60 * 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FxRefreshStatus {
    pub availability: FxRefreshAvailability,
    pub last_error: Option<String>,
}

impl FxRefreshStatus {
    pub fn available() -> Self {
        Self {
            availability: FxRefreshAvailability::Available,
            last_error: None,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            availability: FxRefreshAvailability::Unavailable,
            last_error: Some(message.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FxRefreshAvailability {
    Available,
    Unavailable,
}

impl FxRefreshAvailability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
        }
    }
}

pub type SharedFxRefreshStatus = Arc<RwLock<FxRefreshStatus>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FxRefreshConfig {
    pub refresh_interval: Duration,
    pub base_url: String,
}

impl FxRefreshConfig {
    pub fn load() -> Self {
        let base_url = env::var("FX_REFRESH_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_FRANKFURTER_BASE_URL.to_string());

        Self {
            refresh_interval: Duration::from_secs(REFRESH_INTERVAL_SECS),
            base_url,
        }
    }
}

#[derive(Debug)]
pub enum FxRefreshError {
    Config(String),
    Provider(String),
    Storage(StorageError),
}

impl std::fmt::Display for FxRefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(message) | Self::Provider(message) => f.write_str(message),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl From<StorageError> for FxRefreshError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

#[derive(Deserialize)]
struct FrankfurterLatestResponse {
    rates: std::collections::BTreeMap<String, serde_json::Number>,
}

pub fn new_shared_fx_refresh_status() -> SharedFxRefreshStatus {
    Arc::new(RwLock::new(FxRefreshStatus::unavailable(
        "FX refresh unavailable: no successful refresh has completed",
    )))
}

pub async fn spawn_fx_refresh_task(
    pool: SqlitePool,
    status: SharedFxRefreshStatus,
    config: FxRefreshConfig,
) {
    tokio::spawn(async move {
        let client = Client::new();

        loop {
            info!(
                endpoint = %config.base_url,
                refresh_interval_seconds = config.refresh_interval.as_secs(),
                "starting fx refresh"
            );
            let refresh_result = refresh_fx_rates(&pool, &client, &config).await;
            let next_status = match refresh_result {
                Ok(()) => {
                    info!("fx refresh succeeded");
                    FxRefreshStatus::available()
                }
                Err(error) => {
                    warn!(error = %error, "fx refresh failed");
                    FxRefreshStatus::unavailable(error.to_string())
                }
            };

            *status.write().await = next_status;
            sleep(config.refresh_interval).await;
        }
    });
}

pub async fn refresh_fx_rates(
    pool: &SqlitePool,
    client: &Client,
    config: &FxRefreshConfig,
) -> Result<(), FxRefreshError> {
    let fetched_at = current_utc_timestamp()?;
    let rates = fetch_frankfurter_rates(client, config).await?;
    info!(fetched_at = %fetched_at, pair_count = rates.len(), "storing refreshed fx rates");
    replace_fx_rates(pool, rates, &fetched_at).await?;
    Ok(())
}

pub async fn fetch_frankfurter_rates(
    client: &Client,
    config: &FxRefreshConfig,
) -> Result<Vec<UpsertFxRateInput>, FxRefreshError> {
    let symbols = Currency::supported_non_base()
        .into_iter()
        .map(Currency::as_str)
        .collect::<Vec<_>>()
        .join(",");
    let url = format!("{}/latest", config.base_url);
    let response = client
        .get(url)
        .query(&[("symbols", symbols.as_str())])
        .send()
        .await
        .map_err(|error| FxRefreshError::Provider(format!("FX refresh failed: {error}")))?;

    if !response.status().is_success() {
        return Err(FxRefreshError::Provider(format!(
            "FX refresh failed: provider returned status {}",
            response.status()
        )));
    }

    let payload = response
        .json::<FrankfurterLatestResponse>()
        .await
        .map_err(|error| FxRefreshError::Provider(format!("FX refresh failed: {error}")))?;

    Currency::supported_non_base()
        .into_iter()
        .map(|currency| {
            let quoted_rate = payload.rates.get(currency.as_str()).ok_or_else(|| {
                FxRefreshError::Provider(format!(
                    "FX refresh failed: provider response missing {} quote",
                    currency.as_str()
                ))
            })?;
            let quoted_rate = Decimal::from_str(&quoted_rate.to_string()).map_err(|_| {
                FxRefreshError::Provider(format!(
                    "FX refresh failed: provider returned invalid {} quote",
                    currency.as_str()
                ))
            })?;

            if quoted_rate <= Decimal::ZERO {
                return Err(FxRefreshError::Provider(format!(
                    "FX refresh failed: provider returned non-positive {} quote",
                    currency.as_str()
                )));
            }

            let inverse_rate = Decimal::ONE / quoted_rate;
            let fx_rate = FxRate::try_from(format_decimal_amount(inverse_rate).as_str())?;

            Ok(UpsertFxRateInput {
                from_currency: currency,
                to_currency: PRODUCT_BASE_CURRENCY,
                rate: fx_rate,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, str::FromStr, time::Duration};

    use axum::{Json, Router, routing::get};
    use reqwest::Client;
    use serde_json::json;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tokio::net::TcpListener;

    use super::{FxRefreshConfig, fetch_frankfurter_rates, refresh_fx_rates};
    use crate::{Currency, FxRate, get_latest_fx_rate, init_db};

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

    async fn start_test_server(body: serde_json::Value, status: u16) -> SocketAddr {
        let app = Router::new().route(
            "/latest",
            get(move || {
                let body = body.clone();
                async move {
                    (
                        axum::http::StatusCode::from_u16(status).unwrap(),
                        Json(body),
                    )
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let address = listener.local_addr().expect("local addr should resolve");

        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should keep serving");
        });

        address
    }

    #[tokio::test]
    async fn fetches_supported_non_base_rates_from_frankfurter() {
        let address = start_test_server(
            json!({
                "rates": {
                    "CHF": 0.95,
                    "GBP": 0.85,
                    "USD": 1.10
                }
            }),
            200,
        )
        .await;
        let client = Client::new();
        let config = FxRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            base_url: format!("http://{address}"),
        };

        let rates = fetch_frankfurter_rates(&client, &config)
            .await
            .expect("rates should fetch");

        assert_eq!(rates.len(), 3);
        assert_eq!(rates[0].from_currency, Currency::Chf);
        assert_eq!(rates[0].to_currency, Currency::Eur);
        assert_eq!(rates[0].rate, FxRate::try_from("1.05263158").unwrap());
        assert_eq!(rates[1].from_currency, Currency::Gbp);
        assert_eq!(rates[2].from_currency, Currency::Usd);
    }

    #[tokio::test]
    async fn refresh_replaces_existing_rates_and_preserves_previous_data_on_failure() {
        let pool = test_pool().await;
        let client = Client::new();

        let success_address = start_test_server(
            json!({
                "rates": {
                    "CHF": 0.95,
                    "GBP": 0.85,
                    "USD": 1.10
                }
            }),
            200,
        )
        .await;
        let success_config = FxRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            base_url: format!("http://{success_address}"),
        };

        refresh_fx_rates(&pool, &client, &success_config)
            .await
            .expect("refresh should succeed");
        let first_rate = get_latest_fx_rate(&pool, Currency::Usd, Currency::Eur)
            .await
            .expect("fx lookup should succeed")
            .expect("usd eur rate should exist");

        let replacement_address = start_test_server(
            json!({
                "rates": {
                    "CHF": 0.80,
                    "GBP": 0.90,
                    "USD": 1.25
                }
            }),
            200,
        )
        .await;
        let replacement_config = FxRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            base_url: format!("http://{replacement_address}"),
        };

        refresh_fx_rates(&pool, &client, &replacement_config)
            .await
            .expect("replacement refresh should succeed");
        let replaced_rate = get_latest_fx_rate(&pool, Currency::Usd, Currency::Eur)
            .await
            .expect("fx lookup should succeed")
            .expect("usd eur rate should exist");

        assert_eq!(replaced_rate.rate, FxRate::try_from("0.80000000").unwrap());
        assert_ne!(first_rate.rate, replaced_rate.rate);

        let failing_address = start_test_server(json!({ "error": "boom" }), 500).await;
        let failing_config = FxRefreshConfig {
            refresh_interval: Duration::from_secs(60),
            base_url: format!("http://{failing_address}"),
        };

        refresh_fx_rates(&pool, &client, &failing_config)
            .await
            .expect_err("failed refresh should surface");
        let preserved_rate = get_latest_fx_rate(&pool, Currency::Usd, Currency::Eur)
            .await
            .expect("fx lookup should succeed")
            .expect("usd eur rate should remain stored");

        assert_eq!(preserved_rate.rate, replaced_rate.rate);
        assert_eq!(preserved_rate.updated_at, replaced_rate.updated_at);
    }
}
