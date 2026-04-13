use reqwest::Client;
use serde::Deserialize;

use crate::AssetUnitPrice;

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::{fetch_json, unix_timestamp_to_rfc3339};

#[derive(Debug, Deserialize)]
struct FmpQuote {
    price: serde_json::Number,
    timestamp: Option<i64>,
}

pub struct FmpProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::QuoteProvider for FmpProvider {
    fn name(&self) -> &'static str {
        "fmp"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_fmp_quote(client, &self.base_url, &self.api_key, symbol).await
    }
}

/// Fetches a quote from the Financial Modeling Prep `/stable/quote` endpoint.
/// Note: FMP does not return the currency in this endpoint.
/// Currency is inferred from the exchange suffix in the symbol (e.g. `.MI` → EUR).
pub async fn fetch_fmp_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!("{}/stable/quote", base_url.trim_end_matches('/'));
    let payload = fetch_json::<Vec<FmpQuote>>(
        client
            .get(url)
            .query(&[("symbol", symbol), ("apikey", api_key)]),
    )
    .await?;

    let quote = payload.first().ok_or_else(|| {
        AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned no data for symbol {symbol}"
        ))
    })?;

    let price = AssetUnitPrice::try_from(quote.price.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    let as_of = match quote.timestamp {
        Some(ts) => unix_timestamp_to_rfc3339(ts)?,
        None => crate::current_utc_timestamp()?,
    };

    Ok(AssetQuote {
        price,
        currency: super::currency_from_symbol(symbol),
        as_of,
    })
}
