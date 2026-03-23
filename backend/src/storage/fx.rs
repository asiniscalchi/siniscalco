use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

use crate::fx_refresh::PRODUCT_BASE_CURRENCY;
use crate::storage::records::*;
use crate::storage::{Currency, FxRate, StorageError};

pub async fn upsert_fx_rate(
    pool: &SqlitePool,
    input: UpsertFxRateInput,
) -> Result<UpsertOutcome, StorageError> {
    let updated_at = current_utc_timestamp()?;
    upsert_fx_rate_at(pool, input, &updated_at).await
}

pub async fn replace_fx_rates(
    pool: &SqlitePool,
    inputs: Vec<UpsertFxRateInput>,
    updated_at: &str,
) -> Result<(), StorageError> {
    let mut transaction = pool.begin().await?;

    for input in inputs {
        validate_fx_pair(input.from_currency, input.to_currency)?;
        upsert_fx_rate_in_transaction(&mut transaction, input, updated_at).await?;
    }

    transaction.commit().await?;
    Ok(())
}

async fn upsert_fx_rate_at(
    pool: &SqlitePool,
    input: UpsertFxRateInput,
    updated_at: &str,
) -> Result<UpsertOutcome, StorageError> {
    validate_fx_pair(input.from_currency, input.to_currency)?;
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
    .bind(input.rate.to_string())
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

async fn upsert_fx_rate_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    input: UpsertFxRateInput,
    updated_at: &str,
) -> Result<(), StorageError> {
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
    .bind(input.rate.to_string())
    .bind(updated_at)
    .execute(&mut **transaction)
    .await?;

    Ok(())
}

fn validate_fx_pair(from_currency: Currency, to_currency: Currency) -> Result<(), StorageError> {
    if from_currency == to_currency {
        return Err(StorageError::Validation(
            "fx pair must contain two different currencies",
        ));
    }

    if to_currency != PRODUCT_BASE_CURRENCY || from_currency == PRODUCT_BASE_CURRENCY {
        return Err(StorageError::Validation(
            "fx pair must convert a supported non-EUR currency into EUR",
        ));
    }

    Ok(())
}

pub async fn list_currencies(_pool: &SqlitePool) -> Result<Vec<CurrencyRecord>, StorageError> {
    Ok(Currency::all()
        .into_iter()
        .map(|code| CurrencyRecord { code })
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

    rows.into_iter()
        .map(|row| {
            Ok(FxRateRecord {
                from_currency: Currency::try_from(row.get::<&str, _>("from_currency"))?,
                to_currency: Currency::try_from(row.get::<&str, _>("to_currency"))?,
                rate: FxRate::try_from(row.get::<&str, _>("rate"))?,
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()
}

pub async fn get_latest_fx_rate(
    pool: &SqlitePool,
    from_currency: Currency,
    to_currency: Currency,
) -> Result<Option<FxRateDetailRecord>, StorageError> {
    validate_fx_pair(from_currency, to_currency)?;

    let row = sqlx::query(
        r#"
        SELECT
            from_currency,
            to_currency,
            CAST(rate AS TEXT) AS rate,
            updated_at
        FROM fx_rates
        WHERE from_currency = ? AND to_currency = ?
        "#,
    )
    .bind(from_currency.as_str())
    .bind(to_currency.as_str())
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(FxRateDetailRecord {
            from_currency: Currency::try_from(row.get::<&str, _>("from_currency"))?,
            to_currency: Currency::try_from(row.get::<&str, _>("to_currency"))?,
            rate: FxRate::try_from(row.get::<&str, _>("rate"))?,
            updated_at: row.get("updated_at"),
        })
    })
    .transpose()
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
        .map(|row| {
            Ok(FxRateSummaryItemRecord {
                from_currency: Currency::try_from(row.get::<&str, _>("from_currency"))?,
                rate: FxRate::try_from(row.get::<&str, _>("rate"))?,
                updated_at: row.get("updated_at"),
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()?;

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

    rate.map(|value| FxRate::try_from(value.as_str()).map(|rate| rate.as_decimal()))
        .transpose()
}
