use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency, current_utc_timestamp};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::{fetch_json, normalize_provider_datetime};

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

pub struct AlphaVantageProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::StockProvider for AlphaVantageProvider {
    fn name(&self) -> &'static str {
        "alpha_vantage"
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_alpha_vantage_quote(client, &self.base_url, &self.api_key, symbol).await
    }
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
    let payload = fetch_json::<AlphaVantageQuoteResponse>(client.get(url).query(&[
        ("function", "GLOBAL_QUOTE"),
        ("symbol", symbol),
        ("apikey", api_key),
    ]))
    .await?;

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
        .unwrap_or_else(|| current_utc_timestamp().unwrap_or_default());

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}
