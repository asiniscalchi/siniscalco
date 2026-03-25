use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency, current_utc_timestamp_iso8601};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::normalize_provider_datetime;

#[derive(Debug, Deserialize)]
struct TiingoPriceEntry {
    close: serde_json::Number,
    date: Option<String>,
}

/// Fetches a quote from the Tiingo `/tiingo/daily/{symbol}/prices` endpoint.
/// API token is sent via the Authorization header.
pub async fn fetch_tiingo_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!(
        "{}/tiingo/daily/{}/prices",
        base_url.trim_end_matches('/'),
        symbol
    );
    let response = client
        .get(url)
        .header("Authorization", format!("Token {api_key}"))
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
        .json::<Vec<TiingoPriceEntry>>()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    let entry = payload.first().ok_or_else(|| {
        AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned no data for symbol {symbol}"
        ))
    })?;

    let price = AssetUnitPrice::try_from(entry.close.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    let as_of = match entry.date.clone() {
        Some(date) => normalize_provider_datetime(date)?,
        None => current_utc_timestamp_iso8601()?,
    };

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}
