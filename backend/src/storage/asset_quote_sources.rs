use sqlx::SqlitePool;

use crate::storage::records::{UpsertAssetQuoteSourceInput, current_utc_timestamp};
use crate::storage::{StorageError, UpsertOutcome};

pub async fn upsert_asset_quote_source(
    pool: &SqlitePool,
    input: UpsertAssetQuoteSourceInput,
) -> Result<UpsertOutcome, StorageError> {
    let updated_at = current_utc_timestamp()?;
    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM asset_quote_sources WHERE asset_id = ?)",
    )
    .bind(input.asset_id.as_i64())
    .fetch_one(&mut *transaction)
    .await?
        != 0;

    sqlx::query(
        r#"
        INSERT INTO asset_quote_sources (
            asset_id,
            quote_symbol,
            provider,
            last_success_at,
            created_at,
            updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(asset_id) DO UPDATE SET
            quote_symbol = excluded.quote_symbol,
            provider = excluded.provider,
            last_success_at = excluded.last_success_at,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(input.asset_id.as_i64())
    .bind(input.quote_symbol.trim())
    .bind(input.provider.trim())
    .bind(&input.last_success_at)
    .bind(&updated_at)
    .bind(&updated_at)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    if existed {
        Ok(UpsertOutcome::Updated)
    } else {
        Ok(UpsertOutcome::Created)
    }
}
