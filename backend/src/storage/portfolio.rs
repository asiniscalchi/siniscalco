use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::storage::asset_prices::get_historical_asset_price;
use crate::storage::asset_transactions::list_account_positions_as_of;
use crate::storage::balances::{list_account_balances, list_account_balances_as_of};
use crate::storage::fx::{get_direct_fx_rate, get_fx_rate_or_one, get_historical_fx_rate};
use crate::storage::portfolio_account_summaries::{
    load_assets_by_id, parse_decimal_amount, summarize_account_in_currency,
};
use crate::storage::portfolio_allocation::compute_allocation_totals;
use crate::storage::portfolio_holdings::compute_top_holdings;
use crate::storage::records::*;
use crate::storage::{AccountSummaryStatus, Amount, AssetRecord, Currency, StorageError};

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

    for account in &accounts {
        let balances = list_account_balances(pool, account.id).await?;
        let value_summary =
            summarize_account_in_currency(pool, account, &assets_by_id, display_currency).await?;

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
            name: account.name.clone(),
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
        let rate = get_fx_rate_or_one(pool, cash_record.currency, display_currency).await?;

        let converted = rate.map(|r| parse_decimal_amount(cash_record.amount.as_decimal() * r));
        if let Some(amount) = &converted {
            total_from_currency_breakdown += amount.as_decimal();
        }
        cash_record.converted_amount = converted;
    }

    cash_by_currency.sort_by_key(|balance| balance.currency.as_str());

    let (allocation_totals, allocation_is_partial) =
        compute_allocation_totals(pool, &accounts, &assets_by_id, display_currency).await?;

    let (holdings, holdings_is_partial) = compute_top_holdings(
        pool,
        &accounts,
        &assets_by_id,
        display_currency,
        &cash_by_currency,
    )
    .await?;

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
        allocation_totals,
        allocation_is_partial,
        holdings,
        holdings_is_partial,
    })
}

pub async fn convert_asset_total_value_in_currency(
    pool: &SqlitePool,
    asset: &AssetRecord,
    target_currency: Currency,
) -> Result<Option<Amount>, StorageError> {
    let Some(quantity) = asset.total_quantity else {
        return Ok(None);
    };
    let Some(price) = asset.current_price else {
        return Ok(None);
    };
    let Some(price_currency) = asset.current_price_currency else {
        return Ok(None);
    };

    let total_value = quantity.as_decimal() * price.as_decimal();
    if price_currency == target_currency {
        return Ok(Some(parse_decimal_amount(total_value)));
    }

    let Some(rate) = get_direct_fx_rate(pool, price_currency, target_currency).await? else {
        return Ok(None);
    };

    Ok(Some(parse_decimal_amount(total_value * rate)))
}

pub async fn compute_portfolio_value_at(
    pool: &SqlitePool,
    as_of: &str,
    display_currency: Currency,
) -> Result<Option<Amount>, StorageError> {
    let accounts = crate::storage::accounts::list_accounts(pool).await?;
    let mut total = Decimal::ZERO;

    for account in &accounts {
        let balances = list_account_balances_as_of(pool, account.id, as_of).await?;
        for balance in &balances {
            let Some(rate) =
                get_historical_fx_rate(pool, balance.currency, display_currency, as_of).await?
            else {
                return Ok(None);
            };
            total += balance.amount.as_decimal() * rate;
        }

        let positions = list_account_positions_as_of(pool, account.id, as_of).await?;
        for position in &positions {
            let Some((price, price_currency)) =
                get_historical_asset_price(pool, position.asset_id, as_of).await?
            else {
                return Ok(None);
            };
            let position_value = position.quantity.as_decimal() * price.as_decimal();
            let Some(rate) =
                get_historical_fx_rate(pool, price_currency, display_currency, as_of).await?
            else {
                return Ok(None);
            };
            total += position_value * rate;
        }
    }

    Ok(Some(parse_decimal_amount(total)))
}

// ── Private helpers ───────────────────────────────────────────────────────────
