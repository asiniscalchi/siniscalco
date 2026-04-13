/// Cash-impact helpers used within asset transaction mutations.
///
/// All functions in this module operate on an already-open `SqliteConnection`
/// (inside an active transaction) and must not start their own.
use rust_decimal::Decimal;
use sqlx::sqlite::SqliteConnection;

use crate::storage::{
    AccountId, AssetQuantity, AssetTransactionType, AssetUnitPrice, Currency, FxRate, StorageError,
};

use super::balances::{CashEntrySource, insert_cash_entry_on_connection};

/// Apply the cash impact of a transaction to the account balance.
///
/// Returns the FX rate that was used so the caller can persist it on the
/// transaction row for correct future reversals.
#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_cash_impact(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    currency: Currency,
    transaction_id: i64,
    created_at: &str,
) -> Result<FxRate, StorageError> {
    let base_currency = load_account_base_currency(connection, account_id).await?;
    let rate = resolve_fx_rate(connection, currency, base_currency).await?;
    let delta = compute_delta(transaction_type, quantity, unit_price, rate);
    insert_cash_entry_on_connection(
        connection,
        account_id,
        base_currency,
        delta,
        CashEntrySource::AssetTransaction,
        Some(transaction_id),
        created_at,
    )
    .await?;
    Ok(rate)
}

/// Apply the cash impact of a transaction using an already-known rate.
///
/// Used when updating a transaction whose currency has not changed: the
/// correction should preserve the original trade's FX conditions so that
/// only the price/quantity delta affects the balance, not FX drift.
#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_cash_impact_at_rate(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    locked_rate: FxRate,
    transaction_id: i64,
    created_at: &str,
) -> Result<(), StorageError> {
    let base_currency = load_account_base_currency(connection, account_id).await?;
    let delta = compute_delta(transaction_type, quantity, unit_price, locked_rate);
    insert_cash_entry_on_connection(
        connection,
        account_id,
        base_currency,
        delta,
        CashEntrySource::AssetTransaction,
        Some(transaction_id),
        created_at,
    )
    .await
}

/// Reverse the cash impact of a past transaction using the rate that was
/// locked in at execution time.
///
/// The caller must pass `locked_rate` from the stored `fx_rate` column on
/// the original transaction row. The live `fx_rates` table is not consulted,
/// so FX drift between trade date and the reversal date has no effect.
#[allow(clippy::too_many_arguments)]
pub(super) async fn reverse_cash_impact(
    connection: &mut SqliteConnection,
    account_id: AccountId,
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    locked_rate: FxRate,
    transaction_id: i64,
    created_at: &str,
) -> Result<(), StorageError> {
    let base_currency = load_account_base_currency(connection, account_id).await?;
    let reversed_type = match transaction_type {
        AssetTransactionType::Buy => AssetTransactionType::Sell,
        AssetTransactionType::Sell => AssetTransactionType::Buy,
    };
    let delta = compute_delta(reversed_type, quantity, unit_price, locked_rate);
    insert_cash_entry_on_connection(
        connection,
        account_id,
        base_currency,
        delta,
        CashEntrySource::AssetTransaction,
        Some(transaction_id),
        created_at,
    )
    .await
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn compute_delta(
    transaction_type: AssetTransactionType,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    rate: FxRate,
) -> Decimal {
    let amount = quantity.as_decimal() * unit_price.as_decimal() * rate.as_decimal();
    match transaction_type {
        AssetTransactionType::Buy => -amount,
        AssetTransactionType::Sell => amount,
    }
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
