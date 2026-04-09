mod config;
mod providers;
mod refresh;
mod types;

#[cfg(test)]
mod tests;

pub use config::AssetPriceRefreshConfig;
pub use providers::{
    fetch_alpha_vantage_quote, fetch_coingecko_quote, fetch_eodhd_quote, fetch_finnhub_quote,
    fetch_fmp_quote, fetch_marketstack_quote, fetch_openfigi_tickers, fetch_polygon_quote,
    fetch_tiingo_quote, fetch_twelve_data_quote,
};
pub use refresh::{fill_missing_asset_prices, refresh_asset_prices, refresh_single_asset_price};
pub use types::{AssetPriceRefreshError, AssetQuote};

use reqwest::Client;
use sqlx::SqlitePool;
use tokio::time::sleep;
use tracing::{info, warn};

pub async fn spawn_asset_price_refresh_task(pool: SqlitePool, config: AssetPriceRefreshConfig) {
    tokio::spawn(async move {
        let client = Client::new();

        match fill_missing_asset_prices(&pool, &client, &config).await {
            Ok(0) => {}
            Ok(updated_count) => info!(updated_count, "startup asset price fill succeeded"),
            Err(error) => warn!(error = %error, "startup asset price fill failed"),
        }

        loop {
            sleep(config.refresh_interval).await;

            info!(
                refresh_interval_seconds = config.refresh_interval.as_secs(),
                "starting asset price refresh"
            );

            match refresh_asset_prices(&pool, &client, &config).await {
                Ok(updated_count) => info!(updated_count, "asset price refresh succeeded"),
                Err(error) => warn!(error = %error, "asset price refresh failed"),
            }
        }
    });
}
