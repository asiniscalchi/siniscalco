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

    /// CoinCap API base URL for crypto price refresh (fallback)
    #[arg(
        long,
        env = "COINCAP_BASE_URL",
        default_value = "https://rest.coincap.io/v3"
    )]
    pub coincap_base_url: String,

    /// CoinCap API key (enables CoinCap as the crypto price fallback provider)
    #[arg(long, env = "COINCAP_API_KEY")]
    pub coincap_api_key: Option<String>,

    /// OpenFIGI API base URL for ISIN-to-ticker resolution
    #[arg(
        long,
        env = "OPENFIGI_BASE_URL",
        default_value = "https://api.openfigi.com"
    )]
    pub openfigi_base_url: String,

    /// OpenFIGI API key (optional, increases rate limits)
    #[arg(long, env = "OPENFIGI_API_KEY")]
    pub openfigi_api_key: Option<String>,

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

    /// Polygon.io API base URL
    #[arg(
        long,
        env = "POLYGON_BASE_URL",
        default_value = "https://api.polygon.io"
    )]
    pub polygon_base_url: String,

    /// Polygon.io API key (enables Polygon.io as the stock price provider)
    #[arg(long, env = "POLYGON_API_KEY")]
    pub polygon_api_key: Option<String>,

    /// Financial Modeling Prep API base URL
    #[arg(
        long,
        env = "FMP_BASE_URL",
        default_value = "https://financialmodelingprep.com"
    )]
    pub fmp_base_url: String,

    /// Financial Modeling Prep API key (enables FMP as the stock price provider)
    #[arg(long, env = "FMP_API_KEY")]
    pub fmp_api_key: Option<String>,

    /// EODHD API base URL
    #[arg(long, env = "EODHD_BASE_URL", default_value = "https://eodhd.com")]
    pub eodhd_base_url: String,

    /// EODHD API key (enables EODHD as the stock price provider)
    #[arg(long, env = "EODHD_API_KEY")]
    pub eodhd_api_key: Option<String>,

    /// Tiingo API base URL
    #[arg(
        long,
        env = "TIINGO_BASE_URL",
        default_value = "https://api.tiingo.com"
    )]
    pub tiingo_base_url: String,

    /// Tiingo API key (enables Tiingo as the stock price provider)
    #[arg(long, env = "TIINGO_API_KEY")]
    pub tiingo_api_key: Option<String>,

    /// Marketstack API base URL
    #[arg(
        long,
        env = "MARKETSTACK_BASE_URL",
        default_value = "https://api.marketstack.com"
    )]
    pub marketstack_base_url: String,

    /// Marketstack API key (enables Marketstack as the stock price provider)
    #[arg(long, env = "MARKETSTACK_API_KEY")]
    pub marketstack_api_key: Option<String>,
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
            coincap_base_url: trim_url(&self.coincap_base_url),
            coincap_api_key: non_empty(self.coincap_api_key.as_deref()),
            openfigi_base_url: trim_url(&self.openfigi_base_url),
            openfigi_api_key: non_empty(self.openfigi_api_key.as_deref()),
            twelve_data_base_url: trim_url(&self.twelve_data_base_url),
            twelve_data_api_key: non_empty(self.twelve_data_api_key.as_deref()),
            finnhub_base_url: trim_url(&self.finnhub_base_url),
            finnhub_api_key: non_empty(self.finnhub_api_key.as_deref()),
            alpha_vantage_base_url: trim_url(&self.alpha_vantage_base_url),
            alpha_vantage_api_key: non_empty(self.alpha_vantage_api_key.as_deref()),
            polygon_base_url: trim_url(&self.polygon_base_url),
            polygon_api_key: non_empty(self.polygon_api_key.as_deref()),
            fmp_base_url: trim_url(&self.fmp_base_url),
            fmp_api_key: non_empty(self.fmp_api_key.as_deref()),
            eodhd_base_url: trim_url(&self.eodhd_base_url),
            eodhd_api_key: non_empty(self.eodhd_api_key.as_deref()),
            tiingo_base_url: trim_url(&self.tiingo_base_url),
            tiingo_api_key: non_empty(self.tiingo_api_key.as_deref()),
            marketstack_base_url: trim_url(&self.marketstack_base_url),
            marketstack_api_key: non_empty(self.marketstack_api_key.as_deref()),
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
