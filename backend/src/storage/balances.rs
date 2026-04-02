use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnection};

use crate::format_decimal_amount;
use crate::storage::records::*;
use crate::storage::{AccountId, Amount, Currency, StorageError};

pub async fn upsert_account_balance(
    pool: &SqlitePool,
    input: UpsertAccountBalanceInput,
) -> Result<UpsertOutcome, StorageError> {
    let updated_at = current_utc_timestamp()?;
    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM account_balances WHERE account_id = ? AND currency = ?)",
    )
    .bind(input.account_id.as_i64())
    .bind(input.currency.as_str())
    .fetch_one(&mut *transaction)
    .await?
        != 0;

    sqlx::query(
        r#"
        INSERT INTO account_balances (account_id, currency, amount, updated_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(account_id, currency) DO UPDATE SET
            amount = excluded.amount,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(input.account_id.as_i64())
    .bind(input.currency.as_str())
    .bind(input.amount.as_scaled_i64())
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

pub async fn list_account_balances(
    pool: &SqlitePool,
    account_id: AccountId,
) -> Result<Vec<AccountBalanceRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            account_id,
            currency,
            amount,
            updated_at
        FROM account_balances
        WHERE account_id = ?
        ORDER BY currency
        "#,
    )
    .bind(account_id.as_i64())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AccountBalanceRecord {
                account_id: AccountId::try_from(row.get::<i64, _>("account_id"))?,
                currency: Currency::try_from(row.get::<&str, _>("currency"))?,
                amount: Amount::from_scaled_i64(row.get::<i64, _>("amount")),
                updated_at: row.get("updated_at"),
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()
}

pub async fn delete_account_balance(
    pool: &SqlitePool,
    account_id: AccountId,
    currency: Currency,
) -> Result<(), StorageError> {
    let result = sqlx::query("DELETE FROM account_balances WHERE account_id = ? AND currency = ?")
        .bind(account_id.as_i64())
        .bind(currency.as_str())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}

// ── Connection-scoped helpers (used inside open transactions) ─────────────────

pub(super) async fn load_balance_on_connection(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    currency: Currency,
) -> Result<Decimal, StorageError> {
    let amount = sqlx::query_scalar::<_, i64>(
        "SELECT amount FROM account_balances WHERE account_id = ? AND currency = ?",
    )
    .bind(account_id.as_i64())
    .bind(currency.as_str())
    .fetch_optional(&mut *connection)
    .await?;
    Ok(amount
        .map(|v| Amount::from_scaled_i64(v).as_decimal())
        .unwrap_or(Decimal::ZERO))
}

pub(super) async fn upsert_balance_on_connection(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    currency: Currency,
    new_amount: Decimal,
    updated_at: &str,
) -> Result<(), StorageError> {
    let amount = Amount::try_from(format_decimal_amount(new_amount).as_str())?;
    sqlx::query(
        r#"
        INSERT INTO account_balances (account_id, currency, amount, updated_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(account_id, currency) DO UPDATE SET
            amount = excluded.amount,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(account_id.as_i64())
    .bind(currency.as_str())
    .bind(amount.as_scaled_i64())
    .bind(updated_at)
    .execute(&mut *connection)
    .await?;
    Ok(())
}
