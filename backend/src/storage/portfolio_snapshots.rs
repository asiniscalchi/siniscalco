use sqlx::{Row, SqlitePool};

use crate::storage::records::PortfolioSnapshotRecord;
use crate::storage::{Amount, Currency, StorageError};

pub async fn insert_portfolio_snapshot_if_missing(
    pool: &SqlitePool,
    total_value: Amount,
    currency: Currency,
    recorded_at: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO portfolio_snapshots (total_value, currency_code, recorded_at)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(total_value.as_scaled_i64())
    .bind(currency.as_str())
    .bind(recorded_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_portfolio_snapshots(
    pool: &SqlitePool,
    currency: Currency,
) -> Result<Vec<PortfolioSnapshotRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT total_value, currency_code, recorded_at
        FROM portfolio_snapshots
        WHERE currency_code = ?
        ORDER BY recorded_at ASC
        "#,
    )
    .bind(currency.as_str())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(PortfolioSnapshotRecord {
                total_value: Amount::from_scaled_i64(row.get::<i64, _>("total_value")),
                currency: Currency::try_from(row.get::<&str, _>("currency_code"))?,
                recorded_at: row.get("recorded_at"),
            })
        })
        .collect()
}
