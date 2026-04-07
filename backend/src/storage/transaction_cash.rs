/// Cash-impact helpers used within asset transaction mutations.
///
/// All functions in this module operate on an already-open `SqliteConnection`
/// (inside an active transaction) and must not start their own.
use rust_decimal::Decimal;
use sqlx::sqlite::SqliteConnection;

use crate::storage::{
    AccountId, AssetQuantity, AssetTransactionType, AssetUnitPrice, Currency, FxRate, StorageError,
};

use super::balances::{load_balance_on_connection, upsert_balance_on_connection};

/// Apply the cash impact of a transaction to the account balance.
///
/// Returns the FX rate that was used so the caller can persist it on the
/// transaction row for correct future reversals.
pub(super) async fn apply_cash_impact(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    currency: Currency,
    updated_at: &str,
) -> Result<FxRate, StorageError> {
    let base_currency = load_account_base_currency(connection, account_id).await?;
    let rate = resolve_fx_rate(connection, currency, base_currency).await?;
    apply_with_rate(
        connection,
        account_id,
        transaction_type,
        quantity,
        unit_price,
        base_currency,
        rate,
        updated_at,
    )
    .await?;
    Ok(rate)
}

/// Apply the cash impact of a transaction using an already-known rate.
///
/// Used when updating a transaction whose currency has not changed: the
/// correction should preserve the original trade's FX conditions so that
/// only the price/quantity delta affects the balance, not FX drift.
pub(super) async fn apply_cash_impact_at_rate(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    locked_rate: FxRate,
    updated_at: &str,
) -> Result<(), StorageError> {
    let base_currency = load_account_base_currency(connection, account_id).await?;
    apply_with_rate(
        connection,
        account_id,
        transaction_type,
        quantity,
        unit_price,
        base_currency,
        locked_rate,
        updated_at,
    )
    .await
}

/// Reverse the cash impact of a past transaction using the rate that was
/// locked in at execution time.
///
/// The caller must pass `locked_rate` from the stored `fx_rate` column on
/// the original transaction row. The live `fx_rates` table is not consulted,
/// so FX drift between trade date and the reversal date has no effect.
pub(super) async fn reverse_cash_impact(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    locked_rate: FxRate,
    updated_at: &str,
) -> Result<(), StorageError> {
    let base_currency = load_account_base_currency(connection, account_id).await?;
    let reversed_type = match transaction_type {
        AssetTransactionType::Buy => AssetTransactionType::Sell,
        AssetTransactionType::Sell => AssetTransactionType::Buy,
    };
    apply_with_rate(
        connection,
        account_id,
        reversed_type,
        quantity,
        unit_price,
        base_currency,
        locked_rate,
        updated_at,
    )
    .await
}

// ── Private helpers ───────────────────────────────────────────────────────────

async fn apply_with_rate(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    base_currency: Currency,
    rate: FxRate,
    updated_at: &str,
) -> Result<(), StorageError> {
    let transaction_amount = quantity.as_decimal() * unit_price.as_decimal();
    let base_amount = transaction_amount * rate.as_decimal();
    let current_balance =
        load_balance_on_connection(&mut *connection, account_id, base_currency).await?;

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
        &mut *connection,
        account_id,
        base_currency,
        new_balance,
        updated_at,
    )
    .await
}

async fn resolve_fx_rate(
    connection: &mut SqliteConnection,
    from_currency: Currency,
    to_currency: Currency,
) -> Result<FxRate, StorageError> {
    if from_currency == to_currency {
        return Ok(FxRate::from_scaled_i64(1_000_000).unwrap());
    }

    sqlx::query_scalar::<_, i64>(
        "SELECT rate FROM fx_rates WHERE from_currency = ? AND to_currency = ?",
    )
    .bind(from_currency.as_str())
    .bind(to_currency.as_str())
    .fetch_optional(&mut *connection)
    .await?
    .map(FxRate::from_scaled_i64)
    .transpose()?
    .ok_or(StorageError::Validation(
        "fx rate not available for transaction currency conversion",
    ))
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
