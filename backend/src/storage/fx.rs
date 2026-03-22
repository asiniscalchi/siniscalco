use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

use crate::storage::balances::{parse_stored_decimal, validate_decimal_20_8};
use crate::storage::models::*;

pub async fn upsert_fx_rate(
    pool: &SqlitePool,
    input: UpsertFxRateInput<'_>,
) -> Result<UpsertOutcome, StorageError> {
    validate_decimal_20_8(input.rate)?;
    validate_positive_decimal(input.rate)?;

    let updated_at = current_utc_timestamp()?;
    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM fx_rates WHERE from_currency = ? AND to_currency = ?)",
    )
    .bind(input.from_currency.as_str())
    .bind(input.to_currency.as_str())
    .fetch_one(&mut *transaction)
    .await?
        != 0;

    sqlx::query(
        r#"
        INSERT INTO fx_rates (from_currency, to_currency, rate, updated_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(from_currency, to_currency) DO UPDATE SET
            rate = excluded.rate,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(input.from_currency.as_str())
    .bind(input.to_currency.as_str())
    .bind(input.rate)
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
            code: Currency::try_from(row.get::<&str, _>("code")).expect("stored currency is valid"),
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
            from_currency: Currency::try_from(row.get::<&str, _>("from_currency"))
                .expect("stored currency is valid"),
            to_currency: Currency::try_from(row.get::<&str, _>("to_currency"))
                .expect("stored currency is valid"),
            rate: row.get("rate"),
        })
        .collect())
}

pub async fn list_fx_rate_summary(
    pool: &SqlitePool,
    target_currency: Currency,
) -> Result<FxRateSummaryRecord, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            from_currency,
            CAST(rate AS TEXT) AS rate,
            updated_at
        FROM fx_rates
        WHERE to_currency = ? AND from_currency != ?
        ORDER BY from_currency
        "#,
    )
    .bind(target_currency.as_str())
    .bind(target_currency.as_str())
    .fetch_all(pool)
    .await?;

    let rates: Vec<FxRateSummaryItemRecord> = rows
        .into_iter()
        .map(|row| FxRateSummaryItemRecord {
            from_currency: Currency::try_from(row.get::<&str, _>("from_currency"))
                .expect("stored currency is valid"),
            rate: row.get("rate"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    let last_updated = rates.iter().map(|rate| rate.updated_at.clone()).max();

    Ok(FxRateSummaryRecord {
        target_currency,
        rates,
        last_updated,
    })
}

pub(crate) async fn get_direct_fx_rate(
    pool: &SqlitePool,
    from_currency: Currency,
    to_currency: Currency,
) -> Result<Option<Decimal>, StorageError> {
    let rate = sqlx::query_scalar::<_, String>(
        r#"
        SELECT CAST(rate AS TEXT) AS rate
        FROM fx_rates
        WHERE from_currency = ? AND to_currency = ?
        "#,
    )
    .bind(from_currency.as_str())
    .bind(to_currency.as_str())
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
