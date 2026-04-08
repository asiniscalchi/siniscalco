use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::storage::balances::list_account_balances;
use crate::storage::fx::get_fx_rate_or_one;
use crate::storage::portfolio_account_summaries::parse_decimal_amount;
use crate::storage::records::PortfolioAllocationSliceRecord;
use crate::storage::{
    AssetId, AssetRecord, AssetType, Currency, StorageError, list_account_positions,
};

pub(crate) async fn compute_allocation_totals(
    pool: &SqlitePool,
    accounts: &[crate::storage::AccountRecord],
    assets_by_id: &BTreeMap<AssetId, AssetRecord>,
    display_currency: Currency,
) -> Result<(Vec<PortfolioAllocationSliceRecord>, bool), StorageError> {
    let mut class_totals: BTreeMap<&'static str, Decimal> = BTreeMap::new();
    let mut is_partial = false;

    for account in accounts {
        let balances = list_account_balances(pool, account.id).await?;
        for balance in &balances {
            let rate = get_fx_rate_or_one(pool, balance.currency, display_currency).await?;
            match rate {
                Some(rate) => {
                    *class_totals.entry("Cash").or_insert(Decimal::ZERO) +=
                        balance.amount.as_decimal() * rate;
                }
                None => is_partial = true,
            }
        }

        let positions = list_account_positions(pool, account.id).await?;
        for position in &positions {
            let asset = assets_by_id
                .get(&position.asset_id)
                .ok_or(StorageError::Validation(
                    "asset referenced by position was not found",
                ))?;

            let Some(price) = asset.current_price else {
                is_partial = true;
                continue;
            };
            let Some(price_currency) = asset.current_price_currency else {
                is_partial = true;
                continue;
            };

            let position_value = position.quantity.as_decimal() * price.as_decimal();
            let rate = get_fx_rate_or_one(pool, price_currency, display_currency).await?;

            match rate {
                Some(rate) => {
                    *class_totals
                        .entry(asset_class_label(asset.asset_type))
                        .or_insert(Decimal::ZERO) += position_value * rate;
                }
                None => is_partial = true,
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
