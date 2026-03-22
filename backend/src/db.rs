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
    if let Some(parent) = path.as_ref().parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| sqlx::Error::Configuration(Box::new(error)))?;
        }
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
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'account_balances', 'currencies', '_sqlx_migrations')",
        )
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed");

        assert_eq!(tables, 4);
    }

    #[tokio::test]
    async fn bootstraps_file_database_and_runs_migrations() {
        let file = NamedTempFile::new().expect("temp db file should be created");
        let pool = connect_db_file(file.path())
            .await
            .expect("file database should initialize");

        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'account_balances', 'currencies')",
        )
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed");

        assert_eq!(tables, 3);
    }

    #[tokio::test]
    async fn migration_metadata_contains_the_initial_migration() {
        let pool = test_pool().await;

        let versions: Vec<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&pool)
            .await
            .expect("migration metadata query should succeed");

        assert_eq!(versions, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn seeds_supported_currencies() {
        let pool = test_pool().await;

        let codes: Vec<String> = sqlx::query_scalar("SELECT code FROM currencies ORDER BY code")
            .fetch_all(&pool)
            .await
            .expect("currency seed query should succeed");

        assert_eq!(codes, vec!["CHF", "EUR", "GBP", "USD"]);
    }

    #[tokio::test]
    async fn account_currency_migration_preserves_existing_records() {
        let file = NamedTempFile::new().expect("temp db file should be created");
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", file.path().display()))
            .expect("sqlite connect options should parse")
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("sqlite pool should connect");

        sqlx::raw_sql(include_str!("../migrations/0001_initial_cash_storage.sql"))
            .execute(&pool)
            .await
            .expect("initial schema should apply");
        sqlx::raw_sql(include_str!("../migrations/0002_create_currencies.sql"))
            .execute(&pool)
            .await
            .expect("currencies schema should apply");
        sqlx::raw_sql(include_str!("../migrations/0003_seed_currencies.sql"))
            .execute(&pool)
            .await
            .expect("currency seeds should apply");

        sqlx::query(
            "INSERT INTO accounts (id, name, account_type, base_currency, created_at) VALUES (1, 'IBKR', 'broker', 'EUR', '2026-03-22 00:00:00')",
        )
        .execute(&pool)
        .await
        .expect("legacy account insert should succeed");

        sqlx::raw_sql(include_str!("../migrations/0004_link_account_currencies.sql"))
            .execute(&pool)
            .await
            .expect("account currency migration should apply");

        let currency: String =
            sqlx::query_scalar("SELECT base_currency FROM accounts WHERE id = 1")
                .fetch_one(&pool)
                .await
                .expect("migrated account query should succeed");

        assert_eq!(currency, "EUR");
    }
}
