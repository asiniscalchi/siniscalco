use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;

use crate::AssetUnitPrice;

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::unix_timestamp_to_rfc3339;

#[derive(Debug, Deserialize)]
struct ITickQuote {
    ld: Option<serde_json::Number>,
    t: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ITickResponse {
    code: i32,
    msg: Option<String>,
    data: Option<HashMap<String, ITickQuote>>,
}

pub struct ITickProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::StockProvider for ITickProvider {
    fn name(&self) -> &'static str {
        "itick"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_itick_quote(client, &self.base_url, &self.api_key, symbol).await
    }
}

/// Maps our exchange suffix to an iTick region code, and returns
/// the bare ticker code (without the suffix).
fn split_symbol(symbol: &str) -> (&str, &str) {
    let (base, suffix) = match symbol.rsplit_once('.') {
        Some((b, s)) => (b, s),
        None => return ("US", symbol),
    };
    let region = match suffix {
        "MI" => "IT",
        "AS" => "NL",
        "PA" => "FR",
        "DE" | "F" => "DE",
        "L" | "IL" => "GB",
        "SW" | "VX" => "CH",
        "HK" => "HK",
        "T" => "JP",
        "SI" => "SG",
        "TW" => "TW",
        "AX" => "AU",
        "SA" => "BR",
        "TO" | "V" => "CA",
        "MX" => "MX",
        "NS" | "BO" => "IN",
        _ => "US",
    };
    (region, base)
}

/// Fetches a quote from the iTick `/stock/quotes` endpoint.
/// The symbol is split into a region code and bare ticker.
/// Currency is inferred from the exchange suffix in the original symbol.
pub async fn fetch_itick_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let (region, code) = split_symbol(symbol);
    let url = format!("{}/stock/quotes", base_url.trim_end_matches('/'));

    let response = client
        .get(url)
        .header("accept", "application/json")
        .header("token", api_key)
        .query(&[("region", region), ("codes", code)])
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

    let payload: ITickResponse = response.json().await.map_err(|error| {
        AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
    })?;

    if payload.code != 0 {
        let msg = payload.msg.unwrap_or_default();
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: {msg}"
        )));
    }

    let quote = payload
        .data
        .as_ref()
        .and_then(|d| d.values().next())
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(format!(
                "asset price refresh failed: provider returned no data for symbol {symbol}"
            ))
        })?;

    let price_str = quote.ld.as_ref().map(|n| n.to_string()).ok_or_else(|| {
        AssetPriceRefreshError::Provider(
            "asset price refresh failed: provider response missing price".into(),
        )
    })?;

    let price =
        AssetUnitPrice::try_from(price_str.as_str()).map_err(AssetPriceRefreshError::from)?;

    // iTick returns timestamps in milliseconds
    let as_of = match quote.t {
        Some(ts) if ts > 0 => unix_timestamp_to_rfc3339(ts / 1000)?,
        _ => crate::current_utc_timestamp()?,
    };

    Ok(AssetQuote {
        price,
        currency: super::currency_from_symbol(symbol),
        as_of,
    })
}
