use reqwest::Client;
use serde::Deserialize;

use crate::AssetUnitPrice;

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::{fetch_json, unix_timestamp_to_rfc3339};

#[derive(Debug, Deserialize)]
struct FcsApiActive {
    c: serde_json::Number,
    t: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct FcsApiEntry {
    active: FcsApiActive,
}

#[derive(Debug, Deserialize)]
struct FcsApiResponse {
    status: bool,
    response: Option<Vec<FcsApiEntry>>,
    msg: Option<String>,
}

pub struct FcsApiProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::StockProvider for FcsApiProvider {
    fn name(&self) -> &'static str {
        "fcsapi"
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_fcsapi_quote(client, &self.base_url, &self.api_key, symbol).await
    }
}

/// Fetches a quote from the FCS API `/stock/latest` endpoint.
/// Note: FCS API does not return currency in this endpoint.
/// Currency is inferred from the exchange suffix in the symbol.
pub async fn fetch_fcsapi_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!("{}/stock/latest", base_url.trim_end_matches('/'));
    let payload = fetch_json::<FcsApiResponse>(
        client
            .get(url)
            .query(&[("symbol", symbol), ("access_key", api_key)]),
    )
    .await?;

    if !payload.status {
        let msg = payload.msg.unwrap_or_default();
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: {msg}"
        )));
    }

    let entry = payload
        .response
        .as_ref()
        .and_then(|r| r.first())
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(format!(
                "asset price refresh failed: provider returned no data for symbol {symbol}"
            ))
        })?;

    let price = AssetUnitPrice::try_from(entry.active.c.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    let as_of = match entry.active.t {
        Some(ts) if ts > 0 => unix_timestamp_to_rfc3339(ts)?,
        _ => crate::current_utc_timestamp()?,
    };

    Ok(AssetQuote {
        price,
        currency: super::currency_from_symbol(symbol),
        as_of,
    })
}
