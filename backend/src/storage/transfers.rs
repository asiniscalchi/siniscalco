use sqlx::{Row, SqlitePool};

use crate::storage::records::*;
use crate::storage::{
    AccountId, Amount, Currency, StorageError, TradeDate, TransferId, current_utc_timestamp,
};

use super::balances::{CashEntrySource, insert_cash_entry_on_connection};

pub async fn create_transfer(
    pool: &SqlitePool,
    input: CreateTransferInput,
) -> Result<TransferRecord, StorageError> {
    if input.from_account_id == input.to_account_id {
        return Err(StorageError::Validation(
            "from and to accounts must be different",
        ));
    }

    let mut tx = pool.begin().await?;
    let timestamp = current_utc_timestamp()?;

    // Insert the transfer record first to obtain its id for source_id.
    let result = sqlx::query(
        r#"
        INSERT INTO account_transfers (
            from_account_id, to_account_id,
            from_currency, from_amount,
            to_currency, to_amount,
            transfer_date, notes, created_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(input.from_account_id.as_i64())
    .bind(input.to_account_id.as_i64())
    .bind(input.from_currency.as_str())
    .bind(input.from_amount.as_scaled_i64())
    .bind(input.to_currency.as_str())
    .bind(input.to_amount.as_scaled_i64())
    .bind(input.transfer_date.as_str())
    .bind(input.notes.as_deref())
    .bind(&timestamp)
    .execute(&mut *tx)
    .await?;

    let transfer_id = TransferId::try_from(result.last_insert_rowid())?;

    // Debit the source account.
    insert_cash_entry_on_connection(
        &mut *tx,
        input.from_account_id,
        input.from_currency,
        -input.from_amount.as_decimal(),
        CashEntrySource::Transfer,
        Some(transfer_id.as_i64()),
        &timestamp,
    )
    .await?;

    // Credit the destination account.
    insert_cash_entry_on_connection(
        &mut *tx,
        input.to_account_id,
        input.to_currency,
        input.to_amount.as_decimal(),
        CashEntrySource::Transfer,
        Some(transfer_id.as_i64()),
        &timestamp,
    )
    .await?;

    let row = sqlx::query(
        r#"
        SELECT id, from_account_id, to_account_id,
               from_currency, from_amount, to_currency, to_amount,
               transfer_date, notes, created_at
        FROM account_transfers
        WHERE id = ?
        "#,
    )
    .bind(transfer_id.as_i64())
    .fetch_one(&mut *tx)
    .await?;

    let record = map_transfer_row(row)?;
    tx.commit().await?;
    Ok(record)
}

pub async fn delete_transfer(
    pool: &SqlitePool,
    transfer_id: TransferId,
) -> Result<(), StorageError> {
    let mut tx = pool.begin().await?;
    let timestamp = current_utc_timestamp()?;

    let row = sqlx::query(
        r#"
        SELECT id, from_account_id, to_account_id,
               from_currency, from_amount, to_currency, to_amount,
               transfer_date, notes, created_at
        FROM account_transfers
        WHERE id = ?
        "#,
    )
    .bind(transfer_id.as_i64())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(StorageError::Database(sqlx::Error::RowNotFound))?;

    let transfer = map_transfer_row(row)?;

    // Reverse: credit the source account back.
    insert_cash_entry_on_connection(
        &mut *tx,
        transfer.from_account_id,
        transfer.from_currency,
        transfer.from_amount.as_decimal(),
        CashEntrySource::Transfer,
        Some(transfer_id.as_i64()),
        &timestamp,
    )
    .await?;

    // Reverse: debit the destination account.
    insert_cash_entry_on_connection(
        &mut *tx,
        transfer.to_account_id,
        transfer.to_currency,
        -transfer.to_amount.as_decimal(),
        CashEntrySource::Transfer,
        Some(transfer_id.as_i64()),
        &timestamp,
    )
    .await?;

    let deleted = sqlx::query("DELETE FROM account_transfers WHERE id = ?")
        .bind(transfer_id.as_i64())
        .execute(&mut *tx)
        .await?;

    if deleted.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    tx.commit().await?;
    Ok(())
}

pub async fn list_transfers(pool: &SqlitePool) -> Result<Vec<TransferRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, from_account_id, to_account_id,
               from_currency, from_amount, to_currency, to_amount,
               transfer_date, notes, created_at
        FROM account_transfers
        ORDER BY transfer_date DESC, created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_transfer_row).collect()
}

fn map_transfer_row(row: sqlx::sqlite::SqliteRow) -> Result<TransferRecord, StorageError> {
    Ok(TransferRecord {
        id: TransferId::try_from(row.get::<i64, _>("id"))?,
        from_account_id: AccountId::try_from(row.get::<i64, _>("from_account_id"))?,
        to_account_id: AccountId::try_from(row.get::<i64, _>("to_account_id"))?,
        from_currency: Currency::try_from(row.get::<&str, _>("from_currency"))?,
        from_amount: Amount::from_scaled_i64(row.get::<i64, _>("from_amount")),
        to_currency: Currency::try_from(row.get::<&str, _>("to_currency"))?,
        to_amount: Amount::from_scaled_i64(row.get::<i64, _>("to_amount")),
        transfer_date: TradeDate::try_from(row.get::<&str, _>("transfer_date"))?,
        notes: row.get("notes"),
        created_at: row.get("created_at"),
    })
}
