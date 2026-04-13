use reqwest::{Client, header::USER_AGENT};
use serde::Deserialize;

use crate::{AssetUnitPrice, Currency};

use super::super::{AssetPriceRefreshError, AssetQuote};
use super::unix_timestamp_to_rfc3339;

const YAHOO_FINANCE_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug, Deserialize)]
struct YahooChartResponse {
    chart: YahooChart,
}

#[derive(Debug, Deserialize)]
struct YahooChart {
    result: Option<Vec<YahooChartResult>>,
    error: Option<YahooError>,
}

#[derive(Debug, Deserialize)]
struct YahooError {
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YahooChartResult {
    meta: YahooMeta,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YahooMeta {
    regular_market_price: Option<f64>,
    regular_market_time: Option<i64>,
    currency: Option<String>,
}

pub struct YahooFinanceProvider {
    pub base_url: String,
}

#[async_trait::async_trait]
impl super::StockProvider for YahooFinanceProvider {
    fn name(&self) -> &'static str {
        "yahoo"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn fetch_quote(
        &self,
        client: &Client,
        symbol: &str,
    ) -> Result<AssetQuote, AssetPriceRefreshError> {
        fetch_yahoo_quote(client, &self.base_url, symbol).await
    }
}

/// Fetches a quote from the Yahoo Finance `/v8/finance/chart/{symbol}` endpoint.
/// Yahoo Finance returns the currency of the exchange, unlike most other providers.
/// No API key is required.
pub async fn fetch_yahoo_quote(
    client: &Client,
    base_url: &str,
    symbol: &str,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let url = format!(
        "{}/v8/finance/chart/{}",
        base_url.trim_end_matches('/'),
        symbol
    );

    let response = client
        .get(url)
        .header(USER_AGENT, YAHOO_FINANCE_USER_AGENT)
        .query(&[("range", "1d"), ("interval", "1d")])
        .send()
        .await
        .map_err(|e| {
            AssetPriceRefreshError::Provider(format!("asset price refresh failed: {e}"))
        })?;

    if !response.status().is_success() {
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: provider returned status {}",
            response.status()
        )));
    }

    let payload = response.json::<YahooChartResponse>().await.map_err(|e| {
        AssetPriceRefreshError::Provider(format!("asset price refresh failed: {e}"))
    })?;

    if let Some(error) = payload.chart.error {
        let desc = error.description.unwrap_or_default();
        return Err(AssetPriceRefreshError::Provider(format!(
            "asset price refresh failed: {desc}"
        )));
    }

    let result = payload
        .chart
        .result
        .and_then(|r| r.into_iter().next())
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(format!(
                "asset price refresh failed: provider returned no data for symbol {symbol}"
            ))
        })?;

    let price_f64 = result.meta.regular_market_price.ok_or_else(|| {
        AssetPriceRefreshError::Provider(
            "asset price refresh failed: provider response missing price".into(),
        )
    })?;

    let price = AssetUnitPrice::try_from(price_f64.to_string().as_str())
        .map_err(AssetPriceRefreshError::from)?;

    let timestamp = result.meta.regular_market_time.ok_or_else(|| {
        AssetPriceRefreshError::Provider(
            "asset price refresh failed: provider response missing timestamp".into(),
        )
    })?;

    let as_of = unix_timestamp_to_rfc3339(timestamp)?;

    let currency = result
        .meta
        .currency
        .as_deref()
        .and_then(|c| Currency::try_from(c).ok())
        .unwrap_or(Currency::Usd);

    Ok(AssetQuote {
        price,
        currency,
        as_of,
    })
}
