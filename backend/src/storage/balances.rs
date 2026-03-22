use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};
use time::OffsetDateTime;

use crate::storage::accounts::validate_allowed_currency;
use crate::storage::fx::get_direct_fx_rate;
use crate::storage::models::*;

pub async fn upsert_account_balance(
    pool: &SqlitePool,
    input: UpsertAccountBalanceInput<'_>,
) -> Result<UpsertOutcome, StorageError> {
    validate_allowed_currency(pool, input.currency).await?;
    validate_decimal_20_8(input.amount)?;

    let updated_at = current_utc_timestamp()?;
    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM account_balances WHERE account_id = ? AND currency = ?)",
    )
    .bind(input.account_id)
    .bind(input.currency)
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
    .bind(input.account_id)
    .bind(input.currency)
    .bind(input.amount)
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
    account_id: i64,
) -> Result<Vec<AccountBalanceRecord>, StorageError> {
    let rows = sqlx::query(
        r#"
        SELECT
            account_id,
            currency,
            CAST(amount AS TEXT) AS amount,
            updated_at
        FROM account_balances
        WHERE account_id = ?
        ORDER BY currency
        "#,
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AccountBalanceRecord {
            account_id: row.get("account_id"),
            currency: row.get("currency"),
            amount: row.get("amount"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

pub async fn list_account_summaries(
    pool: &SqlitePool,
) -> Result<Vec<AccountSummaryRecord>, StorageError> {
    let accounts = crate::storage::accounts::list_accounts(pool).await?;
    let mut summaries = Vec::with_capacity(accounts.len());

    for account in accounts {
        let balances = list_account_balances(pool, account.id).await?;
        let summary = summarize_account(pool, &account, &balances).await?;

        summaries.push(AccountSummaryRecord {
            id: account.id,
            name: account.name,
            account_type: account.account_type,
            base_currency: account.base_currency,
            summary_status: summary.status,
            total_amount: summary.total_amount,
            total_currency: summary.total_currency,
        });
    }

    Ok(summaries)
}

pub(crate) struct AccountTotalSummaryInternal {
    pub(crate) status: AccountSummaryStatus,
    pub(crate) total_amount: Option<String>,
    pub(crate) total_currency: Option<String>,
}

async fn summarize_account(
    pool: &SqlitePool,
    account: &AccountRecord,
    balances: &[AccountBalanceRecord],
) -> Result<AccountTotalSummaryInternal, StorageError> {
    if balances.is_empty() {
        return Ok(AccountTotalSummaryInternal {
            status: AccountSummaryStatus::Ok,
            total_amount: Some("0.00000000".to_string()),
            total_currency: Some(account.base_currency.clone()),
        });
    }

    let mut total = Decimal::ZERO;

    for balance in balances {
        let amount = parse_stored_decimal(&balance.amount)?;

        if balance.currency == account.base_currency {
            total += amount;
            continue;
        }

        let Some(rate) =
            get_direct_fx_rate(pool, &balance.currency, &account.base_currency).await?
        else {
            return Ok(AccountTotalSummaryInternal {
                status: AccountSummaryStatus::ConversionUnavailable,
                total_amount: None,
                total_currency: None,
            });
        };

        total += amount * rate;
    }

    Ok(AccountTotalSummaryInternal {
        status: AccountSummaryStatus::Ok,
        total_amount: Some(crate::format_decimal_amount(total)),
        total_currency: Some(account.base_currency.clone()),
    })
}

pub(crate) fn parse_stored_decimal(value: &str) -> Result<Decimal, StorageError> {
    value
        .parse::<Decimal>()
        .map_err(|_| StorageError::Internal("stored decimal value is invalid"))
}

pub async fn delete_account_balance(
    pool: &SqlitePool,
    account_id: i64,
    currency: &str,
) -> Result<(), StorageError> {
    validate_allowed_currency(pool, currency).await?;

    let result = sqlx::query("DELETE FROM account_balances WHERE account_id = ? AND currency = ?")
        .bind(account_id)
        .bind(currency)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}

pub(crate) fn validate_decimal_20_8(amount: &str) -> Result<(), StorageError> {
    let amount = amount.strip_prefix('-').unwrap_or(amount);

    if amount.is_empty() {
        return Err(StorageError::Validation("amount must not be empty"));
    }

    let (integer_part, fractional_part) = match amount.split_once('.') {
        Some((integer_part, fractional_part)) => (integer_part, Some(fractional_part)),
        None => (amount, None),
    };

    if integer_part.is_empty() || !integer_part.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
    }

    if let Some(fractional_part) = fractional_part
        && (fractional_part.is_empty()
            || fractional_part.len() > 8
            || !fractional_part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
    }

    let total_digits = integer_part.len() + fractional_part.map_or(0, str::len);
    if total_digits > 20 || integer_part.len() > 12 {
        return Err(StorageError::Validation("amount must match DECIMAL(20,8)"));
    }

    Ok(())
}

fn current_utc_timestamp() -> Result<String, StorageError> {
    OffsetDateTime::now_utc()
        .format(UTC_TIMESTAMP_FORMAT)
        .map_err(|_| StorageError::Validation("failed to generate UTC timestamp"))
}
