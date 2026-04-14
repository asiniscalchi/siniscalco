use std::{fs, path::Path, str::FromStr};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tracing::info;

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
    let migration_versions = MIGRATOR
        .iter()
        .map(|migration| migration.version)
        .collect::<Vec<_>>();
    let latest_available_version = migration_versions.last().copied();
    let current_version = latest_applied_migration_version(pool).await?;

    info!(
        current_version = ?current_version,
        latest_available_version = ?latest_available_version,
        migration_count = migration_versions.len(),
        migration_versions = ?migration_versions,
        "database migration status"
    );

    MIGRATOR.run(pool).await?;

    let current_version = latest_applied_migration_version(pool).await?;
    info!(
        current_version = ?current_version,
        latest_available_version = ?latest_available_version,
        "database migrations initialized"
    );

    Ok(())
}

async fn latest_applied_migration_version(pool: &SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    let migrations_table_exists: i64 = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1
            FROM sqlite_master
            WHERE type = 'table' AND name = '_sqlx_migrations'
        )",
    )
    .fetch_one(pool)
    .await?;

    if migrations_table_exists == 0 {
        return Ok(None);
    }

    sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tempfile::NamedTempFile;

    use super::{MIGRATOR, connect_db_file, init_db, latest_applied_migration_version};
    use crate::Currency;

    async fn test_pool() -> sqlx::SqlitePool {
        let pool = uninitialized_test_pool().await;

        init_db(&pool).await.expect("schema should initialize");
        pool
    }

    async fn uninitialized_test_pool() -> sqlx::SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .expect("in-memory sqlite connect options should parse")
            .foreign_keys(true);

        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("in-memory sqlite pool should connect")
    }

    #[tokio::test]
    async fn applies_migrations_and_creates_tables() {
        let pool = test_pool().await;

        let tables: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'cash_entries', 'assets', 'asset_transactions', 'currencies', '_sqlx_migrations')",
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
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('accounts', 'cash_entries', 'assets', 'asset_transactions', 'currencies')",
        )
        .fetch_one(&pool)
        .await
        .expect("table lookup should succeed");

        assert_eq!(tables, 5);
    }

    #[tokio::test]
    async fn migration_metadata_contains_all_migrations() {
        let pool = test_pool().await;

        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .expect("migration metadata query should succeed");
        let expected_versions = MIGRATOR
            .iter()
            .map(|migration| migration.version)
            .collect::<Vec<_>>();

        assert_eq!(versions, expected_versions);
    }

    #[tokio::test]
    async fn latest_applied_migration_version_returns_none_before_migrations() {
        let pool = uninitialized_test_pool().await;

        let version = latest_applied_migration_version(&pool)
            .await
            .expect("migration version lookup should succeed");

        assert_eq!(version, None);
    }

    #[tokio::test]
    async fn latest_applied_migration_version_returns_latest_migration_after_init() {
        let pool = test_pool().await;

        let version = latest_applied_migration_version(&pool)
            .await
            .expect("migration version lookup should succeed");
        let expected_version = MIGRATOR.iter().map(|migration| migration.version).last();

        assert_eq!(version, expected_version);
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
