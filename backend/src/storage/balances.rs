use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

use crate::storage::fx::get_direct_fx_rate;
use crate::storage::records::*;
use crate::storage::{AccountId, AccountSummaryStatus, Amount, Currency, StorageError};

pub async fn upsert_account_balance(
    pool: &SqlitePool,
    input: UpsertAccountBalanceInput,
) -> Result<UpsertOutcome, StorageError> {
    let updated_at = current_utc_timestamp()?;
    let mut transaction = pool.begin().await?;

    let existed = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM account_balances WHERE account_id = ? AND currency = ?)",
    )
    .bind(input.account_id.as_i64())
    .bind(input.currency.as_str())
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
    .bind(input.account_id.as_i64())
    .bind(input.currency.as_str())
    .bind(input.amount.to_string())
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
    account_id: AccountId,
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
    .bind(account_id.as_i64())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AccountBalanceRecord {
            account_id: AccountId::try_from(row.get::<i64, _>("account_id"))
                .expect("stored account id should be valid"),
            currency: Currency::try_from(row.get::<&str, _>("currency"))
                .expect("stored currency should be valid"),
            amount: Amount::try_from(row.get::<&str, _>("amount"))
                .expect("stored amount should be valid"),
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
    pub(crate) total_amount: Option<Amount>,
    pub(crate) total_currency: Option<Currency>,
}

async fn summarize_account(
    pool: &SqlitePool,
    account: &AccountRecord,
    balances: &[AccountBalanceRecord],
) -> Result<AccountTotalSummaryInternal, StorageError> {
    if balances.is_empty() {
        return Ok(AccountTotalSummaryInternal {
            status: AccountSummaryStatus::Ok,
            total_amount: Some(Amount::try_from("0.00000000").expect("zero amount should parse")),
            total_currency: Some(account.base_currency),
        });
    }

    let mut total = Decimal::ZERO;

    for balance in balances {
        let amount = balance.amount.as_decimal();

        if balance.currency == account.base_currency {
            total += amount;
            continue;
        }

        let Some(rate) = get_direct_fx_rate(pool, balance.currency, account.base_currency).await?
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
        total_amount: Some(
            Amount::try_from(crate::format_decimal_amount(total).as_str())
                .expect("formatted total amount should parse"),
        ),
        total_currency: Some(account.base_currency),
    })
}

pub async fn delete_account_balance(
    pool: &SqlitePool,
    account_id: AccountId,
    currency: Currency,
) -> Result<(), StorageError> {
    let result = sqlx::query("DELETE FROM account_balances WHERE account_id = ? AND currency = ?")
        .bind(account_id.as_i64())
        .bind(currency.as_str())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::Database(sqlx::Error::RowNotFound));
    }

    Ok(())
}
