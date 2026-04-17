use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use sqlx::SqlitePool;

use crate::storage::fx::get_fx_rate_or_one;
use crate::storage::portfolio_account_summaries::parse_decimal_amount;
use crate::storage::records::{PortfolioCashByCurrencyRecord, PortfolioHoldingRecord};
use crate::storage::{AssetId, AssetRecord, Currency, StorageError, list_account_positions};

pub(crate) async fn compute_top_holdings(
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

    for cash in cash_by_currency {
        if cash.amount.as_decimal() <= Decimal::ZERO {
            continue;
        }

        let rate = get_fx_rate_or_one(pool, cash.currency, display_currency).await?;
        let converted_value = match rate {
            Some(rate) => cash.amount.as_decimal() * rate,
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

    for account in accounts {
        let positions = list_account_positions(pool, account.id).await?;
        for position in &positions {
            let asset = assets_by_id
                .get(&position.asset_id)
                .ok_or(StorageError::Validation(
                    "asset referenced by position was not found",
                ))?;

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
            let rate = get_fx_rate_or_one(pool, price_currency, display_currency).await?;
            let converted_value = match rate {
                Some(rate) => position_value * rate,
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

    holdings.sort_by_key(|h| std::cmp::Reverse(h.value.as_decimal()));

    Ok((holdings, is_partial))
}
