use backend::connect_db_file;

#[tokio::main]
async fn main() {
    match connect_db_file("data/app.db").await {
        Ok(_) => println!("backend database ready"),
        Err(error) => {
            eprintln!("failed to initialize backend database: {error}");
            std::process::exit(1);
        }
    }
}
