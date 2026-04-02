use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::{fetch_json, unix_timestamp_to_rfc3339};

#[derive(Debug, Deserialize)]
struct FinnhubQuoteResponse {
    c: Option<serde_json::Number>,
    t: Option<i64>,
    error: Option<String>,
}

pub struct FinnhubProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::StockProvider for FinnhubProvider {
    fn name(&self) -> &'static str {
        "finnhub"
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_finnhub_quote(client, &self.base_url, &self.api_key, symbol).await
    }
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
    let payload = fetch_json::<FinnhubQuoteResponse>(
        client
            .get(url)
            .query(&[("symbol", symbol), ("token", api_key)]),
    )
    .await?;

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

    let as_of = unix_timestamp_to_rfc3339(timestamp)?;

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}
