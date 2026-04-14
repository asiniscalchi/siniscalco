use sqlx::{Row, SqlitePool};
use tracing::warn;

use crate::storage::portfolio::compute_portfolio_value_at;
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

pub async fn delete_snapshots_from_date(
    pool: &SqlitePool,
    from_date: &str,
    currency: Currency,
) -> Result<u64, StorageError> {
    let result = sqlx::query(
        r#"
        DELETE FROM portfolio_snapshots
        WHERE date(recorded_at) >= date(?) AND currency_code = ?
        "#,
    )
    .bind(from_date)
    .bind(currency.as_str())
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn recalculate_snapshots_from_date(
    pool: &SqlitePool,
    from_date: &str,
    currency: Currency,
) -> Result<(), StorageError> {
    let dates: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT date(recorded_at) AS snap_date
        FROM portfolio_snapshots
        WHERE date(recorded_at) >= date(?) AND currency_code = ?
        ORDER BY snap_date
        "#,
    )
    .bind(from_date)
    .bind(currency.as_str())
    .fetch_all(pool)
    .await?;

    if dates.is_empty() {
        return Ok(());
    }

    let deleted = delete_snapshots_from_date(pool, from_date, currency).await?;
    tracing::info!(deleted, from_date, "deleted stale portfolio snapshots");

    for date in &dates {
        match compute_portfolio_value_at(pool, date, currency).await {
            Ok(Some(value)) => {
                let recorded_at = format!("{date}T22:00:00Z");
                if let Err(error) =
                    insert_portfolio_snapshot_if_missing(pool, value, currency, &recorded_at).await
                {
                    warn!(error = %error, date, "failed to reinsert snapshot");
                }
            }
            Ok(None) => {
                warn!(
                    date,
                    "skipping snapshot recalculation: missing price or FX data"
                );
            }
            Err(error) => {
                warn!(error = %error, date, "failed to compute historical portfolio value");
            }
        }
    }

    Ok(())
}
