use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

use crate::format_decimal_amount;
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
        .map(|row| {
            Ok(AccountBalanceRecord {
                account_id: AccountId::try_from(row.get::<i64, _>("account_id"))?,
                currency: Currency::try_from(row.get::<&str, _>("currency"))?,
                amount: Amount::try_from(row.get::<&str, _>("amount"))?,
                updated_at: row.get("updated_at"),
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()?)
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

pub async fn get_portfolio_summary(
    pool: &SqlitePool,
    display_currency: Currency,
) -> Result<PortfolioSummaryRecord, StorageError> {
    let accounts = crate::storage::accounts::list_accounts(pool).await?;
    let fx_summary = crate::storage::fx::list_fx_rate_summary(pool, display_currency).await?;
    let mut account_totals = Vec::with_capacity(accounts.len());
    let mut cash_by_currency = Vec::<PortfolioCashByCurrencyRecord>::new();
    let mut portfolio_total_decimal = Decimal::ZERO;
    let mut portfolio_status = AccountSummaryStatus::Ok;

    for account in accounts {
        let balances = list_account_balances(pool, account.id).await?;
        let mut account_total_decimal = Decimal::ZERO;
        let mut account_status = AccountSummaryStatus::Ok;

        for balance in balances {
            let rate = if balance.currency == display_currency {
                Some(Decimal::ONE)
            } else {
                get_direct_fx_rate(pool, balance.currency, display_currency).await?
            };

            let converted_balance_decimal = rate.map(|r| balance.amount.as_decimal() * r);

            if let Some(converted_decimal) = converted_balance_decimal {
                account_total_decimal += converted_decimal;
                portfolio_total_decimal += converted_decimal;
            } else {
                account_status = AccountSummaryStatus::ConversionUnavailable;
                portfolio_status = AccountSummaryStatus::ConversionUnavailable;
            }

            if let Some(existing_balance) = cash_by_currency
                .iter_mut()
                .find(|existing_balance| existing_balance.currency == balance.currency)
            {
                existing_balance.amount = parse_decimal_amount(
                    existing_balance.amount.as_decimal() + balance.amount.as_decimal(),
                );
                // We'll fix converted_amount later to avoid intermediate rounding
            } else {
                cash_by_currency.push(PortfolioCashByCurrencyRecord {
                    currency: balance.currency,
                    amount: balance.amount,
                    converted_amount: None, // Will be calculated after the loop
                });
            }
        }

        account_totals.push(PortfolioAccountTotalRecord {
            id: account.id,
            name: account.name,
            account_type: account.account_type,
            summary_status: account_status,
            total_amount: if account_status == AccountSummaryStatus::Ok {
                Some(parse_decimal_amount(account_total_decimal))
            } else {
                None
            },
            total_currency: display_currency,
        });
    }

    // Now calculate converted amounts for each currency in the summary
    let mut total_from_currency_breakdown = Decimal::ZERO;
    for cash_record in &mut cash_by_currency {
        let rate = if cash_record.currency == display_currency {
            Some(Decimal::ONE)
        } else {
            get_direct_fx_rate(pool, cash_record.currency, display_currency).await?
        };

        let converted = rate.map(|r| parse_decimal_amount(cash_record.amount.as_decimal() * r));
        if let Some(amount) = &converted {
            total_from_currency_breakdown += amount.as_decimal();
        }
        cash_record.converted_amount = converted;
    }

    cash_by_currency.sort_by_key(|balance| balance.currency.as_str());

    Ok(PortfolioSummaryRecord {
        display_currency,
        total_value_status: portfolio_status,
        total_value_amount: if portfolio_status == AccountSummaryStatus::Ok {
            Some(parse_decimal_amount(total_from_currency_breakdown))
        } else {
            None
        },
        account_totals,
        cash_by_currency,
        fx_last_updated: fx_summary.last_updated,
    })
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
    let summary = summarize_balances_in_currency(pool, balances, account.base_currency).await?;

    Ok(AccountTotalSummaryInternal {
        status: summary.status,
        total_amount: summary.total_amount,
        total_currency: summary.total_currency,
    })
}

async fn summarize_balances_in_currency(
    pool: &SqlitePool,
    balances: &[AccountBalanceRecord],
    target_currency: Currency,
) -> Result<AccountTotalSummaryInternal, StorageError> {
    if balances.is_empty() {
        return Ok(AccountTotalSummaryInternal {
            status: AccountSummaryStatus::Ok,
            total_amount: Some(parse_decimal_amount(Decimal::ZERO)),
            total_currency: Some(target_currency),
        });
    }

    let mut total = Decimal::ZERO;

    for balance in balances {
        if balance.currency == target_currency {
            total += balance.amount.as_decimal();
            continue;
        }

        let Some(rate) = get_direct_fx_rate(pool, balance.currency, target_currency).await? else {
            return Ok(AccountTotalSummaryInternal {
                status: AccountSummaryStatus::ConversionUnavailable,
                total_amount: None,
                total_currency: None,
            });
        };

        total += balance.amount.as_decimal() * rate;
    }

    Ok(AccountTotalSummaryInternal {
        status: AccountSummaryStatus::Ok,
        total_amount: Some(parse_decimal_amount(total)),
        total_currency: Some(target_currency),
    })
}

fn parse_decimal_amount(amount: Decimal) -> Amount {
    Amount::try_from(format_decimal_amount(amount).as_str())
        .expect("formatted total amount should parse")
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
