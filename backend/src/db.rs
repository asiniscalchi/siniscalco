use std::{fs, path::Path, str::FromStr};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn connect_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    init_db(&pool).await?;
    Ok(pool)
}

pub async fn connect_db_file(path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::Error> {
    if let Some(parent) = path.as_ref().parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| sqlx::Error::Configuration(Box::new(error)))?;
    }

    let url = format!("sqlite://{}", path.as_ref().display());
    connect_db(&url).await
}

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tempfile::NamedTempFile;

    use super::{connect_db_file, init_db};
    use crate::Currency;

    async fn test_pool() -> sqlx::SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .expect("in-memory sqlite connect options should parse")
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("in-memory sqlite pool should connect");

        init_db(&pool).await.expect("schema should initialize");
        pool
    }

    #[tokio::test]
    async fn applies_migrations_and_creates_tables() {
        let pool = test_pool().await;

        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'account_balances', 'assets', 'asset_transactions', 'currencies', '_sqlx_migrations')",
        )
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed");

        assert_eq!(tables, 6);
    }

    #[tokio::test]
    async fn bootstraps_file_database_and_runs_migrations() {
        let file = NamedTempFile::new().expect("temp db file should be created");
        let pool = connect_db_file(file.path())
            .await
            .expect("file database should initialize");

        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'account_balances', 'assets', 'asset_transactions', 'currencies')",
        )
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed");

        assert_eq!(tables, 5);
    }

    #[tokio::test]
    async fn migration_metadata_contains_all_migrations() {
        let pool = test_pool().await;

        let versions: Vec<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations")
            .fetch_all(&pool)
            .await
            .expect("migration metadata query should succeed");

        assert_eq!(versions, vec![1]);
    }

    #[tokio::test]
    async fn creates_asset_transaction_indexes() {
        let pool = test_pool().await;

        let indexes: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name IN ('asset_transactions_account_trade_date_idx', 'asset_transactions_account_asset_idx') ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .expect("index lookup should succeed");

        assert_eq!(
            indexes,
            vec![
                "asset_transactions_account_asset_idx".to_string(),
                "asset_transactions_account_trade_date_idx".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn seeds_supported_currencies() {
        let pool = test_pool().await;

        let codes: Vec<String> = sqlx::query_scalar("SELECT code FROM currencies ORDER BY code")
            .fetch_all(&pool)
            .await
            .expect("currency seed query should succeed");

        assert_eq!(
            codes,
            Currency::all()
                .into_iter()
                .map(|currency| currency.as_str().to_string())
                .collect::<Vec<_>>()
        );
    }
}
