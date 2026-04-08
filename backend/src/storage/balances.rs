use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnection};

use crate::format_decimal_amount;
use crate::storage::records::*;
use crate::storage::{AccountId, Amount, Currency, StorageError};

pub async fn list_account_balances(
    pool: &SqlitePool,
    account_id: AccountId,
) -> Result<Vec<AccountBalanceRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            account_id,
            currency,
            SUM(amount) AS amount,
            MAX(created_at) AS updated_at
        FROM cash_entries
        WHERE account_id = ?
        GROUP BY account_id, currency
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

pub async fn create_cash_movement(
    pool: &SqlitePool,
    input: CreateCashMovementInput,
) -> Result<CashMovementRecord, StorageError> {
    let mut tx = pool.begin().await?;
    let created_at = crate::storage::records::current_utc_timestamp()?;
    let stored_amount =
        Amount::try_from(format_decimal_amount(input.amount.as_decimal()).as_str())?;

    let result = sqlx::query(
        r#"
        INSERT INTO cash_entries (account_id, currency, amount, source, source_id, created_at)
        VALUES (?, ?, ?, 'deposit', NULL, ?)
        "#,
    )
    .bind(input.account_id.as_i64())
    .bind(input.currency.as_str())
    .bind(stored_amount.as_scaled_i64())
    .bind(&created_at)
    .execute(&mut *tx)
    .await?;

    let id = result.last_insert_rowid();
    tx.commit().await?;

    Ok(CashMovementRecord {
        id,
        account_id: input.account_id,
        currency: input.currency,
        amount: stored_amount,
        date: input.date,
        notes: input.notes,
        created_at,
    })
}

// ── Connection-scoped helpers (used inside open transactions) ─────────────────

pub(super) async fn insert_cash_entry_on_connection(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    currency: Currency,
    amount: Decimal,
    source: CashEntrySource,
    source_id: Option<i64>,
    created_at: &str,
) -> Result<(), StorageError> {
    let stored_amount = Amount::try_from(format_decimal_amount(amount).as_str())?;
    sqlx::query(
        r#"
        INSERT INTO cash_entries (account_id, currency, amount, source, source_id, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(account_id.as_i64())
    .bind(currency.as_str())
    .bind(stored_amount.as_scaled_i64())
    .bind(source.as_str())
    .bind(source_id)
    .bind(created_at)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub(super) enum CashEntrySource {
    AssetTransaction,
    Transfer,
}

impl CashEntrySource {
    fn as_str(&self) -> &'static str {
        match self {
            CashEntrySource::AssetTransaction => "asset_transaction",
            CashEntrySource::Transfer => "transfer",
        }
    }
}
