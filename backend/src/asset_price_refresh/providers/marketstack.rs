use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::normalize_provider_datetime;

#[derive(Debug, Deserialize)]
struct MarketstackEntry {
    close: serde_json::Number,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MarketstackResponse {
    data: Option<Vec<MarketstackEntry>>,
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
    let response = client
        .get(url)
        .query(&[("symbols", symbol), ("access_key", api_key)])
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

    let payload = response
        .json::<MarketstackResponse>()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

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
        None => crate::current_utc_timestamp_iso8601()?,
    };

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}
