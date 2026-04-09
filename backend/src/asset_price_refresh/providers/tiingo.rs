use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, current_utc_timestamp};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::{fetch_json, normalize_provider_datetime};

#[derive(Debug, Deserialize)]
struct TiingoPriceEntry {
    close: serde_json::Number,
    date: Option<String>,
}

pub struct TiingoProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::StockProvider for TiingoProvider {
    fn name(&self) -> &'static str {
        "tiingo"
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_tiingo_quote(client, &self.base_url, &self.api_key, symbol).await
    }
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
    let payload = fetch_json::<Vec<TiingoPriceEntry>>(
        client
            .get(url)
            .header("Authorization", format!("Token {api_key}")),
    )
    .await?;

    let entry = payload.first().ok_or_else(|| {
        AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned no data for symbol {symbol}"
        ))
    })?;

    let price = AssetUnitPrice::try_from(entry.close.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    let as_of = match entry.date.clone() {
        Some(date) => normalize_provider_datetime(date)?,
        None => current_utc_timestamp()?,
    };

    Ok(AssetQuote {
        price,
        currency: super::currency_from_symbol(symbol),
        as_of,
    })
}
