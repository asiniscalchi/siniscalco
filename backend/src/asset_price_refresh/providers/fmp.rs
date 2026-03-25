use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::unix_timestamp_to_rfc3339;

#[derive(Debug, Deserialize)]
struct FmpQuote {
    price: serde_json::Number,
    timestamp: Option<i64>,
}

/// Fetches a quote from the Financial Modeling Prep `/stable/quote` endpoint.
/// Note: FMP does not return the currency in this endpoint.
/// Prices are returned in the local currency of the exchange (defaults to USD).
pub async fn fetch_fmp_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!("{}/stable/quote", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .query(&[("symbol", symbol), ("apikey", api_key)])
        .send()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    if !response.status().is_success() {
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned status {}",
            response.status()
        )));
    }

    let payload = response.json::<Vec<FmpQuote>>().await.map_err(|error| {
        AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
    })?;

    let quote = payload.first().ok_or_else(|| {
        AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned no data for symbol {symbol}"
        ))
    })?;

    let price = AssetUnitPrice::try_from(quote.price.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    let as_of = match quote.timestamp {
        Some(ts) => unix_timestamp_to_rfc3339(ts)?,
        None => crate::current_utc_timestamp_iso8601()?,
    };

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}
