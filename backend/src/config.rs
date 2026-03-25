use std::time::Duration;

use clap::Parser;

use crate::{AssetPriceRefreshConfig, FxRefreshConfig};

const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 60 * 60;

#[derive(Parser, Debug)]
#[command(about = "Siniscalco portfolio backend")]
pub struct Config {
    /// Port to listen on
    #[arg(long, env = "PORT", default_value_t = 3000)]
    pub port: u16,

    /// Path to the SQLite database file
    #[arg(long, env = "DB_PATH", default_value = "data/app.db")]
    pub db_path: String,

    /// Frankfurter API base URL for FX rate refresh
    #[arg(
        long,
        env = "FX_REFRESH_BASE_URL",
        default_value = "https://api.frankfurter.dev/v1"
    )]
    pub fx_refresh_base_url: String,

    /// CoinGecko API base URL for crypto price refresh
    #[arg(
        long,
        env = "COINGECKO_BASE_URL",
        default_value = "https://api.coingecko.com/api/v3"
    )]
    pub coingecko_base_url: String,

    /// Twelve Data API base URL
    #[arg(
        long,
        env = "TWELVE_DATA_BASE_URL",
        default_value = "https://api.twelvedata.com"
    )]
    pub twelve_data_base_url: String,

    /// Twelve Data API key (enables Twelve Data as the stock price provider)
    #[arg(long, env = "TWELVE_DATA_API_KEY")]
    pub twelve_data_api_key: Option<String>,

    /// Finnhub API base URL
    #[arg(long, env = "FINNHUB_BASE_URL", default_value = "https://finnhub.io")]
    pub finnhub_base_url: String,

    /// Finnhub API key (enables Finnhub as the stock price provider)
    #[arg(long, env = "FINNHUB_API_KEY")]
    pub finnhub_api_key: Option<String>,

    /// Alpha Vantage API base URL
    #[arg(
        long,
        env = "ALPHA_VANTAGE_BASE_URL",
        default_value = "https://www.alphavantage.co"
    )]
    pub alpha_vantage_base_url: String,

    /// Alpha Vantage API key (enables Alpha Vantage as the stock price provider)
    #[arg(long, env = "ALPHA_VANTAGE_API_KEY")]
    pub alpha_vantage_api_key: Option<String>,
}

impl Config {
    pub fn fx_refresh_config(&self) -> FxRefreshConfig {
        FxRefreshConfig {
            refresh_interval: Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS),
            base_url: trim_url(&self.fx_refresh_base_url),
        }
    }

    pub fn asset_price_refresh_config(&self) -> AssetPriceRefreshConfig {
        AssetPriceRefreshConfig {
            refresh_interval: Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS),
            coingecko_base_url: trim_url(&self.coingecko_base_url),
            twelve_data_base_url: trim_url(&self.twelve_data_base_url),
            twelve_data_api_key: non_empty(self.twelve_data_api_key.as_deref()),
            finnhub_base_url: trim_url(&self.finnhub_base_url),
            finnhub_api_key: non_empty(self.finnhub_api_key.as_deref()),
            alpha_vantage_base_url: trim_url(&self.alpha_vantage_base_url),
            alpha_vantage_api_key: non_empty(self.alpha_vantage_api_key.as_deref()),
        }
    }
}

fn trim_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}
