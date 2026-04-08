use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::format_decimal_amount;
use crate::storage::balances::list_account_balances;
use crate::storage::fx::get_direct_fx_rate;
use crate::storage::records::*;
use crate::storage::{
    AccountSummaryStatus, Amount, AssetId, AssetRecord, Currency, StorageError,
    list_account_positions, list_assets,
};

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

pub async fn get_account_value_summary(
    pool: &SqlitePool,
    account: &crate::storage::AccountRecord,
) -> Result<AccountValueSummaryRecord, StorageError> {
    let assets_by_id = load_assets_by_id(pool).await?;
    get_account_value_summary_with_assets(pool, account, &assets_by_id).await
}

pub(crate) async fn summarize_account_in_currency(
    pool: &SqlitePool,
    account: &crate::storage::AccountRecord,
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

pub(crate) async fn load_assets_by_id(
    pool: &SqlitePool,
) -> Result<BTreeMap<AssetId, AssetRecord>, StorageError> {
    Ok(list_assets(pool)
        .await?
        .into_iter()
        .map(|asset| (asset.id, asset))
        .collect())
}

pub(crate) fn parse_decimal_amount(amount: Decimal) -> Amount {
    Amount::try_from(format_decimal_amount(amount).as_str())
        .expect("formatted total amount should parse")
}

struct AccountTotalSummaryInternal {
    summary_status: AccountSummaryStatus,
    total_amount: Option<Amount>,
}

async fn get_account_value_summary_with_assets(
    pool: &SqlitePool,
    account: &crate::storage::AccountRecord,
    assets_by_id: &BTreeMap<AssetId, AssetRecord>,
) -> Result<AccountValueSummaryRecord, StorageError> {
    summarize_account_in_currency(pool, account, assets_by_id, account.base_currency).await
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
    positions: &[crate::storage::AssetPositionRecord],
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
