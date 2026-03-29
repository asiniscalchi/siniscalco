use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

use crate::format_decimal_amount;
use crate::storage::records::*;
use crate::storage::{
    AccountId, Amount, AssetId, AssetPosition, AssetQuantity, AssetTransactionType, AssetUnitPrice,
    Currency, FxRate, StorageError, TradeDate, current_utc_timestamp_iso8601,
};

pub async fn create_asset_transaction(
    pool: &SqlitePool,
    input: CreateAssetTransactionInput,
) -> Result<AssetTransactionRecord, StorageError> {
    let mut connection = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *connection)
        .await?;

    let result = async {
        if input.transaction_type == AssetTransactionType::Sell {
            let current_quantity =
                load_current_quantity(&mut connection, input.account_id, input.asset_id).await?;

            if current_quantity < input.quantity.as_decimal() {
                return Err(StorageError::Validation(
                    "sell transaction would make position negative",
                ));
            }
        }

        let timestamp = current_utc_timestamp_iso8601()?;
        let result = sqlx::query(
            r#"
            INSERT INTO asset_transactions (
                account_id,
                asset_id,
                transaction_type,
                trade_date,
                quantity,
                unit_price,
                currency_code,
                notes,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(input.account_id.as_i64())
        .bind(input.asset_id.as_i64())
        .bind(input.transaction_type.as_str())
        .bind(input.trade_date.as_str())
        .bind(input.quantity.as_scaled_i64())
        .bind(input.unit_price.as_scaled_i64())
        .bind(input.currency_code.as_str())
        .bind(input.notes.as_deref())
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *connection)
        .await?;

        apply_cash_impact(
            &mut connection,
            input.account_id,
            input.transaction_type,
            input.quantity,
            input.unit_price,
            input.currency_code,
            &timestamp,
        )
        .await?;

        let transaction_id = result.last_insert_rowid();
        let row = sqlx::query(
            r#"
            SELECT id, account_id, asset_id, transaction_type, trade_date, quantity, unit_price,
                   currency_code, notes, created_at, updated_at
            FROM asset_transactions
            WHERE id = ?
            "#,
        )
        .bind(transaction_id)
        .fetch_one(&mut *connection)
        .await?;

        sqlx::query("COMMIT").execute(&mut *connection).await?;
        map_transaction_row(row)
    }
    .await;

    if result.is_err() {
        let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
    }

    result
}

pub async fn list_asset_transactions(
    pool: &SqlitePool,
    account_id: AccountId,
) -> Result<Vec<AssetTransactionRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, account_id, asset_id, transaction_type, trade_date, quantity, unit_price,
               currency_code, notes, created_at, updated_at
        FROM asset_transactions
        WHERE account_id = ?
        ORDER BY trade_date DESC, created_at DESC, id DESC
        "#,
    )
    .bind(account_id.as_i64())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_transaction_row).collect()
}

pub async fn list_transactions(
    pool: &SqlitePool,
) -> Result<Vec<AssetTransactionRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, account_id, asset_id, transaction_type, trade_date, quantity, unit_price,
               currency_code, notes, created_at, updated_at
        FROM asset_transactions
        ORDER BY trade_date DESC, created_at DESC, id DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_transaction_row).collect()
}

pub async fn get_transaction(
    pool: &SqlitePool,
    transaction_id: i64,
) -> Result<AssetTransactionRecord, StorageError> {
    let mut connection = pool.acquire().await?;
    get_asset_transaction(&mut connection, transaction_id).await
}

pub async fn update_asset_transaction(
    pool: &SqlitePool,
    transaction_id: i64,
    input: UpdateAssetTransactionInput,
) -> Result<AssetTransactionRecord, StorageError> {
    let mut connection = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *connection)
        .await?;

    let result = async {
        let existing_transaction = get_asset_transaction(&mut connection, transaction_id).await?;
        validate_position_change_for_transaction_update(
            &mut connection,
            &existing_transaction,
            &input,
        )
        .await?;

        let updated_at = current_utc_timestamp_iso8601()?;

        reverse_cash_impact(
            &mut connection,
            existing_transaction.account_id,
            existing_transaction.transaction_type,
            existing_transaction.quantity,
            existing_transaction.unit_price,
            existing_transaction.currency_code,
            &updated_at,
        )
        .await?;

        apply_cash_impact(
            &mut connection,
            input.account_id,
            input.transaction_type,
            input.quantity,
            input.unit_price,
            input.currency_code,
            &updated_at,
        )
        .await?;

        let result = sqlx::query(
            r#"
            UPDATE asset_transactions
            SET account_id = ?,
                asset_id = ?,
                transaction_type = ?,
                trade_date = ?,
                quantity = ?,
                unit_price = ?,
                currency_code = ?,
                notes = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(input.account_id.as_i64())
        .bind(input.asset_id.as_i64())
        .bind(input.transaction_type.as_str())
        .bind(input.trade_date.as_str())
        .bind(input.quantity.as_scaled_i64())
        .bind(input.unit_price.as_scaled_i64())
        .bind(input.currency_code.as_str())
        .bind(input.notes.as_deref())
        .bind(&updated_at)
        .bind(transaction_id)
        .execute(&mut *connection)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::Database(sqlx::Error::RowNotFound));
        }

        let transaction = get_asset_transaction(&mut connection, transaction_id).await?;

        sqlx::query("COMMIT").execute(&mut *connection).await?;
        Ok(transaction)
    }
    .await;

    if result.is_err() {
        let _ = sqlx::query("ROLLBACK").execute(&mut *connection).await;
    }

    result
}

pub async fn delete_asset_transaction(
    pool: &SqlitePool,
    transaction_id: i64,
) -> Result<(), StorageError> {
    let mut connection = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *connection)
        .await?;

    let result = async {
        let existing_transaction = get_asset_transaction(&mut connection, transaction_id).await?;
        validate_position_change_for_transaction_delete(&mut connection, &existing_transaction)
            .await?;

        let updated_at = current_utc_timestamp_iso8601()?;
        reverse_cash_impact(
            &mut connection,
            existing_transaction.account_id,
            existing_transaction.transaction_type,
            existing_transaction.quantity,
            existing_transaction.unit_price,
            existing_transaction.currency_code,
            &updated_at,
        )
        .await?;

        let result = sqlx::query("DELETE FROM asset_transactions WHERE id = ?")
            .bind(transaction_id)
            .execute(&mut *connection)
            .await?;

        if result.rows_affected() == 0 {
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

pub async fn list_account_positions(
    pool: &SqlitePool,
    account_id: AccountId,
) -> Result<Vec<AssetPositionRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT asset_id, transaction_type, quantity
        FROM asset_transactions
        WHERE account_id = ?
        ORDER BY asset_id
        "#,
    )
    .bind(account_id.as_i64())
    .fetch_all(pool)
    .await?;

    let mut positions_by_asset = BTreeMap::<AssetId, Decimal>::new();

    for row in rows {
        let asset_id = AssetId::try_from(row.get::<i64, _>("asset_id"))?;
        let transaction_type =
            AssetTransactionType::try_from(row.get::<&str, _>("transaction_type"))?;
        let quantity = AssetQuantity::from_scaled_i64(row.get::<i64, _>("quantity"))?.as_decimal();
        let signed_quantity = signed_quantity_delta(transaction_type, quantity);

        positions_by_asset
            .entry(asset_id)
            .and_modify(|current_quantity| *current_quantity += signed_quantity)
            .or_insert(signed_quantity);
    }

    positions_by_asset
        .into_iter()
        .filter(|(_, quantity)| *quantity > Decimal::ZERO)
        .map(|(asset_id, quantity)| {
            Ok(AssetPositionRecord {
                account_id,
                asset_id,
                quantity: AssetPosition::try_from(quantity)?,
            })
        })
        .collect()
}

async fn load_current_quantity(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    account_id: AccountId,
    asset_id: AssetId,
) -> Result<Decimal, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT transaction_type, quantity
        FROM asset_transactions
        WHERE account_id = ? AND asset_id = ?
        "#,
    )
    .bind(account_id.as_i64())
    .bind(asset_id.as_i64())
    .fetch_all(&mut **connection)
    .await?;

    rows.into_iter()
        .try_fold(Decimal::ZERO, |current_quantity, row| {
            let transaction_type =
                AssetTransactionType::try_from(row.get::<&str, _>("transaction_type"))?;
            let quantity =
                AssetQuantity::from_scaled_i64(row.get::<i64, _>("quantity"))?.as_decimal();

            Ok(current_quantity + signed_quantity_delta(transaction_type, quantity))
        })
}

async fn get_asset_transaction(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    transaction_id: i64,
) -> Result<AssetTransactionRecord, StorageError> {
    let row = sqlx::query(
        r#"
        SELECT id, account_id, asset_id, transaction_type, trade_date, quantity, unit_price,
               currency_code, notes, created_at, updated_at
        FROM asset_transactions
        WHERE id = ?
        "#,
    )
    .bind(transaction_id)
    .fetch_one(&mut **connection)
    .await?;

    map_transaction_row(row)
}

async fn validate_position_change_for_transaction_update(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    existing_transaction: &AssetTransactionRecord,
    input: &UpdateAssetTransactionInput,
) -> Result<(), StorageError> {
    let old_delta = signed_quantity_delta(
        existing_transaction.transaction_type,
        existing_transaction.quantity.as_decimal(),
    );
    let new_delta = signed_quantity_delta(input.transaction_type, input.quantity.as_decimal());

    if existing_transaction.account_id == input.account_id
        && existing_transaction.asset_id == input.asset_id
    {
        let current_quantity =
            load_current_quantity(connection, input.account_id, input.asset_id).await?;
        if current_quantity - old_delta + new_delta < Decimal::ZERO {
            return Err(StorageError::Validation(
                "sell transaction would make position negative",
            ));
        }

        return Ok(());
    }

    let old_quantity = load_current_quantity(
        connection,
        existing_transaction.account_id,
        existing_transaction.asset_id,
    )
    .await?;
    if old_quantity - old_delta < Decimal::ZERO {
        return Err(StorageError::Validation(
            "sell transaction would make position negative",
        ));
    }

    let new_quantity = load_current_quantity(connection, input.account_id, input.asset_id).await?;
    if new_quantity + new_delta < Decimal::ZERO {
        return Err(StorageError::Validation(
            "sell transaction would make position negative",
        ));
    }

    Ok(())
}

async fn validate_position_change_for_transaction_delete(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    existing_transaction: &AssetTransactionRecord,
) -> Result<(), StorageError> {
    let current_quantity = load_current_quantity(
        connection,
        existing_transaction.account_id,
        existing_transaction.asset_id,
    )
    .await?;
    let delta = signed_quantity_delta(
        existing_transaction.transaction_type,
        existing_transaction.quantity.as_decimal(),
    );

    if current_quantity - delta < Decimal::ZERO {
        return Err(StorageError::Validation(
            "sell transaction would make position negative",
        ));
    }

    Ok(())
}

fn signed_quantity_delta(transaction_type: AssetTransactionType, quantity: Decimal) -> Decimal {
    match transaction_type {
        AssetTransactionType::Buy => quantity,
        AssetTransactionType::Sell => -quantity,
    }
}

async fn apply_cash_impact(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    account_id: AccountId,
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    currency: Currency,
    updated_at: &str,
) -> Result<(), StorageError> {
    let base_currency = load_account_base_currency(connection, account_id).await?;
    let transaction_amount = quantity.as_decimal() * unit_price.as_decimal();
    let fx_rate = get_fx_rate_on_connection(connection, currency, base_currency)
        .await?
        .ok_or(StorageError::Validation(
            "fx rate not available for transaction currency conversion",
        ))?;
    let base_amount = transaction_amount * fx_rate;
    let current_balance = load_balance_on_connection(connection, account_id, base_currency).await?;

    let new_balance = match transaction_type {
        AssetTransactionType::Buy => {
            if current_balance < base_amount {
                return Err(StorageError::Validation(
                    "insufficient cash balance for this buy transaction",
                ));
            }
            current_balance - base_amount
        }
        AssetTransactionType::Sell => current_balance + base_amount,
    };

    upsert_balance_on_connection(
        connection,
        account_id,
        base_currency,
        new_balance,
        updated_at,
    )
    .await
}

async fn reverse_cash_impact(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    account_id: AccountId,
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    currency: Currency,
    updated_at: &str,
) -> Result<(), StorageError> {
    let reversed = match transaction_type {
        AssetTransactionType::Buy => AssetTransactionType::Sell,
        AssetTransactionType::Sell => AssetTransactionType::Buy,
    };
    apply_cash_impact(
        connection, account_id, reversed, quantity, unit_price, currency, updated_at,
    )
    .await
}

async fn load_account_base_currency(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    account_id: AccountId,
) -> Result<Currency, StorageError> {
    let currency_str =
        sqlx::query_scalar::<_, String>("SELECT base_currency FROM accounts WHERE id = ?")
            .bind(account_id.as_i64())
            .fetch_one(&mut **connection)
            .await?;
    Currency::try_from(currency_str.as_str())
}

async fn get_fx_rate_on_connection(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    from_currency: Currency,
    to_currency: Currency,
) -> Result<Option<Decimal>, StorageError> {
    if from_currency == to_currency {
        return Ok(Some(Decimal::ONE));
    }
    let rate = sqlx::query_scalar::<_, i64>(
        "SELECT rate FROM fx_rates WHERE from_currency = ? AND to_currency = ?",
    )
    .bind(from_currency.as_str())
    .bind(to_currency.as_str())
    .fetch_optional(&mut **connection)
    .await?;
    rate.map(|value| FxRate::from_scaled_i64(value).map(|r| r.as_decimal()))
        .transpose()
}

async fn load_balance_on_connection(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
    account_id: AccountId,
    currency: Currency,
) -> Result<Decimal, StorageError> {
    let amount = sqlx::query_scalar::<_, i64>(
        "SELECT amount FROM account_balances WHERE account_id = ? AND currency = ?",
    )
    .bind(account_id.as_i64())
    .bind(currency.as_str())
    .fetch_optional(&mut **connection)
    .await?;
    Ok(amount
        .map(|v| Amount::from_scaled_i64(v).as_decimal())
        .unwrap_or(Decimal::ZERO))
}

async fn upsert_balance_on_connection(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
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
    .execute(&mut **connection)
    .await?;
    Ok(())
}

fn map_transaction_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<AssetTransactionRecord, StorageError> {
    Ok(AssetTransactionRecord {
        id: row.get("id"),
        account_id: AccountId::try_from(row.get::<i64, _>("account_id"))?,
        asset_id: AssetId::try_from(row.get::<i64, _>("asset_id"))?,
        transaction_type: AssetTransactionType::try_from(row.get::<&str, _>("transaction_type"))?,
        trade_date: TradeDate::try_from(row.get::<&str, _>("trade_date"))?,
        quantity: AssetQuantity::from_scaled_i64(row.get::<i64, _>("quantity"))?,
        unit_price: AssetUnitPrice::from_scaled_i64(row.get::<i64, _>("unit_price"))?,
        currency_code: Currency::try_from(row.get::<&str, _>("currency_code"))?,
        notes: row.get::<Option<String>, _>("notes"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
