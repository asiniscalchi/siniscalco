use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::super::AssetPriceRefreshError;

#[derive(Serialize)]
struct OpenFigiRequest {
    #[serde(rename = "idType")]
    id_type: &'static str,
    #[serde(rename = "idValue")]
    id_value: String,
}

#[derive(Deserialize)]
struct OpenFigiMappingResult {
    data: Option<Vec<OpenFigiSecurity>>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct OpenFigiSecurity {
    ticker: Option<String>,
}

pub async fn fetch_openfigi_ticker(
    client: &Client,
    base_url: &str,
    api_key: Option<&str>,
    isin: &str,
) -> Result<String, AssetPriceRefreshError> {
    let url = format!("{}/v3/mapping", base_url.trim_end_matches('/'));
    let body = vec![OpenFigiRequest {
        id_type: "ID_ISIN",
        id_value: isin.to_string(),
    }];

    let mut request = client.post(&url).json(&body);
    if let Some(key) = api_key {
        request = request.header("X-OPENFIGI-APIKEY", key);
    }

    let response = request
        .send()
        .await
        .map_err(|e| AssetPriceRefreshError::Provider(format!("OpenFIGI request failed: {e}")))?;

    if !response.status().is_success() {
        return Err(AssetPriceRefreshError::Provider(format!(
            "OpenFIGI returned status {}",
            response.status()
        )));
    }

    let results = response
        .json::<Vec<OpenFigiMappingResult>>()
        .await
        .map_err(|e| {
            AssetPriceRefreshError::Provider(format!("OpenFIGI response parse failed: {e}"))
        })?;

    let result = results.into_iter().next().ok_or_else(|| {
        AssetPriceRefreshError::Provider("OpenFIGI returned empty response".into())
    })?;

    if let Some(error) = result.error {
        return Err(AssetPriceRefreshError::Provider(format!(
            "OpenFIGI ISIN resolution failed: {error}"
        )));
    }

    result
        .data
        .as_deref()
        .and_then(|securities| securities.first())
        .and_then(|s| s.ticker.clone())
        .ok_or_else(|| {
            AssetPriceRefreshError::Provider(format!("OpenFIGI returned no ticker for ISIN {isin}"))
        })
}
