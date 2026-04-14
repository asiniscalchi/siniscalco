use sqlx::{Row, SqlitePool};

use crate::storage::records::{UpsertAssetPriceInput, current_utc_timestamp};
use crate::storage::{Amount, AssetId, Currency, StorageError, UpsertOutcome};

pub async fn upsert_asset_price(
    pool: &SqlitePool,
    input: UpsertAssetPriceInput,
) -> Result<UpsertOutcome, StorageError> {
    let updated_at = current_utc_timestamp()?;
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
    .bind(&input.as_of)
    .bind(&updated_at)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO asset_price_history (asset_id, price, currency_code, recorded_at) VALUES (?, ?, ?, ?)",
    )
    .bind(input.asset_id.as_i64())
    .bind(input.price.as_scaled_i64())
    .bind(input.currency.as_str())
    .bind(&input.as_of)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    if existed {
        Ok(UpsertOutcome::Updated)
    } else {
        Ok(UpsertOutcome::Created)
    }
}

/// Returns the most recent asset price from `asset_price_history` at or before `as_of`.
pub(crate) async fn get_historical_asset_price(
    pool: &SqlitePool,
    asset_id: AssetId,
    as_of: &str,
) -> Result<Option<(Amount, Currency)>, StorageError> {
    let row = sqlx::query(
        r#"
        SELECT price, currency_code
        FROM asset_price_history
        WHERE asset_id = ? AND date(recorded_at) <= date(?)
        ORDER BY recorded_at DESC
        LIMIT 1
        "#,
    )
    .bind(asset_id.as_i64())
    .bind(as_of)
    .fetch_optional(pool)
    .await?;

    row.map(|r| {
        Ok((
            Amount::from_scaled_i64(r.get::<i64, _>("price")),
            Currency::try_from(r.get::<&str, _>("currency_code"))?,
        ))
    })
    .transpose()
}
