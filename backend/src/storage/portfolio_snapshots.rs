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

    let mut replacements = Vec::new();

    for date in &dates {
        match compute_portfolio_value_at(pool, date, currency).await {
            Ok(Some(value)) => {
                replacements.push((format!("{date}T22:00:00Z"), value));
            }
            Ok(None) => {
                warn!(
                    date,
                    "removing snapshot: missing price or FX data for recalculation"
                );
            }
            Err(error) => {
                warn!(error = %error, date, "failed to compute historical portfolio value");
                return Err(error);
            }
        }
    }

    let mut tx = pool.begin().await?;
    let deleted = sqlx::query(
        r#"
        DELETE FROM portfolio_snapshots
        WHERE date(recorded_at) >= date(?) AND currency_code = ?
        "#,
    )
    .bind(from_date)
    .bind(currency.as_str())
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tracing::info!(deleted, from_date, "deleted stale portfolio snapshots");

    for (recorded_at, value) in replacements {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO portfolio_snapshots (total_value, currency_code, recorded_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(value.as_scaled_i64())
        .bind(currency.as_str())
        .bind(recorded_at)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}
