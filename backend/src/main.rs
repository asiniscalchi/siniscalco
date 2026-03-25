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

    if let Err(error) = dotenvy::dotenv() {
        if !error.not_found() {
            eprintln!("failed to load .env file: {error}");
            std::process::exit(1);
        }
    }

    let config = Config::parse();

    match connect_db_file(&config.db_path).await {
        Ok(pool) => {
            let fx_refresh_status = new_shared_fx_refresh_status();
            let fx_refresh_config = config.fx_refresh_config();
            let asset_price_refresh_config = config.asset_price_refresh_config();
            let app = build_router_with_state(AppState {
                pool: pool.clone(),
                fx_refresh_status: fx_refresh_status.clone(),
                asset_price_refresh_config: asset_price_refresh_config.clone(),
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
    if config.twelve_data_api_key.is_some() {
        info!(
            provider = "twelve_data",
            refresh_interval_seconds = config.refresh_interval.as_secs(),
            endpoint = %config.twelve_data_base_url,
            "asset price refresh configuration"
        );
    } else if config.finnhub_api_key.is_some() {
        info!(
            provider = "finnhub",
            refresh_interval_seconds = config.refresh_interval.as_secs(),
            endpoint = %config.finnhub_base_url,
            "asset price refresh configuration"
        );
    } else if config.alpha_vantage_api_key.is_some() {
        info!(
            provider = "alpha_vantage",
            refresh_interval_seconds = config.refresh_interval.as_secs(),
            endpoint = %config.alpha_vantage_base_url,
            "asset price refresh configuration"
        );
    } else {
        info!(enabled = false, "asset price refresh configuration");
    }
}
