/// Cash-impact helpers used within asset transaction mutations.
///
/// All functions in this module operate on an already-open `SqliteConnection`
/// (inside an active transaction) and must not start their own.
use rust_decimal::Decimal;
use sqlx::sqlite::SqliteConnection;

use crate::format_decimal_amount;
use crate::storage::{
    AccountId, Amount, AssetQuantity, AssetTransactionType, AssetUnitPrice, Currency, FxRate,
    StorageError,
};

pub(super) async fn apply_cash_impact(
    connection: &mut SqliteConnection,
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

pub(super) async fn reverse_cash_impact(
    connection: &mut SqliteConnection,
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

async fn get_fx_rate_on_connection(
    connection: &mut SqliteConnection,
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
    .fetch_optional(&mut *connection)
    .await?;
    rate.map(|value| FxRate::from_scaled_i64(value).map(|r| r.as_decimal()))
        .transpose()
}

async fn load_balance_on_connection(
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

async fn upsert_balance_on_connection(
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
