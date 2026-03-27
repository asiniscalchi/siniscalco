use std::{collections::BTreeSet, net::SocketAddr};

use clap::Parser;
use if_addrs::get_if_addrs;
use tracing::{error, info};

use backend::{
    AppState, AssetPriceRefreshConfig, Config, FxRefreshConfig, build_router_with_state,
    connect_db_file, init_tracing, new_shared_fx_refresh_status, spawn_asset_price_refresh_task,
    spawn_fx_refresh_task,
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

    match connect_db_file(&config.db_path).await {
        Ok(pool) => {
            let fx_refresh_status = new_shared_fx_refresh_status();
            let fx_refresh_config = config.fx_refresh_config();
            let asset_price_refresh_config = config.asset_price_refresh_config();
            let http_client = reqwest::Client::new();
            let app = build_router_with_state(AppState {
                pool: pool.clone(),
                fx_refresh_status: fx_refresh_status.clone(),
                asset_price_refresh_config: asset_price_refresh_config.clone(),
                http_client,
            });
            let address = SocketAddr::from(([0, 0, 0, 0], config.port));

            log_fx_refresh_configuration(&fx_refresh_config);
            log_asset_price_refresh_configuration(&asset_price_refresh_config);

            spawn_fx_refresh_task(pool.clone(), fx_refresh_status, fx_refresh_config).await;
            spawn_asset_price_refresh_task(pool.clone(), asset_price_refresh_config).await;

            log_listening_addresses(address);

            match tokio::net::TcpListener::bind(address).await {
                Ok(listener) => {
                    if let Err(error) = axum::serve(listener, app).await {
                        error!(error = %error, "backend server error");
                        std::process::exit(1);
                    }
                }
                Err(error) => {
                    error!(error = %error, "failed to bind backend server");
                    std::process::exit(1);
                }
            }
        }
        Err(error) => {
            error!(error = %error, "failed to initialize backend database");
            std::process::exit(1);
        }
    }
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

    let stock_providers: &[(&str, bool, &str)] = &[
        (
            "twelve_data",
            config.twelve_data_api_key.is_some(),
            &config.twelve_data_base_url,
        ),
        (
            "finnhub",
            config.finnhub_api_key.is_some(),
            &config.finnhub_base_url,
        ),
        (
            "alpha_vantage",
            config.alpha_vantage_api_key.is_some(),
            &config.alpha_vantage_base_url,
        ),
        (
            "polygon",
            config.polygon_api_key.is_some(),
            &config.polygon_base_url,
        ),
        ("fmp", config.fmp_api_key.is_some(), &config.fmp_base_url),
        (
            "eodhd",
            config.eodhd_api_key.is_some(),
            &config.eodhd_base_url,
        ),
        (
            "tiingo",
            config.tiingo_api_key.is_some(),
            &config.tiingo_base_url,
        ),
        (
            "marketstack",
            config.marketstack_api_key.is_some(),
            &config.marketstack_base_url,
        ),
    ];

    let enabled_count = stock_providers
        .iter()
        .filter(|(_, enabled, _)| *enabled)
        .count();

    if enabled_count == 0 {
        info!(
            enabled = false,
            "stock asset price refresh disabled: no provider API keys configured"
        );
        return;
    }

    for (provider, enabled, endpoint) in stock_providers {
        if *enabled {
            info!(
                provider,
                refresh_interval_seconds = config.refresh_interval.as_secs(),
                endpoint,
                "stock asset price provider enabled"
            );
        }
    }
}
