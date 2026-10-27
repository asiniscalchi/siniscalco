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
    .bind(input.rate.as_scaled_i64())
    .bind(updated_at)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO fx_rate_history (from_currency, to_currency, rate, recorded_at) VALUES (?, ?, ?, ?)",
    )
    .bind(input.from_currency.as_str())
    .bind(input.to_currency.as_str())
    .bind(input.rate.as_scaled_i64())
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
    .bind(input.rate.as_scaled_i64())
    .bind(updated_at)
    .execute(&mut **transaction)
    .await?;

    sqlx::query(
        "INSERT INTO fx_rate_history (from_currency, to_currency, rate, recorded_at) VALUES (?, ?, ?, ?)",
    )
    .bind(input.from_currency.as_str())
    .bind(input.to_currency.as_str())
    .bind(input.rate.as_scaled_i64())
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
            rate
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
                rate: FxRate::from_scaled_i64(row.get::<i64, _>("rate"))?,
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
            rate,
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
            rate: FxRate::from_scaled_i64(row.get::<i64, _>("rate"))?,
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
            rate,
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
                rate: FxRate::from_scaled_i64(row.get::<i64, _>("rate"))?,
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
    let rate = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT rate
        FROM fx_rates
        WHERE from_currency = ? AND to_currency = ?
        "#,
    )
    .bind(from_currency.as_str())
    .bind(to_currency.as_str())
    .fetch_optional(pool)
    .await?;

    rate.map(|value| FxRate::from_scaled_i64(value).map(|rate| rate.as_decimal()))
        .transpose()
}

/// Controls whether dates before the first observation may use that observation.
#[derive(Clone, Copy)]
pub(crate) enum HistoricalFxFallback {
    None,
    EarliestLater,
}

/// Select direct-pair history, breaking timestamp ties by history id.
/// OPENING uses EarliestLater; portfolio valuation uses only observations on/before as_of.
/// Keep the EarliestLater selection rule in sync with frozen migration 0008.
pub(crate) async fn select_historical_fx_rate<'e, E>(
    executor: E,
    from_currency: Currency,
    to_currency: Currency,
    as_of: &str,
    fallback: HistoricalFxFallback,
) -> Result<Option<FxRate>, StorageError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    if from_currency == to_currency {
        return Ok(Some(FxRate::from_scaled_i64(1_000_000)?));
    }
    let rate = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT COALESCE(
            (SELECT rate FROM fx_rate_history
             WHERE from_currency = ?1 AND to_currency = ?2
               AND date(recorded_at) <= date(?3)
             ORDER BY recorded_at DESC, id DESC LIMIT 1),
            (SELECT rate FROM fx_rate_history
             WHERE from_currency = ?1 AND to_currency = ?2
               AND date(recorded_at) > date(?3) AND ?4
             ORDER BY recorded_at ASC, id ASC LIMIT 1)
        )
        "#,
    )
    .bind(from_currency.as_str())
    .bind(to_currency.as_str())
    .bind(as_of)
    .bind(matches!(fallback, HistoricalFxFallback::EarliestLater))
    .fetch_one(executor)
    .await?;
    rate.map(FxRate::from_scaled_i64).transpose()
}

/// Portfolio history never substitutes an observation after as_of.
pub(crate) async fn get_historical_fx_rate(
    pool: &SqlitePool,
    from_currency: Currency,
    to_currency: Currency,
    as_of: &str,
) -> Result<Option<Decimal>, StorageError> {
    Ok(select_historical_fx_rate(
        pool,
        from_currency,
        to_currency,
        as_of,
        HistoricalFxFallback::None,
    )
    .await?
    .map(FxRate::as_decimal))
}

/// Returns `Some(Decimal::ONE)` when `from_currency == to_currency`,
/// otherwise delegates to `get_direct_fx_rate`.
pub(crate) async fn get_fx_rate_or_one(
    pool: &SqlitePool,
    from_currency: Currency,
    to_currency: Currency,
) -> Result<Option<Decimal>, StorageError> {
    if from_currency == to_currency {
        return Ok(Some(Decimal::ONE));
    }
    get_direct_fx_rate(pool, from_currency, to_currency).await
}
