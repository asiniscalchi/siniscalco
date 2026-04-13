use reqwest::Client;
use serde::Deserialize;

use crate::AssetUnitPrice;

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::{fetch_json, unix_timestamp_to_rfc3339};

#[derive(Debug, Deserialize)]
struct EodhdRealTimeResponse {
    close: Option<serde_json::Number>,
    timestamp: Option<i64>,
}

pub struct EodhdProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::StockProvider for EodhdProvider {
    fn name(&self) -> &'static str {
        "eodhd"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_eodhd_quote(client, &self.base_url, &self.api_key, symbol).await
    }
}

/// Fetches a quote from the EODHD `/api/real-time/{symbol}` endpoint.
/// Note: EODHD does not return currency in this endpoint.
/// Currency is inferred from the exchange suffix in the symbol (e.g. `.MI` → EUR).
/// The symbol should include the exchange suffix (e.g. "AAPL.US").
pub async fn fetch_eodhd_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!(
        "{}/api/real-time/{}",
        base_url.trim_end_matches('/'),
        symbol
    );
    let payload = fetch_json::<EodhdRealTimeResponse>(
        client
            .get(url)
            .query(&[("api_token", api_key), ("fmt", "json")]),
    )
    .await?;

    let price_num = payload.close.ok_or_else(|| {
        AssetPriceRefreshError::Provider(
            "asset price refresh failed: provider response missing close price".into(),
        )
    })?;

    let price = AssetUnitPrice::try_from(price_num.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    let as_of = match payload.timestamp {
        Some(ts) => unix_timestamp_to_rfc3339(ts)?,
        None => crate::current_utc_timestamp()?,
    };

    Ok(AssetQuote {
        price,
        currency: super::currency_from_symbol(symbol),
        as_of,
    })
}
