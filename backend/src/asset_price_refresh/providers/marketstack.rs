use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::{fetch_json, normalize_provider_datetime};

#[derive(Debug, Deserialize)]
struct MarketstackEntry {
    close: serde_json::Number,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MarketstackResponse {
    data: Option<Vec<MarketstackEntry>>,
}

pub struct MarketstackProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::StockProvider for MarketstackProvider {
    fn name(&self) -> &'static str {
        "marketstack"
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_marketstack_quote(client, &self.base_url, &self.api_key, symbol).await
    }
}

/// Fetches a quote from the Marketstack `/v1/eod/latest` endpoint.
/// Note: Marketstack does not return currency in this endpoint.
/// Prices are in the local currency of the exchange (defaults to USD).
pub async fn fetch_marketstack_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!("{}/v1/eod/latest", base_url.trim_end_matches('/'));
    let payload = fetch_json::<MarketstackResponse>(
        client
            .get(url)
            .query(&[("symbols", symbol), ("access_key", api_key)]),
    )
    .await?;

    let entry = payload
        .data
        .as_ref()
        .and_then(|d| d.first())
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(format!(
                "asset price refresh failed: provider returned no data for symbol {symbol}"
            ))
        })?;

    let price = AssetUnitPrice::try_from(entry.close.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    let as_of = match entry.date.clone() {
        Some(date) => normalize_provider_datetime(date)?,
        None => crate::current_utc_timestamp()?,
    };

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}
