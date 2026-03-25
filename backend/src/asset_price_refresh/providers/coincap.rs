use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::unix_timestamp_to_rfc3339;

#[derive(Debug, Deserialize)]
struct CoinCapPriceResponse {
    timestamp: i64,
    data: Vec<Option<String>>,
}

pub async fn fetch_coincap_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    coin_id: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let symbol = coin_id.to_uppercase();
    let url = format!(
        "{}/price/bysymbol/{}",
        base_url.trim_end_matches('/'),
        symbol
    );
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
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
        .json::<CoinCapPriceResponse>()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    let price_str = payload.data.into_iter().next().flatten().ok_or_else(|| {
        AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned no data for coin {coin_id}"
        ))
    })?;

    let price_str = truncate_decimals(&price_str, 6);
    let price =
        AssetUnitPrice::try_from(price_str.as_str()).map_err(AssetPriceRefreshError::from)?;

    // CoinCap v3 returns timestamp in milliseconds
    let as_of = unix_timestamp_to_rfc3339(payload.timestamp / 1000)?;

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}

fn truncate_decimals(value: &str, max_decimals: usize) -> String {
    match value.split_once('.') {
        Some((integer, fractional)) => {
            let truncated = &fractional[..fractional.len().min(max_decimals)];
            if truncated.is_empty() {
                integer.to_string()
            } else {
                format!("{integer}.{truncated}")
            }
        }
        None => value.to_string(),
    }
}
