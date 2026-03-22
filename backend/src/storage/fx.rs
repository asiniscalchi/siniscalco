use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

use crate::storage::accounts::validate_allowed_currency;
use crate::storage::balances::{parse_stored_decimal, validate_decimal_20_8};
use crate::storage::models::*;

pub async fn upsert_fx_rate(
    pool: &SqlitePool,
    input: UpsertFxRateInput<'_>,
) -> Result<UpsertOutcome, StorageError> {
    validate_allowed_currency(pool, input.from_currency).await?;
    validate_allowed_currency(pool, input.to_currency).await?;
    validate_decimal_20_8(input.rate)?;
    validate_positive_decimal(input.rate)?;

    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM fx_rates WHERE from_currency = ? AND to_currency = ?)",
    )
    .bind(input.from_currency)
    .bind(input.to_currency)
    .fetch_one(&mut *transaction)
    .await?
        != 0;

    sqlx::query(
        r#"
        INSERT INTO fx_rates (from_currency, to_currency, rate)
        VALUES (?, ?, ?)
        ON CONFLICT(from_currency, to_currency) DO UPDATE SET
            rate = excluded.rate
        "#,
    )
    .bind(input.from_currency)
    .bind(input.to_currency)
    .bind(input.rate)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    if existed {
        Ok(UpsertOutcome::Updated)
    } else {
        Ok(UpsertOutcome::Created)
    }
}

pub async fn list_currencies(pool: &SqlitePool) -> Result<Vec<CurrencyRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT code
        FROM currencies
        ORDER BY code
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| CurrencyRecord {
            code: row.get("code"),
        })
        .collect())
}

pub async fn list_fx_rates(pool: &SqlitePool) -> Result<Vec<FxRateRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            from_currency,
            to_currency,
            CAST(rate AS TEXT) AS rate
        FROM fx_rates
        ORDER BY from_currency, to_currency
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| FxRateRecord {
            from_currency: row.get("from_currency"),
            to_currency: row.get("to_currency"),
            rate: row.get("rate"),
        })
        .collect())
}

pub(crate) async fn get_direct_fx_rate(
    pool: &SqlitePool,
    from_currency: &str,
    to_currency: &str,
) -> Result<Option<Decimal>, StorageError> {
    let rate = sqlx::query_scalar::<_, String>(
        r#"
        SELECT CAST(rate AS TEXT) AS rate
        FROM fx_rates
        WHERE from_currency = ? AND to_currency = ?
        "#,
    )
    .bind(from_currency)
    .bind(to_currency)
    .fetch_optional(pool)
    .await?;

    rate.map(|value| parse_stored_decimal(&value)).transpose()
}

fn validate_positive_decimal(amount: &str) -> Result<(), StorageError> {
    if amount.starts_with('-')
        || amount == "0"
        || amount.starts_with("0.") && amount[2..].bytes().all(|byte| byte == b'0')
    {
        return Err(StorageError::Validation("rate must be greater than zero"));
    }

    Ok(())
}
