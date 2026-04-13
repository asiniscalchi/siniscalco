use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency, current_utc_timestamp};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::{QuoteProvider, fetch_json, unix_timestamp_to_rfc3339};

#[derive(Debug, Deserialize)]
struct CoinGeckoPrice {
    usd: serde_json::Number,
    last_updated_at: Option<i64>,
}

pub struct CoinGeckoProvider {
    pub base_url: String,
}

#[async_trait::async_trait]
impl QuoteProvider for CoinGeckoProvider {
    fn name(&self) -> &'static str {
        "coingecko"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_coingecko_quote(client, &self.base_url, symbol).await
    }
}

pub async fn fetch_coingecko_quote(
    client: &Client,
    base_url: &str,
    coin_id: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!("{}/simple/price", base_url.trim_end_matches('/'));
    let payload =
        fetch_json::<std::collections::BTreeMap<String, CoinGeckoPrice>>(client.get(url).query(&[
            ("ids", coin_id),
            ("vs_currencies", "usd"),
            ("include_last_updated_at", "true"),
        ]))
        .await?;

    let coin_data = payload.get(coin_id).ok_or_else(|| {
        AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned no data for coin {coin_id}"
        ))
    })?;

    let price = AssetUnitPrice::try_from(coin_data.usd.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    let as_of = match coin_data.last_updated_at {
        Some(ts) => unix_timestamp_to_rfc3339(ts)?,
        None => current_utc_timestamp()?,
    };

    Ok(AssetQuote {
        price,
        currency: Currency::Usd,
        as_of,
    })
}
