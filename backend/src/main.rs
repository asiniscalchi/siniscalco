use std::net::SocketAddr;

use backend::{build_router, connect_db_file};

#[tokio::main]
async fn main() {
    match connect_db_file("data/app.db").await {
        Ok(pool) => {
            let app = build_router(pool);
            let address = SocketAddr::from(([127, 0, 0, 1], 3000));

            println!("backend listening on http://{address}");

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
