use reqwest::Client;
use sqlx::SqlitePool;
use tracing::{info, warn};

use crate::{
    AssetId, AssetRecord, AssetType, UpsertAssetPriceInput, get_asset, list_assets,
    upsert_asset_price,
};

use super::config::AssetPriceRefreshConfig;
use super::providers::{fetch_coincap_quote, fetch_coingecko_quote, fetch_openfigi_tickers};
use super::types::{AssetPriceRefreshError, AssetQuote};

pub async fn refresh_asset_prices(
    pool: &SqlitePool,
    client: &Client,
    config: &AssetPriceRefreshConfig,
) -> Result<usize, AssetPriceRefreshError> {
    let assets = list_assets(pool).await?;
    let mut updated_count = 0usize;

    for asset in assets {
        match refresh_single_asset_price(pool, client, config, asset.id).await {
            Ok(true) => {
                updated_count += 1;
            }
            Ok(false) => {}
            Err(error) => {
                warn!(
                    asset_id = asset.id.as_i64(),
                    error = %error,
                    "asset price refresh failed for stored asset"
                );
            }
        }
    }

    Ok(updated_count)
}

pub async fn fill_missing_asset_prices(
    pool: &SqlitePool,
    client: &Client,
    config: &AssetPriceRefreshConfig,
) -> Result<usize, AssetPriceRefreshError> {
    let assets = list_assets(pool).await?;
    let mut updated_count = 0usize;

    for asset in assets.into_iter().filter(|a| a.current_price.is_none()) {
        match refresh_single_asset_price(pool, client, config, asset.id).await {
            Ok(true) => updated_count += 1,
            Ok(false) => {}
            Err(error) => {
                warn!(
                    asset_id = asset.id.as_i64(),
                    symbol = %asset.symbol,
                    error = %error,
                    "startup asset price fill failed for asset"
                );
            }
        }
    }

    Ok(updated_count)
}

pub async fn refresh_single_asset_price(
    pool: &SqlitePool,
    client: &Client,
    config: &AssetPriceRefreshConfig,
    asset_id: AssetId,
) -> Result<bool, AssetPriceRefreshError> {
    let asset = get_asset(pool, asset_id).await?;

    let quote = if asset.asset_type == AssetType::Crypto {
        fetch_crypto_quote(client, config, &asset).await?
    } else {
        match fetch_stock_quote(client, config, &asset).await? {
            Some(quote) => quote,
            None => return Ok(false),
        }
    };

    upsert_asset_price(
        pool,
        UpsertAssetPriceInput {
            asset_id,
            price: quote.price,
            currency: quote.currency,
            as_of: quote.as_of,
        },
    )
    .await?;

    Ok(true)
}

async fn fetch_crypto_quote(
    client: &Client,
    config: &AssetPriceRefreshConfig,
    asset: &AssetRecord,
) -> Result<AssetQuote, AssetPriceRefreshError> {
    let coin_id = asset
        .quote_symbol
        .as_deref()
        .unwrap_or(asset.symbol.as_str())
        .to_lowercase();

    match fetch_coingecko_quote(client, &config.coingecko_base_url, &coin_id).await {
        Ok(quote) => Ok(quote),
        Err(coingecko_err) => {
            if let Some(api_key) = config.coincap_api_key.as_deref() {
                warn!(
                    coin_id,
                    error = %coingecko_err,
                    "CoinGecko failed, falling back to CoinCap"
                );
                Ok(
                    fetch_coincap_quote(client, &config.coincap_base_url, api_key, &coin_id)
                        .await?,
                )
            } else {
                Err(coingecko_err)
            }
        }
    }
}

async fn fetch_stock_quote(
    client: &Client,
    config: &AssetPriceRefreshConfig,
    asset: &AssetRecord,
) -> Result<Option<AssetQuote>, AssetPriceRefreshError> {
    let symbols = resolve_stock_symbols(client, config, asset).await;
    let providers = config.stock_providers();
    Ok(try_stock_providers(client, &providers, &symbols)
        .await
        .transpose()?)
}

async fn resolve_stock_symbols(
    client: &Client,
    config: &AssetPriceRefreshConfig,
    asset: &AssetRecord,
) -> Vec<String> {
    if let Some(quote_symbol) = asset.quote_symbol.as_deref() {
        return vec![quote_symbol.to_string()];
    }
    if let Some(isin) = asset.isin.as_deref() {
        match fetch_openfigi_tickers(
            client,
            &config.openfigi_base_url,
            config.openfigi_api_key.as_deref(),
            isin,
        )
        .await
        {
            Ok(tickers) => return tickers,
            Err(e) => {
                warn!(isin, error = %e, "OpenFIGI ISIN resolution failed, falling back to asset symbol");
            }
        }
    }
    vec![asset.symbol.as_str().to_string()]
}

async fn try_stock_providers(
    client: &Client,
    providers: &[Box<dyn super::providers::StockProvider>],
    symbols: &[String],
) -> Option<Result<AssetQuote, AssetPriceRefreshError>> {
    let mut last_err = None;

    for symbol in symbols {
        for provider in providers {
            match provider.fetch_quote(client, symbol).await {
                Ok(quote) => {
                    info!(provider = provider.name(), symbol, "asset price fetched");
                    return Some(Ok(quote));
                }
                Err(e) => {
                    warn!(provider = provider.name(), symbol, error = %e, "provider failed, trying next");
                    last_err = Some(e);
                }
            }
        }
    }

    last_err.map(Err)
}
