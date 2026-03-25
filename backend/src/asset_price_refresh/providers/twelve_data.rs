use reqwest::Client;
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency, current_utc_timestamp_iso8601};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::normalize_provider_datetime;

#[derive(Debug, Deserialize)]
struct TwelveDataQuoteResponse {
    close: Option<String>,
    currency: Option<String>,
    datetime: Option<String>,
    code: Option<i64>,
    message: Option<String>,
    status: Option<String>,
}

pub struct TwelveDataProvider {
    pub base_url: String,
    pub api_key: String,
}

#[async_trait::async_trait]
impl super::StockProvider for TwelveDataProvider {
    fn name(&self) -> &'static str {
        "twelve_data"
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_twelve_data_quote(client, &self.base_url, &self.api_key, symbol).await
    }
}

pub async fn fetch_twelve_data_quote(
    client: &Client,
    base_url: &str,
    api_key: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!("{}/quote", base_url.trim_end_matches('/'));
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

    let payload = response
        .json::<TwelveDataQuoteResponse>()
        .await
        .map_err(|error| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {error}"))
        })?;

    if payload.status.as_deref() == Some("error") || payload.code.is_some() {
        return Err(AssetPriceRefreshError::Provider(
            payload
                .message
                .unwrap_or_else(|| "asset price refresh failed: provider returned an error".into()),
        ));
    }

    let price = payload
        .close
        .as_deref()
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: provider response missing close price".into(),
            )
        })
        .and_then(|price| AssetUnitPrice::try_from(price).map_err(AssetPriceRefreshError::from))?;

    let currency = payload
        .currency
        .as_deref()
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(
                "asset price refresh failed: provider response missing currency".into(),
            )
        })
        .and_then(|currency| Currency::try_from(currency).map_err(AssetPriceRefreshError::from))?;

    let as_of = match payload.datetime {
        Some(datetime) => normalize_provider_datetime(datetime)?,
        None => current_utc_timestamp_iso8601()?,
    };

    Ok(AssetQuote {
        price,
        currency,
        as_of,
    })
}
