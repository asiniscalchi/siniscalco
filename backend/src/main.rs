use std::{collections::BTreeSet, net::SocketAddr};

use if_addrs::get_if_addrs;

use backend::{build_router, connect_db_file};

#[tokio::main]
async fn main() {
    match connect_db_file("data/app.db").await {
        Ok(pool) => {
            let app = build_router(pool);
            let address = SocketAddr::from(([0, 0, 0, 0], 3000));

            log_listening_addresses(address);

            match tokio::net::TcpListener::bind(address).await {
                Ok(listener) => {
                    if let Err(error) = axum::serve(listener, app).await {
                        eprintln!("backend server error: {error}");
                        std::process::exit(1);
                    }
                }
                Err(error) => {
                    eprintln!("failed to bind backend server: {error}");
                    std::process::exit(1);
                }
            }
        }
        Err(error) => {
            eprintln!("failed to initialize backend database: {error}");
            std::process::exit(1);
        }
    }
}

fn log_listening_addresses(address: SocketAddr) {
    println!("backend listening on:");

    for url in listening_urls(address) {
        println!("- {url}");
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
