use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::format_decimal_amount;
use crate::storage::balances::list_account_balances;
use crate::storage::fx::get_direct_fx_rate;
use crate::storage::records::*;
use crate::storage::{
    AccountSummaryStatus, Amount, AssetId, AssetRecord, AssetType, Currency, StorageError,
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

pub async fn get_account_value_summary(
    pool: &SqlitePool,
    account: &crate::storage::AccountRecord,
) -> Result<AccountValueSummaryRecord, StorageError> {
    let assets_by_id = load_assets_by_id(pool).await?;
    get_account_value_summary_with_assets(pool, account, &assets_by_id).await
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

// ── Private helpers ───────────────────────────────────────────────────────────

pub(crate) struct AccountTotalSummaryInternal {
    pub(crate) summary_status: AccountSummaryStatus,
    pub(crate) total_amount: Option<Amount>,
}

async fn get_account_value_summary_with_assets(
    pool: &SqlitePool,
    account: &crate::storage::AccountRecord,
    assets_by_id: &BTreeMap<AssetId, AssetRecord>,
) -> Result<AccountValueSummaryRecord, StorageError> {
    summarize_account_in_currency(pool, account, assets_by_id, account.base_currency).await
}

fn asset_class_label(asset_type: AssetType) -> &'static str {
    match asset_type {
        AssetType::Stock => "Stock",
        AssetType::Etf => "ETF",
        AssetType::Bond => "Bond",
        AssetType::Crypto => "Crypto",
        AssetType::CashEquivalent => "Cash",
        AssetType::Other => "Other",
    }
}

async fn compute_allocation_totals(
    pool: &SqlitePool,
    accounts: &[crate::storage::AccountRecord],
    assets_by_id: &BTreeMap<AssetId, AssetRecord>,
    display_currency: Currency,
) -> Result<(Vec<PortfolioAllocationSliceRecord>, bool), StorageError> {
    let mut class_totals: BTreeMap<&'static str, Decimal> = BTreeMap::new();
    let mut is_partial = false;

    for account in accounts {
        // Cash balances → "Cash" slice
        let balances = list_account_balances(pool, account.id).await?;
        for balance in &balances {
            let rate = if balance.currency == display_currency {
                Some(Decimal::ONE)
            } else {
                get_direct_fx_rate(pool, balance.currency, display_currency).await?
            };
            match rate {
                Some(r) => {
                    *class_totals.entry("Cash").or_insert(Decimal::ZERO) +=
                        balance.amount.as_decimal() * r;
                }
                None => {
                    is_partial = true;
                }
            }
        }

        // Asset positions → grouped by asset type label
        let positions = list_account_positions(pool, account.id).await?;
        for position in &positions {
            let asset = assets_by_id
                .get(&position.asset_id)
                .ok_or(StorageError::Validation(
                    "asset referenced by position was not found",
                ))?;

            let label = asset_class_label(asset.asset_type);

            let Some(price) = asset.current_price else {
                is_partial = true;
                continue;
            };
            let Some(price_currency) = asset.current_price_currency else {
                is_partial = true;
                continue;
            };

            let position_value = position.quantity.as_decimal() * price.as_decimal();

            let rate = if price_currency == display_currency {
                Some(Decimal::ONE)
            } else {
                get_direct_fx_rate(pool, price_currency, display_currency).await?
            };

            match rate {
                Some(r) => {
                    *class_totals.entry(label).or_insert(Decimal::ZERO) += position_value * r;
                }
                None => {
                    is_partial = true;
                }
            }
        }
    }

    let slices = class_totals
        .into_iter()
        .filter(|(_, amount)| *amount > Decimal::ZERO)
        .map(|(label, amount)| PortfolioAllocationSliceRecord {
            label: label.to_string(),
            amount: parse_decimal_amount(amount),
        })
        .collect();

    Ok((slices, is_partial))
}

async fn compute_top_holdings(
    pool: &SqlitePool,
    accounts: &[crate::storage::AccountRecord],
    assets_by_id: &BTreeMap<AssetId, AssetRecord>,
    display_currency: Currency,
    cash_by_currency: &[PortfolioCashByCurrencyRecord],
) -> Result<(Vec<PortfolioHoldingRecord>, bool), StorageError> {
    let mut holdings: Vec<PortfolioHoldingRecord> = Vec::new();
    let mut is_partial = false;
    let mut holding_keys: BTreeSet<String> = BTreeSet::new();
    let mut asset_holding_indexes: BTreeMap<AssetId, usize> = BTreeMap::new();

    // Add cash by currency as holdings
    for cash in cash_by_currency {
        if cash.amount.as_decimal() <= Decimal::ZERO {
            continue;
        }

        let rate = if cash.currency == display_currency {
            Some(Decimal::ONE)
        } else {
            get_direct_fx_rate(pool, cash.currency, display_currency).await?
        };

        let converted_value = match rate {
            Some(r) => cash.amount.as_decimal() * r,
            None => {
                is_partial = true;
                continue;
            }
        };

        holdings.push(PortfolioHoldingRecord {
            asset_id: None,
            symbol: cash.currency.as_str().to_string(),
            name: format!("{} Cash", cash.currency.as_str()),
            value: parse_decimal_amount(converted_value),
        });
        holding_keys.insert(cash.currency.as_str().to_string());
    }

    // Add asset positions as holdings
    for account in accounts {
        let positions = list_account_positions(pool, account.id).await?;
        for position in &positions {
            let asset = assets_by_id
                .get(&position.asset_id)
                .ok_or(StorageError::Validation(
                    "asset referenced by position was not found",
                ))?;

            // Skip if we already have this as cash (duplicate)
            if holding_keys.contains(asset.symbol.to_string().as_str()) {
                continue;
            }

            let Some(price) = asset.current_price else {
                is_partial = true;
                continue;
            };
            let Some(price_currency) = asset.current_price_currency else {
                is_partial = true;
                continue;
            };

            let position_value = position.quantity.as_decimal() * price.as_decimal();

            let rate = if price_currency == display_currency {
                Some(Decimal::ONE)
            } else {
                get_direct_fx_rate(pool, price_currency, display_currency).await?
            };

            let converted_value = match rate {
                Some(r) => position_value * r,
                None => {
                    is_partial = true;
                    continue;
                }
            };

            if let Some(index) = asset_holding_indexes.get(&position.asset_id).copied() {
                let existing_value = holdings[index].value.as_decimal() + converted_value;
                holdings[index].value = parse_decimal_amount(existing_value);
                continue;
            }

            holdings.push(PortfolioHoldingRecord {
                asset_id: Some(position.asset_id),
                symbol: asset.symbol.to_string(),
                name: asset.name.to_string(),
                value: parse_decimal_amount(converted_value),
            });
            asset_holding_indexes.insert(position.asset_id, holdings.len() - 1);
        }
    }

    holdings.sort_by(|a, b| b.value.as_decimal().cmp(&a.value.as_decimal()));

    Ok((holdings, is_partial))
}

async fn summarize_account_in_currency(
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

async fn load_assets_by_id(
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
