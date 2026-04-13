use reqwest::Client;
use serde::Deserialize;

use crate::AssetUnitPrice;

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::{fetch_json, unix_timestamp_to_rfc3339};

#[derive(Debug, Deserialize)]
struct PolygonAgg {
    c: serde_json::Number,
    t: i64,
}

#[derive(Debug, Deserialize)]
struct PolygonPrevCloseResponse {
    status: Option<String>,
    results: Option<Vec<PolygonAgg>>,
}

pub struct PolygonProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::QuoteProvider for PolygonProvider {
    fn name(&self) -> &'static str {
        "polygon"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_polygon_quote(client, &self.base_url, &self.api_key, symbol).await
    }
}

/// Fetches a quote from the Polygon.io `/v2/aggs/ticker/{symbol}/prev` endpoint.
/// Returns the previous trading day's close price in USD.
pub async fn fetch_polygon_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!(
        "{}/v2/aggs/ticker/{}/prev",
        base_url.trim_end_matches('/'),
        symbol
    );
    let payload =
        fetch_json::<PolygonPrevCloseResponse>(client.get(url).query(&[("apiKey", api_key)]))
            .await?;

    if payload.status.as_deref() == Some("ERROR") {
        return Err(AssetPriceRefreshError::Provider(
            "asset price refresh failed: provider returned an error".into(),
        ));
    }

    let agg = payload
        .results
        .as_ref()
        .and_then(|r| r.first())
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(format!(
                "asset price refresh failed: provider returned no data for symbol {symbol}"
            ))
        })?;

    let price = AssetUnitPrice::try_from(agg.c.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    // Polygon timestamps are in milliseconds
    let as_of = unix_timestamp_to_rfc3339(agg.t / 1000)?;

    Ok(AssetQuote {
        price,
        currency: super::currency_from_symbol(symbol),
        as_of,
    })
}
