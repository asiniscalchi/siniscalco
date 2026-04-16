use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool, sqlite::SqliteConnection};

use crate::storage::records::*;
use crate::storage::{
    AccountId, AssetId, AssetPosition, AssetQuantity, AssetTransactionType, AssetUnitPrice,
    Currency, FxRate, StorageError, TradeDate, current_utc_timestamp,
};

use super::portfolio_snapshots::recalculate_snapshots_from_date;
use super::transaction_cash::{
    CashImpactContext, apply_cash_impact, apply_cash_impact_at_rate, reverse_cash_impact,
};

pub async fn create_asset_transaction(
    pool: &SqlitePool,
    input: CreateAssetTransactionInput,
) -> Result<AssetTransactionRecord, StorageError> {
    let mut tx = pool.begin().await?;

    if input.transaction_type == AssetTransactionType::Sell {
        let current_quantity =
            load_current_quantity(&mut tx, input.account_id, input.asset_id).await?;

        if current_quantity < input.quantity.as_decimal() {
            return Err(StorageError::Validation(
                "sell transaction would make position negative",
            ));
        }
    }

    if input.transaction_type == AssetTransactionType::Opening {
        let current_quantity =
            load_current_quantity(&mut tx, input.account_id, input.asset_id).await?;

        if current_quantity > rust_decimal::Decimal::ZERO {
            return Err(StorageError::Validation(
                "opening position can only be recorded when there is no existing position",
            ));
        }
    }

    let timestamp = current_utc_timestamp()?;

    // Insert the row with a placeholder fx_rate of 1; we will update it once
    // we know the transaction_id and have applied the cash impact.
    let placeholder_rate = FxRate::from_scaled_i64(1_000_000).unwrap();
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
            fx_rate,
            notes,
            created_at,
            updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(input.account_id.as_i64())
    .bind(input.asset_id.as_i64())
    .bind(input.transaction_type.as_str())
    .bind(input.trade_date.as_str())
    .bind(input.quantity.as_scaled_i64())
    .bind(input.unit_price.as_scaled_i64())
    .bind(input.currency_code.as_str())
    .bind(placeholder_rate.as_scaled_i64())
    .bind(input.notes.as_deref())
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(&mut *tx)
    .await?;

    let transaction_id = result.last_insert_rowid();

    // Opening positions do not affect cash; skip the impact and leave the
    // placeholder fx_rate of 1 in place.
    if input.transaction_type.has_cash_impact() {
        let fx_rate = apply_cash_impact(
            &mut tx,
            &CashImpactContext {
                account_id: input.account_id,
                transaction_type: input.transaction_type,
                quantity: input.quantity,
                unit_price: input.unit_price,
                transaction_id,
                created_at: &timestamp,
            },
            input.currency_code,
        )
        .await?;

        sqlx::query("UPDATE asset_transactions SET fx_rate = ? WHERE id = ?")
            .bind(fx_rate.as_scaled_i64())
            .bind(transaction_id)
            .execute(&mut *tx)
            .await?;
    }
    let row = sqlx::query(
        r#"
        SELECT id, account_id, asset_id, transaction_type, trade_date, quantity, unit_price,
               currency_code, fx_rate, notes, created_at, updated_at
        FROM asset_transactions
        WHERE id = ?
        "#,
    )
    .bind(transaction_id)
    .fetch_one(&mut *tx)
    .await?;

    let record = map_transaction_row(row)?;
    let trade_date = input.trade_date.as_str().to_owned();
    tx.commit().await?;

    if let Err(error) =
        recalculate_snapshots_from_date(pool, &trade_date, crate::fx_refresh::PRODUCT_BASE_CURRENCY)
            .await
    {
        tracing::warn!(error = %error, "snapshot recalculation failed after transaction create");
    }

    Ok(record)
}

pub async fn list_asset_transactions(
    pool: &SqlitePool,
    account_id: AccountId,
) -> Result<Vec<AssetTransactionRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT id, account_id, asset_id, transaction_type, trade_date, quantity, unit_price,
               currency_code, fx_rate, notes, created_at, updated_at
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
               currency_code, fx_rate, notes, created_at, updated_at
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
    input: CreateAssetTransactionInput,
) -> Result<AssetTransactionRecord, StorageError> {
    let mut tx = pool.begin().await?;

    let existing_transaction = get_asset_transaction(&mut tx, transaction_id).await?;
    validate_position_change_for_transaction_update(&mut tx, &existing_transaction, &input).await?;
    let existing_base_currency =
        load_account_base_currency(&mut tx, existing_transaction.account_id).await?;
    let updated_base_currency = load_account_base_currency(&mut tx, input.account_id).await?;

    let updated_at = current_utc_timestamp()?;

    // Reverse the original cash impact using the rate stored at trade time.
    // Opening positions have no cash impact so nothing to reverse.
    if existing_transaction.transaction_type.has_cash_impact() {
        reverse_cash_impact(
            &mut tx,
            &CashImpactContext {
                account_id: existing_transaction.account_id,
                transaction_type: existing_transaction.transaction_type,
                quantity: existing_transaction.quantity,
                unit_price: existing_transaction.unit_price,
                transaction_id,
                created_at: &updated_at,
            },
            existing_transaction.fx_rate,
        )
        .await?;
    }

    // Apply the new cash impact.
    //
    // If the transaction currency is unchanged, use the stored rate so that
    // a correction (price / quantity fix) is evaluated at the original trade's
    // FX conditions rather than today's rate. This keeps no-op updates truly
    // neutral and avoids injecting FX drift through data-entry corrections.
    //
    // If the currency changes, resolve the current live rate for the new pair.
    // Opening positions have no cash impact so the fx_rate stays as the placeholder.
    let new_fx_rate = if !input.transaction_type.has_cash_impact() {
        FxRate::from_scaled_i64(1_000_000).unwrap()
    } else {
        let can_reuse_locked_rate = input.currency_code == existing_transaction.currency_code
            && existing_base_currency == updated_base_currency
            && existing_transaction.transaction_type.has_cash_impact();

        if can_reuse_locked_rate {
            apply_cash_impact_at_rate(
                &mut tx,
                &CashImpactContext {
                    account_id: input.account_id,
                    transaction_type: input.transaction_type,
                    quantity: input.quantity,
                    unit_price: input.unit_price,
                    transaction_id,
                    created_at: &updated_at,
                },
                existing_transaction.fx_rate,
            )
            .await?;
            existing_transaction.fx_rate
        } else {
            apply_cash_impact(
                &mut tx,
                &CashImpactContext {
                    account_id: input.account_id,
                    transaction_type: input.transaction_type,
                    quantity: input.quantity,
                    unit_price: input.unit_price,
                    transaction_id,
                    created_at: &updated_at,
                },
                input.currency_code,
            )
            .await?
        }
    };

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
            fx_rate = ?,
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
    .bind(new_fx_rate.as_scaled_i64())
    .bind(input.notes.as_deref())
    .bind(&updated_at)
    .bind(transaction_id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    let transaction = get_asset_transaction(&mut tx, transaction_id).await?;
    tx.commit().await?;

    // Recalculate snapshots from the earliest affected date.
    let earliest_date = std::cmp::min(
        existing_transaction.trade_date.as_str().to_owned(),
        input.trade_date.as_str().to_owned(),
    );
    if let Err(error) = recalculate_snapshots_from_date(
        pool,
        &earliest_date,
        crate::fx_refresh::PRODUCT_BASE_CURRENCY,
    )
    .await
    {
        tracing::warn!(error = %error, "snapshot recalculation failed after transaction update");
    }

    Ok(transaction)
}

pub async fn delete_asset_transaction(
    pool: &SqlitePool,
    transaction_id: i64,
) -> Result<(), StorageError> {
    let mut tx = pool.begin().await?;

    let existing_transaction = get_asset_transaction(&mut tx, transaction_id).await?;
    validate_position_change_for_transaction_delete(&mut tx, &existing_transaction).await?;

    let updated_at = current_utc_timestamp()?;

    // Reverse the cash impact using the rate that was locked in at trade time.
    // Opening positions have no cash impact so nothing to reverse.
    if existing_transaction.transaction_type.has_cash_impact() {
        reverse_cash_impact(
            &mut tx,
            &CashImpactContext {
                account_id: existing_transaction.account_id,
                transaction_type: existing_transaction.transaction_type,
                quantity: existing_transaction.quantity,
                unit_price: existing_transaction.unit_price,
                transaction_id,
                created_at: &updated_at,
            },
            existing_transaction.fx_rate,
        )
        .await?;
    }

    let result = sqlx::query("DELETE FROM asset_transactions WHERE id = ?")
        .bind(transaction_id)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    let trade_date = existing_transaction.trade_date.as_str().to_owned();
    tx.commit().await?;

    if let Err(error) =
        recalculate_snapshots_from_date(pool, &trade_date, crate::fx_refresh::PRODUCT_BASE_CURRENCY)
            .await
    {
        tracing::warn!(error = %error, "snapshot recalculation failed after transaction delete");
    }

    Ok(())
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

pub(crate) async fn list_account_positions_as_of(
    pool: &SqlitePool,
    account_id: AccountId,
    as_of: &str,
) -> Result<Vec<AssetPositionRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT asset_id, transaction_type, quantity
        FROM asset_transactions
        WHERE account_id = ? AND trade_date <= ?
        ORDER BY asset_id
        "#,
    )
    .bind(account_id.as_i64())
    .bind(as_of)
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

// ── Private helpers ───────────────────────────────────────────────────────────

async fn load_current_quantity(
    connection: &mut SqliteConnection,
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
    .fetch_all(&mut *connection)
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
    connection: &mut SqliteConnection,
    transaction_id: i64,
) -> Result<AssetTransactionRecord, StorageError> {
    let row = sqlx::query(
        r#"
        SELECT id, account_id, asset_id, transaction_type, trade_date, quantity, unit_price,
               currency_code, fx_rate, notes, created_at, updated_at
        FROM asset_transactions
        WHERE id = ?
        "#,
    )
    .bind(transaction_id)
    .fetch_one(&mut *connection)
    .await?;

    map_transaction_row(row)
}

async fn validate_position_change_for_transaction_update(
    connection: &mut SqliteConnection,
    existing_transaction: &AssetTransactionRecord,
    input: &CreateAssetTransactionInput,
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
    connection: &mut SqliteConnection,
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
        AssetTransactionType::Buy | AssetTransactionType::Opening => quantity,
        AssetTransactionType::Sell => -quantity,
    }
}

async fn load_account_base_currency(
    connection: &mut SqliteConnection,
    account_id: AccountId,
) -> Result<Currency, StorageError> {
    let currency_str =
        sqlx::query_scalar::<_, String>("SELECT base_currency FROM accounts WHERE id = ?")
            .bind(account_id.as_i64())
            .fetch_one(&mut *connection)
            .await?;
    Currency::try_from(currency_str.as_str())
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
        fx_rate: FxRate::from_scaled_i64(row.get::<i64, _>("fx_rate"))?,
        notes: row.get::<Option<String>, _>("notes"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
