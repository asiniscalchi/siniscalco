use std::time::Duration;

use super::providers::{
    AlphaVantageProvider, EodhdProvider, FcsApiProvider, FinnhubProvider, FmpProvider,
    ITickProvider, MarketstackProvider, PolygonProvider, StockProvider, TiingoProvider,
    TwelveDataProvider, YahooFinanceProvider,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetPriceRefreshConfig {
    pub refresh_interval: Duration,
    pub coingecko_base_url: String,
    pub coincap_base_url: String,
    pub coincap_api_key: Option<String>,
    pub openfigi_base_url: String,
    pub openfigi_api_key: Option<String>,
    pub yahoo_finance_base_url: String,
    pub yahoo_finance_enabled: bool,
    pub twelve_data_base_url: String,
    pub twelve_data_api_key: Option<String>,
    pub finnhub_base_url: String,
    pub finnhub_api_key: Option<String>,
    pub alpha_vantage_base_url: String,
    pub alpha_vantage_api_key: Option<String>,
    pub polygon_base_url: String,
    pub polygon_api_key: Option<String>,
    pub fmp_base_url: String,
    pub fmp_api_key: Option<String>,
    pub eodhd_base_url: String,
    pub eodhd_api_key: Option<String>,
    pub tiingo_base_url: String,
    pub tiingo_api_key: Option<String>,
    pub marketstack_base_url: String,
    pub marketstack_api_key: Option<String>,
    pub fcsapi_base_url: String,
    pub fcsapi_api_key: Option<String>,
    pub itick_base_url: String,
    pub itick_api_key: Option<String>,
}

impl AssetPriceRefreshConfig {
    pub fn stock_providers(&self) -> Vec<Box<dyn StockProvider>> {
        let mut providers: Vec<Box<dyn StockProvider>> = Vec::new();
        if self.yahoo_finance_enabled {
            providers.push(Box::new(YahooFinanceProvider {
                base_url: self.yahoo_finance_base_url.clone(),
            }));
        }
        if let Some(ref key) = self.twelve_data_api_key {
            providers.push(Box::new(TwelveDataProvider {
                base_url: self.twelve_data_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        if let Some(ref key) = self.finnhub_api_key {
            providers.push(Box::new(FinnhubProvider {
                base_url: self.finnhub_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        if let Some(ref key) = self.alpha_vantage_api_key {
            providers.push(Box::new(AlphaVantageProvider {
                base_url: self.alpha_vantage_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        if let Some(ref key) = self.polygon_api_key {
            providers.push(Box::new(PolygonProvider {
                base_url: self.polygon_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        if let Some(ref key) = self.fmp_api_key {
            providers.push(Box::new(FmpProvider {
                base_url: self.fmp_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        if let Some(ref key) = self.eodhd_api_key {
            providers.push(Box::new(EodhdProvider {
                base_url: self.eodhd_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        if let Some(ref key) = self.tiingo_api_key {
            providers.push(Box::new(TiingoProvider {
                base_url: self.tiingo_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        if let Some(ref key) = self.marketstack_api_key {
            providers.push(Box::new(MarketstackProvider {
                base_url: self.marketstack_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        if let Some(ref key) = self.fcsapi_api_key {
            providers.push(Box::new(FcsApiProvider {
                base_url: self.fcsapi_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        if let Some(ref key) = self.itick_api_key {
            providers.push(Box::new(ITickProvider {
                base_url: self.itick_base_url.clone(),
                api_key: key.clone(),
            }));
        }
        providers
    }
}
