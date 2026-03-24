use sqlx::SqlitePool;

use crate::storage::records::{UpsertAssetPriceInput, current_utc_timestamp_iso8601};
use crate::storage::{StorageError, UpsertOutcome};

pub async fn upsert_asset_price(
    pool: &SqlitePool,
    input: UpsertAssetPriceInput,
) -> Result<UpsertOutcome, StorageError> {
    let updated_at = current_utc_timestamp_iso8601()?;
    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM asset_prices WHERE asset_id = ?)",
    )
    .bind(input.asset_id.as_i64())
    .fetch_one(&mut *transaction)
    .await?
        != 0;

    sqlx::query(
        r#"
        INSERT INTO asset_prices (asset_id, price, currency_code, as_of, updated_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(asset_id) DO UPDATE SET
            price = excluded.price,
            currency_code = excluded.currency_code,
            as_of = excluded.as_of,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(input.asset_id.as_i64())
    .bind(input.price.as_scaled_i64())
    .bind(input.currency.as_str())
    .bind(input.as_of)
    .bind(updated_at)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    if existed {
        Ok(UpsertOutcome::Updated)
    } else {
        Ok(UpsertOutcome::Created)
    }
}
