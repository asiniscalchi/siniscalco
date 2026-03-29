use sqlx::{Row, SqlitePool};

use crate::format_decimal_amount;
use crate::storage::records::*;
use crate::storage::{
    AccountId, Amount, Currency, StorageError, TradeDate, TransferId,
    current_utc_timestamp_iso8601,
};

pub async fn create_transfer(
    pool: &SqlitePool,
    input: CreateTransferInput,
) -> Result<TransferRecord, StorageError> {
    if input.from_account_id == input.to_account_id {
        return Err(StorageError::Validation(
            "from and to accounts must be different",
        ));
    }

    let mut connection = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *connection)
        .await?;

    let result = async {
        let timestamp = current_utc_timestamp_iso8601()?;

        // Debit the source account
        let from_balance =
            load_balance_on_connection(&mut connection, input.from_account_id, input.from_currency)
                .await?;

        if from_balance < input.from_amount.as_decimal() {
            return Err(StorageError::Validation(
                "insufficient balance in source account for this transfer",
            ));
        }

        let new_from_balance = from_balance - input.from_amount.as_decimal();
        upsert_balance_on_connection(
            &mut connection,
            input.from_account_id,
            input.from_currency,
            new_from_balance,
            &timestamp,
        )
        .await?;

        // Credit the destination account
        let to_balance =
            load_balance_on_connection(&mut connection, input.to_account_id, input.to_currency)
                .await?;

        let new_to_balance = to_balance + input.to_amount.as_decimal();
        upsert_balance_on_connection(
            &mut connection,
            input.to_account_id,
            input.to_currency,
            new_to_balance,
            &timestamp,
        )
        .await?;

        // Insert the transfer record
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
        .execute(&mut *connection)
        .await?;

        let transfer_id = TransferId::try_from(result.last_insert_rowid())?;
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
        .fetch_one(&mut *connection)
        .await?;

        sqlx::query("COMMIT").execute(&mut *connection).await?;
        map_transfer_row(row)
    }
    .await;

    if result.is_err() {
        let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
    }

    result
}

pub async fn delete_transfer(
    pool: &SqlitePool,
    transfer_id: TransferId,
) -> Result<(), StorageError> {
    let mut connection = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *connection)
        .await?;

    let result = async {
        let timestamp = current_utc_timestamp_iso8601()?;

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
        .fetch_optional(&mut *connection)
        .await?
        .ok_or(StorageError::Database(sqlx::Error::RowNotFound))?;

        let transfer = map_transfer_row(row)?;

        // Reverse: credit the source account back
        let from_balance = load_balance_on_connection(
            &mut connection,
            transfer.from_account_id,
            transfer.from_currency,
        )
        .await?;
        let new_from_balance = from_balance + transfer.from_amount.as_decimal();
        upsert_balance_on_connection(
            &mut connection,
            transfer.from_account_id,
            transfer.from_currency,
            new_from_balance,
            &timestamp,
        )
        .await?;

        // Reverse: debit the destination account
        let to_balance = load_balance_on_connection(
            &mut connection,
            transfer.to_account_id,
            transfer.to_currency,
        )
        .await?;
        if to_balance < transfer.to_amount.as_decimal() {
            return Err(StorageError::Validation(
                "insufficient balance in destination account to reverse this transfer",
            ));
        }
        let new_to_balance = to_balance - transfer.to_amount.as_decimal();
        upsert_balance_on_connection(
            &mut connection,
            transfer.to_account_id,
            transfer.to_currency,
            new_to_balance,
            &timestamp,
        )
        .await?;

        let deleted = sqlx::query("DELETE FROM account_transfers WHERE id = ?")
            .bind(transfer_id.as_i64())
            .execute(&mut *connection)
            .await?;

        if deleted.rows_affected() == 0 {
            return Err(StorageError::Database(sqlx::Error::RowNotFound));
        }

        sqlx::query("COMMIT").execute(&mut *connection).await?;
        Ok(())
    }
    .await;

    if result.is_err() {
        let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
    }

    result
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

async fn load_balance_on_connection(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    account_id: AccountId,
    currency: Currency,
) -> Result<rust_decimal::Decimal, StorageError> {
    let amount = sqlx::query_scalar::<_, i64>(
        "SELECT amount FROM account_balances WHERE account_id = ? AND currency = ?",
    )
    .bind(account_id.as_i64())
    .bind(currency.as_str())
    .fetch_optional(&mut **connection)
    .await?;
    Ok(amount
        .map(|v| Amount::from_scaled_i64(v).as_decimal())
        .unwrap_or(rust_decimal::Decimal::ZERO))
}

async fn upsert_balance_on_connection(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    account_id: AccountId,
    currency: Currency,
    new_amount: rust_decimal::Decimal,
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
    .execute(&mut **connection)
    .await?;
    Ok(())
}
