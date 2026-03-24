use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

use crate::format_decimal_amount;
use crate::storage::fx::get_direct_fx_rate;
use crate::storage::records::*;
use crate::storage::{
    AccountId, AccountSummaryStatus, Amount, AssetId, AssetRecord, Currency, StorageError,
    list_account_positions, list_assets,
};

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
    .bind(input.amount.as_scaled_i64())
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
            amount,
            updated_at
        FROM account_balances
        WHERE account_id = ?
        ORDER BY currency
        "#,
    )
    .bind(account_id.as_i64())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AccountBalanceRecord {
                account_id: AccountId::try_from(row.get::<i64, _>("account_id"))?,
                currency: Currency::try_from(row.get::<&str, _>("currency"))?,
                amount: Amount::from_scaled_i64(row.get::<i64, _>("amount")),
                updated_at: row.get("updated_at"),
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()
}

pub async fn list_account_summaries(
    pool: &SqlitePool,
) -> Result<Vec<AccountSummaryRecord>, StorageError> {
    let accounts = crate::storage::accounts::list_accounts(pool).await?;
    let assets_by_id = load_assets_by_id(pool).await?;
    let mut summaries = Vec::with_capacity(accounts.len());

    for account in accounts {
        let summary = get_account_value_summary_with_assets(pool, &account, &assets_by_id).await?;

        summaries.push(AccountSummaryRecord {
            id: account.id,
            name: account.name,
            account_type: account.account_type,
            base_currency: account.base_currency,
            summary_status: summary.summary_status,
            cash_total_amount: summary.cash_total_amount,
            asset_total_amount: summary.asset_total_amount,
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
    let assets_by_id = load_assets_by_id(pool).await?;
    let fx_summary = crate::storage::fx::list_fx_rate_summary(pool, display_currency).await?;
    let mut account_totals = Vec::with_capacity(accounts.len());
    let mut cash_by_currency = Vec::<PortfolioCashByCurrencyRecord>::new();
    let mut portfolio_asset_total_decimal = Decimal::ZERO;
    let mut portfolio_status = AccountSummaryStatus::Ok;

    for account in accounts {
        let balances = list_account_balances(pool, account.id).await?;
        let value_summary =
            summarize_account_in_currency(pool, &account, &assets_by_id, display_currency).await?;

        for balance in balances {
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
            summary_status: value_summary.summary_status,
            cash_total_amount: value_summary.cash_total_amount,
            asset_total_amount: value_summary.asset_total_amount,
            total_amount: value_summary.total_amount,
            total_currency: display_currency,
        });

        if value_summary.summary_status != AccountSummaryStatus::Ok {
            portfolio_status = AccountSummaryStatus::ConversionUnavailable;
        } else if let Some(asset_total_amount) = value_summary.asset_total_amount {
            portfolio_asset_total_decimal += asset_total_amount.as_decimal();
        }
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
            Some(parse_decimal_amount(
                total_from_currency_breakdown + portfolio_asset_total_decimal,
            ))
        } else {
            None
        },
        account_totals,
        cash_by_currency,
        fx_last_updated: fx_summary.last_updated,
    })
}

pub(crate) struct AccountTotalSummaryInternal {
    pub(crate) summary_status: AccountSummaryStatus,
    pub(crate) total_amount: Option<Amount>,
}

pub async fn get_account_value_summary(
    pool: &SqlitePool,
    account: &AccountRecord,
) -> Result<AccountValueSummaryRecord, StorageError> {
    let assets_by_id = load_assets_by_id(pool).await?;
    get_account_value_summary_with_assets(pool, account, &assets_by_id).await
}

async fn get_account_value_summary_with_assets(
    pool: &SqlitePool,
    account: &AccountRecord,
    assets_by_id: &BTreeMap<AssetId, AssetRecord>,
) -> Result<AccountValueSummaryRecord, StorageError> {
    summarize_account_in_currency(pool, account, assets_by_id, account.base_currency).await
}

async fn summarize_account_in_currency(
    pool: &SqlitePool,
    account: &AccountRecord,
    assets_by_id: &BTreeMap<AssetId, AssetRecord>,
    target_currency: Currency,
) -> Result<AccountValueSummaryRecord, StorageError> {
    let balances = list_account_balances(pool, account.id).await?;
    let positions = list_account_positions(pool, account.id).await?;
    let cash_summary = summarize_balances_in_currency(pool, &balances, target_currency).await?;
    let asset_summary =
        summarize_positions_in_currency(pool, &positions, assets_by_id, target_currency).await?;

    let summary_status = if cash_summary.summary_status == AccountSummaryStatus::Ok
        && asset_summary.summary_status == AccountSummaryStatus::Ok
    {
        AccountSummaryStatus::Ok
    } else {
        AccountSummaryStatus::ConversionUnavailable
    };

    let total_amount = match (cash_summary.total_amount, asset_summary.total_amount) {
        (Some(cash_total), Some(asset_total)) => Some(parse_decimal_amount(
            cash_total.as_decimal() + asset_total.as_decimal(),
        )),
        _ => None,
    };

    Ok(AccountValueSummaryRecord {
        summary_status,
        cash_total_amount: cash_summary.total_amount,
        asset_total_amount: asset_summary.total_amount,
        total_amount,
        total_currency: if summary_status == AccountSummaryStatus::Ok {
            Some(target_currency)
        } else {
            None
        },
    })
}

async fn summarize_balances_in_currency(
    pool: &SqlitePool,
    balances: &[AccountBalanceRecord],
    target_currency: Currency,
) -> Result<AccountTotalSummaryInternal, StorageError> {
    if balances.is_empty() {
        return Ok(AccountTotalSummaryInternal {
            summary_status: AccountSummaryStatus::Ok,
            total_amount: Some(parse_decimal_amount(Decimal::ZERO)),
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
                summary_status: AccountSummaryStatus::ConversionUnavailable,
                total_amount: None,
            });
        };

        total += balance.amount.as_decimal() * rate;
    }

    Ok(AccountTotalSummaryInternal {
        summary_status: AccountSummaryStatus::Ok,
        total_amount: Some(parse_decimal_amount(total)),
    })
}

async fn summarize_positions_in_currency(
    pool: &SqlitePool,
    positions: &[AssetPositionRecord],
    assets_by_id: &BTreeMap<AssetId, AssetRecord>,
    target_currency: Currency,
) -> Result<AccountTotalSummaryInternal, StorageError> {
    if positions.is_empty() {
        return Ok(AccountTotalSummaryInternal {
            summary_status: AccountSummaryStatus::Ok,
            total_amount: Some(parse_decimal_amount(Decimal::ZERO)),
        });
    }

    let mut total = Decimal::ZERO;

    for position in positions {
        let asset = assets_by_id
            .get(&position.asset_id)
            .ok_or(StorageError::Validation(
                "asset referenced by position was not found",
            ))?;
        let Some(price) = asset.current_price else {
            return Ok(AccountTotalSummaryInternal {
                summary_status: AccountSummaryStatus::ConversionUnavailable,
                total_amount: None,
            });
        };
        let Some(price_currency) = asset.current_price_currency else {
            return Ok(AccountTotalSummaryInternal {
                summary_status: AccountSummaryStatus::ConversionUnavailable,
                total_amount: None,
            });
        };

        let position_value = position.quantity.as_decimal() * price.as_decimal();

        if price_currency == target_currency {
            total += position_value;
            continue;
        }

        let Some(rate) = get_direct_fx_rate(pool, price_currency, target_currency).await? else {
            return Ok(AccountTotalSummaryInternal {
                summary_status: AccountSummaryStatus::ConversionUnavailable,
                total_amount: None,
            });
        };

        total += position_value * rate;
    }

    Ok(AccountTotalSummaryInternal {
        summary_status: AccountSummaryStatus::Ok,
        total_amount: Some(parse_decimal_amount(total)),
    })
}

async fn load_assets_by_id(
    pool: &SqlitePool,
) -> Result<BTreeMap<AssetId, AssetRecord>, StorageError> {
    Ok(list_assets(pool)
        .await?
        .into_iter()
        .map(|asset| (asset.id, asset))
        .collect())
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
