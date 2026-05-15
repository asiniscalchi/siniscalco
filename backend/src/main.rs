use std::{collections::BTreeSet, net::SocketAddr, path::PathBuf};

use clap::Parser;
use if_addrs::get_if_addrs;
use tracing::{error, info, warn};

use backend::{
    AppState, AssetPriceRefreshConfig, Config, FxRefreshConfig, build_router_with_state,
    connect_db_file, init_tracing, new_shared_fx_refresh_status, spawn_asset_price_refresh_task,
    spawn_fx_refresh_task, spawn_portfolio_snapshot_task,
};

#[tokio::main]
async fn main() {
    if let Err(error) = init_tracing() {
        eprintln!("failed to initialize tracing subscriber: {error}");
        std::process::exit(1);
    }

    if let Err(error) = dotenvy::dotenv()
        && !error.not_found()
    {
        eprintln!("failed to load .env file: {error}");
        std::process::exit(1);
    }

    let config = Config::parse();

    let pool = connect_db_file(&config.db_path)
        .await
        .unwrap_or_else(|error| {
            error!(error = %error, "failed to initialize backend database");
            std::process::exit(1);
        });

    let fx_refresh_status = new_shared_fx_refresh_status();
    let fx_refresh_config = config.fx_refresh_config();
    let asset_price_refresh_config = config.asset_price_refresh_config();
    let http_client = reqwest::Client::new();

    let web_dir = resolve_web_dir(&config.web_dir);
    let app = build_router_with_state(AppState {
        pool: pool.clone(),
        fx_refresh_status: fx_refresh_status.clone(),
        asset_price_refresh_config: asset_price_refresh_config.clone(),
        http_client: http_client.clone(),
        config_markdown: config.to_markdown(),
        web_dir,
    });
    let address = SocketAddr::from(([0, 0, 0, 0], config.port));

    log_fx_refresh_configuration(&fx_refresh_config);
    log_asset_price_refresh_configuration(&asset_price_refresh_config);
    info!(endpoint = "/mcp", "MCP server enabled");

    spawn_fx_refresh_task(pool.clone(), fx_refresh_status, fx_refresh_config).await;
    spawn_asset_price_refresh_task(pool.clone(), asset_price_refresh_config).await;
    spawn_portfolio_snapshot_task(pool.clone()).await;

    log_listening_addresses(address);

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .unwrap_or_else(|error| {
            error!(error = %error, "failed to bind backend server");
            std::process::exit(1);
        });

    if let Err(error) = axum::serve(listener, app).await {
        error!(error = %error, "backend server error");
        std::process::exit(1);
    }
}

fn resolve_web_dir(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = PathBuf::from(trimmed);
    if !path.is_dir() {
        warn!(
            web_dir = %path.display(),
            "configured WEB_DIR does not exist; frontend will not be served"
        );
        return None;
    }
    Some(path)
}

fn log_listening_addresses(address: SocketAddr) {
    for url in listening_urls(address) {
        info!(url = %url, "backend listening");
    }
}

fn listening_urls(address: SocketAddr) -> Vec<String> {
    if address.ip().is_unspecified() {
        let mut urls = BTreeSet::new();

        urls.insert(format!("http://127.0.0.1:{}", address.port()));

        if let Ok(interface_addresses) = get_if_addrs() {
            for interface in interface_addresses {
                let ip = interface.ip();

                if ip.is_loopback() || !ip.is_ipv4() {
                    continue;
                }

                urls.insert(format!("http://{}:{}", ip, address.port()));
            }
        }

        return urls.into_iter().collect();
    }

    vec![format!("http://{address}")]
}

fn log_fx_refresh_configuration(config: &FxRefreshConfig) {
    info!(
        provider = "frankfurter",
        base_currency = "EUR",
        refresh_interval_seconds = config.refresh_interval.as_secs(),
        endpoint = %config.base_url,
        "fx refresh configuration"
    );
}

fn log_asset_price_refresh_configuration(config: &AssetPriceRefreshConfig) {
    info!(
        provider = "coingecko",
        endpoint = %config.coingecko_base_url,
        "crypto asset price refresh configuration"
    );
    info!(
        provider = "coincap",
        endpoint = %config.coincap_base_url,
        api_key_configured = config.coincap_api_key.is_some(),
        "crypto asset price refresh fallback configuration"
    );
    info!(
        provider = "openfigi",
        endpoint = %config.openfigi_base_url,
        api_key_configured = config.openfigi_api_key.is_some(),
        "ISIN resolution configuration"
    );

    let stock_providers = config.stock_providers();

    if stock_providers.is_empty() {
        info!(
            enabled = false,
            "stock asset price refresh disabled: no provider configured"
        );
        return;
    }

    for provider in &stock_providers {
        info!(
            provider = provider.name(),
            refresh_interval_seconds = config.refresh_interval.as_secs(),
            endpoint = provider.base_url(),
            "stock asset price provider enabled"
        );
    }
}
