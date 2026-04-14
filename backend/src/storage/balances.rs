use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnection};

use crate::format_decimal_amount;
use crate::storage::records::*;
use crate::storage::{AccountId, Amount, Currency, StorageError, TradeDate};

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

pub(crate) async fn list_account_balances_as_of(
    pool: &SqlitePool,
    account_id: AccountId,
    as_of: &str,
) -> Result<Vec<AccountBalanceRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            ce.currency,
            SUM(ce.amount) AS amount
        FROM cash_entries ce
        LEFT JOIN asset_transactions at ON ce.source = 'asset_transaction' AND ce.source_id = at.id
        LEFT JOIN account_transfers tr ON ce.source = 'transfer' AND ce.source_id = tr.id
        WHERE ce.account_id = ?
          AND (
            (ce.source = 'deposit' AND ce.date <= ?) OR
            (ce.source = 'asset_transaction' AND at.trade_date <= ?) OR
            (ce.source = 'transfer' AND tr.transfer_date <= ?)
          )
        GROUP BY ce.currency
        ORDER BY ce.currency
        "#,
    )
    .bind(account_id.as_i64())
    .bind(as_of)
    .bind(as_of)
    .bind(as_of)
    .bind(as_of)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AccountBalanceRecord {
                account_id,
                currency: Currency::try_from(row.get::<&str, _>("currency"))?,
                amount: Amount::from_scaled_i64(row.get::<i64, _>("amount")),
                updated_at: String::new(),
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
        INSERT INTO cash_entries (account_id, currency, amount, source, source_id, date, notes, created_at)
        VALUES (?, ?, ?, 'deposit', NULL, ?, ?, ?)
        "#,
    )
    .bind(input.account_id.as_i64())
    .bind(input.currency.as_str())
    .bind(stored_amount.as_scaled_i64())
    .bind(input.date.as_str())
    .bind(input.notes.as_deref())
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

pub async fn list_all_cash_movements(
    pool: &SqlitePool,
) -> Result<Vec<CashMovementRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, account_id, currency, amount, date, notes, created_at
        FROM cash_entries
        WHERE source = 'deposit'
        ORDER BY date DESC, created_at DESC, id DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    map_cash_movement_rows(rows)
}

pub async fn list_cash_movements(
    pool: &SqlitePool,
    account_id: AccountId,
) -> Result<Vec<CashMovementRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, account_id, currency, amount, date, notes, created_at
        FROM cash_entries
        WHERE account_id = ? AND source = 'deposit'
        ORDER BY date DESC, created_at DESC, id DESC
        "#,
    )
    .bind(account_id.as_i64())
    .fetch_all(pool)
    .await?;

    map_cash_movement_rows(rows)
}

fn map_cash_movement_rows(
    rows: Vec<sqlx::sqlite::SqliteRow>,
) -> Result<Vec<CashMovementRecord>, StorageError> {
    rows.into_iter()
        .map(|row| {
            let date_str: Option<&str> = row.get("date");
            let date = match date_str {
                Some(d) => TradeDate::try_from(d)?,
                None => TradeDate::try_from(&row.get::<&str, _>("created_at")[..10])?,
            };
            Ok(CashMovementRecord {
                id: row.get("id"),
                account_id: AccountId::try_from(row.get::<i64, _>("account_id"))?,
                currency: Currency::try_from(row.get::<&str, _>("currency"))?,
                amount: Amount::from_scaled_i64(row.get::<i64, _>("amount")),
                date,
                notes: row.get("notes"),
                created_at: row.get("created_at"),
            })
        })
        .collect()
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
