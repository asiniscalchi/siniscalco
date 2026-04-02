use async_graphql::{Context, Object};
use sqlx::SqlitePool;

use crate::{
    AccountId, AssetId, PRODUCT_BASE_CURRENCY, SharedFxRefreshStatus, compact_decimal_output,
    convert_asset_total_value_in_currency, get_account, get_account_value_summary, get_asset,
    get_portfolio_summary, get_transaction, list_account_balances, list_account_positions,
    list_account_summaries, list_asset_transactions, list_assets, list_currencies,
    list_fx_rate_summary, list_portfolio_snapshots, list_transactions, list_transfers,
    normalize_amount_output, storage::StorageError,
};

use super::types::*;

pub struct QueryRoot;

pub(crate) fn storage_to_gql(err: StorageError) -> async_graphql::Error {
    match err {
        StorageError::Database(sqlx::Error::RowNotFound) => async_graphql::Error::new("Not found"),
        StorageError::Validation(msg) => async_graphql::Error::new(msg),
        _ => async_graphql::Error::new("Internal server error"),
    }
}

pub(crate) fn not_found_or(
    err: StorageError,
    msg: &'static str,
    fallback: impl Fn(StorageError) -> async_graphql::Error,
) -> async_graphql::Error {
    match err {
        StorageError::Database(sqlx::Error::RowNotFound) => async_graphql::Error::new(msg),
        other => fallback(other),
    }
}

pub(crate) async fn read_fx_refresh_status(
    status: &SharedFxRefreshStatus,
) -> (RefreshAvailability, Option<String>) {
    let status = status.read().await;
    (status.availability.into(), status.last_error.clone())
}

pub(crate) fn to_account_detail(
    account: crate::AccountRecord,
    balances: Vec<crate::AccountBalanceRecord>,
    value_summary: crate::AccountValueSummaryRecord,
) -> AccountDetail {
    AccountDetail {
        id: account.id.as_i64(),
        name: account.name.to_string(),
        account_type: account.account_type.into(),
        base_currency: account.base_currency.as_str().to_string(),
        summary_status: value_summary.summary_status.into(),
        cash_total_amount: value_summary
            .cash_total_amount
            .map(|a| normalize_amount_output(&a.to_string())),
        asset_total_amount: value_summary
            .asset_total_amount
            .map(|a| normalize_amount_output(&a.to_string())),
        total_amount: value_summary
            .total_amount
            .map(|a| normalize_amount_output(&a.to_string())),
        total_currency: value_summary.total_currency.map(|c| c.as_str().to_string()),
        created_at: account.created_at,
        balances: balances.into_iter().map(to_balance).collect(),
    }
}

pub(crate) fn to_balance(balance: crate::AccountBalanceRecord) -> Balance {
    Balance {
        currency: balance.currency.as_str().to_string(),
        amount: normalize_amount_output(&balance.amount.to_string()),
        updated_at: balance.updated_at,
    }
}

pub(crate) fn to_asset(
    asset: crate::AssetRecord,
    converted_total_value: Option<crate::Amount>,
    converted_total_value_currency: Option<crate::Currency>,
) -> Asset {
    Asset {
        id: asset.id.as_i64(),
        symbol: asset.symbol.to_string(),
        name: asset.name.to_string(),
        asset_type: asset.asset_type.into(),
        quote_symbol: asset.quote_symbol,
        isin: asset.isin,
        current_price: asset
            .current_price
            .map(|p| normalize_amount_output(&p.to_string())),
        current_price_currency: asset.current_price_currency.map(|c| c.as_str().to_string()),
        current_price_as_of: asset.current_price_as_of,
        total_quantity: asset
            .total_quantity
            .map(|q| normalize_amount_output(&q.to_string())),
        avg_cost_basis: asset
            .avg_cost_basis
            .map(|p| normalize_amount_output(&p.to_string())),
        avg_cost_basis_currency: asset
            .avg_cost_basis_currency
            .map(|c| c.as_str().to_string()),
        previous_close: asset
            .previous_close
            .map(|p| normalize_amount_output(&p.to_string())),
        previous_close_currency: asset
            .previous_close_currency
            .map(|c| c.as_str().to_string()),
        converted_total_value: converted_total_value
            .map(|amount| normalize_amount_output(&amount.to_string())),
        converted_total_value_currency: converted_total_value_currency
            .map(|currency| currency.as_str().to_string()),
        created_at: asset.created_at,
        updated_at: asset.updated_at,
    }
}

async fn to_asset_with_display_total(
    pool: &SqlitePool,
    asset: crate::AssetRecord,
) -> Result<Asset, StorageError> {
    let converted_total_value =
        convert_asset_total_value_in_currency(pool, &asset, PRODUCT_BASE_CURRENCY).await?;

    Ok(to_asset(
        asset,
        converted_total_value,
        converted_total_value.map(|_| PRODUCT_BASE_CURRENCY),
    ))
}

pub(crate) fn to_transaction(tx: crate::AssetTransactionRecord) -> Transaction {
    Transaction {
        id: tx.id,
        account_id: tx.account_id.as_i64(),
        asset_id: tx.asset_id.as_i64(),
        transaction_type: tx.transaction_type.into(),
        trade_date: tx.trade_date.to_string(),
        quantity: normalize_amount_output(&tx.quantity.to_string()),
        unit_price: normalize_amount_output(&tx.unit_price.to_string()),
        currency_code: tx.currency_code.as_str().to_string(),
        notes: tx.notes,
        created_at: tx.created_at,
        updated_at: tx.updated_at,
    }
}

#[Object]
impl QueryRoot {
    async fn portfolio_history(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<Vec<PortfolioSnapshot>> {
        let pool = ctx.data::<SqlitePool>()?;
        let snapshots = list_portfolio_snapshots(pool, PRODUCT_BASE_CURRENCY)
            .await
            .map_err(storage_to_gql)?;
        Ok(snapshots
            .into_iter()
            .map(|s| PortfolioSnapshot {
                total_value: normalize_amount_output(&s.total_value.to_string()),
                currency: s.currency.as_str().to_string(),
                recorded_at: s.recorded_at,
            })
            .collect())
    }

    async fn portfolio(&self, ctx: &Context<'_>) -> async_graphql::Result<PortfolioSummary> {
        let pool = ctx.data::<SqlitePool>()?;
        let fx_status = ctx.data::<SharedFxRefreshStatus>()?;
        let summary = get_portfolio_summary(pool, PRODUCT_BASE_CURRENCY)
            .await
            .map_err(storage_to_gql)?;
        let (refresh_status, refresh_error) = read_fx_refresh_status(fx_status).await;
        Ok(to_portfolio_summary(summary, refresh_status, refresh_error))
    }

    async fn accounts(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<AccountSummary>> {
        let pool = ctx.data::<SqlitePool>()?;
        let accounts = list_account_summaries(pool).await.map_err(storage_to_gql)?;
        Ok(accounts.into_iter().map(to_account_summary).collect())
    }

    async fn account(&self, ctx: &Context<'_>, id: i64) -> async_graphql::Result<AccountDetail> {
        let pool = ctx.data::<SqlitePool>()?;
        let account_id = AccountId::try_from(id).map_err(storage_to_gql)?;
        let account = get_account(pool, account_id)
            .await
            .map_err(|e| not_found_or(e, "Account not found", storage_to_gql))?;
        let balances = list_account_balances(pool, account_id)
            .await
            .map_err(storage_to_gql)?;
        let value_summary = get_account_value_summary(pool, &account)
            .await
            .map_err(storage_to_gql)?;
        Ok(to_account_detail(account, balances, value_summary))
    }

    async fn account_positions(
        &self,
        ctx: &Context<'_>,
        account_id: i64,
    ) -> async_graphql::Result<Vec<AssetPosition>> {
        let pool = ctx.data::<SqlitePool>()?;
        let account_id = AccountId::try_from(account_id).map_err(storage_to_gql)?;
        let positions = list_account_positions(pool, account_id)
            .await
            .map_err(storage_to_gql)?;
        Ok(positions
            .into_iter()
            .map(|p| AssetPosition {
                account_id: p.account_id.as_i64(),
                asset_id: p.asset_id.as_i64(),
                quantity: normalize_amount_output(&p.quantity.to_string()),
            })
            .collect())
    }

    async fn assets(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Asset>> {
        let pool = ctx.data::<SqlitePool>()?;
        let assets = list_assets(pool).await.map_err(storage_to_gql)?;
        let mut gql_assets = Vec::with_capacity(assets.len());
        for asset in assets {
            gql_assets.push(
                to_asset_with_display_total(pool, asset)
                    .await
                    .map_err(storage_to_gql)?,
            );
        }
        Ok(gql_assets)
    }

    async fn asset(&self, ctx: &Context<'_>, id: i64) -> async_graphql::Result<Asset> {
        let pool = ctx.data::<SqlitePool>()?;
        let asset_id = AssetId::try_from(id).map_err(storage_to_gql)?;
        let asset = get_asset(pool, asset_id)
            .await
            .map_err(|e| not_found_or(e, "Asset not found", storage_to_gql))?;
        to_asset_with_display_total(pool, asset)
            .await
            .map_err(storage_to_gql)
    }

    async fn transactions(
        &self,
        ctx: &Context<'_>,
        account_id: Option<i64>,
    ) -> async_graphql::Result<Vec<Transaction>> {
        let pool = ctx.data::<SqlitePool>()?;
        let transactions = if let Some(id) = account_id {
            let account_id = AccountId::try_from(id).map_err(storage_to_gql)?;
            list_asset_transactions(pool, account_id)
                .await
                .map_err(storage_to_gql)?
        } else {
            list_transactions(pool).await.map_err(storage_to_gql)?
        };
        Ok(transactions.into_iter().map(to_transaction).collect())
    }

    async fn transaction(&self, ctx: &Context<'_>, id: i64) -> async_graphql::Result<Transaction> {
        let pool = ctx.data::<SqlitePool>()?;
        let transaction = get_transaction(pool, id)
            .await
            .map_err(|e| not_found_or(e, "Transaction not found", storage_to_gql))?;
        Ok(to_transaction(transaction))
    }

    async fn transfers(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Transfer>> {
        let pool = ctx.data::<SqlitePool>()?;
        let transfers = list_transfers(pool).await.map_err(storage_to_gql)?;
        Ok(transfers
            .into_iter()
            .map(super::mutation::to_transfer)
            .collect())
    }

    async fn currencies(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<String>> {
        let pool = ctx.data::<SqlitePool>()?;
        let currencies = list_currencies(pool).await.map_err(storage_to_gql)?;
        Ok(currencies
            .into_iter()
            .map(|c| c.code.as_str().to_string())
            .collect())
    }

    async fn fx_rates(&self, ctx: &Context<'_>) -> async_graphql::Result<FxRateSummary> {
        let pool = ctx.data::<SqlitePool>()?;
        let fx_status = ctx.data::<SharedFxRefreshStatus>()?;
        let summary = list_fx_rate_summary(pool, PRODUCT_BASE_CURRENCY)
            .await
            .map_err(storage_to_gql)?;
        let (refresh_status, refresh_error) = read_fx_refresh_status(fx_status).await;
        Ok(to_fx_rate_summary(summary, refresh_status, refresh_error))
    }
}

fn to_account_summary(account: crate::AccountSummaryRecord) -> AccountSummary {
    AccountSummary {
        id: account.id.as_i64(),
        name: account.name.to_string(),
        account_type: account.account_type.into(),
        base_currency: account.base_currency.as_str().to_string(),
        summary_status: account.summary_status.into(),
        cash_total_amount: account
            .cash_total_amount
            .map(|a| normalize_amount_output(&a.to_string())),
        asset_total_amount: account
            .asset_total_amount
            .map(|a| normalize_amount_output(&a.to_string())),
        total_amount: account
            .total_amount
            .map(|a| normalize_amount_output(&a.to_string())),
        total_currency: account.total_currency.map(|c| c.as_str().to_string()),
    }
}

fn to_fx_rate_summary(
    summary: crate::FxRateSummaryRecord,
    refresh_status: RefreshAvailability,
    refresh_error: Option<String>,
) -> FxRateSummary {
    FxRateSummary {
        target_currency: summary.target_currency.as_str().to_string(),
        rates: summary
            .rates
            .into_iter()
            .map(|r| FxRateSummaryItem {
                currency: r.from_currency.as_str().to_string(),
                rate: compact_decimal_output(&r.rate.to_string()),
            })
            .collect(),
        last_updated: summary.last_updated,
        refresh_status,
        refresh_error,
    }
}

fn to_portfolio_summary(
    summary: crate::PortfolioSummaryRecord,
    refresh_status: RefreshAvailability,
    refresh_error: Option<String>,
) -> PortfolioSummary {
    PortfolioSummary {
        display_currency: summary.display_currency.as_str().to_string(),
        total_value_status: summary.total_value_status.into(),
        total_value_amount: summary
            .total_value_amount
            .map(|a| normalize_amount_output(&a.to_string())),
        account_totals: summary
            .account_totals
            .into_iter()
            .map(|a| PortfolioAccountTotal {
                id: a.id.as_i64(),
                name: a.name.to_string(),
                account_type: a.account_type.into(),
                summary_status: a.summary_status.into(),
                cash_total_amount: a
                    .cash_total_amount
                    .map(|x| normalize_amount_output(&x.to_string())),
                asset_total_amount: a
                    .asset_total_amount
                    .map(|x| normalize_amount_output(&x.to_string())),
                total_amount: a
                    .total_amount
                    .map(|x| normalize_amount_output(&x.to_string())),
                total_currency: a.total_currency.as_str().to_string(),
            })
            .collect(),
        cash_by_currency: summary
            .cash_by_currency
            .into_iter()
            .map(|c| PortfolioCashByCurrency {
                currency: c.currency.as_str().to_string(),
                amount: normalize_amount_output(&c.amount.to_string()),
                converted_amount: c
                    .converted_amount
                    .map(|a| normalize_amount_output(&a.to_string())),
            })
            .collect(),
        fx_last_updated: summary.fx_last_updated,
        fx_refresh_status: refresh_status,
        fx_refresh_error: refresh_error,
        allocation_totals: summary
            .allocation_totals
            .into_iter()
            .map(|s| PortfolioAllocationSlice {
                label: s.label,
                amount: normalize_amount_output(&s.amount.to_string()),
            })
            .collect(),
        allocation_is_partial: summary.allocation_is_partial,
        holdings: summary
            .holdings
            .into_iter()
            .map(|h| PortfolioHolding {
                asset_id: h.asset_id.map(|id| id.as_i64()),
                symbol: h.symbol,
                name: h.name,
                value: normalize_amount_output(&h.value.to_string()),
            })
            .collect(),
        holdings_is_partial: summary.holdings_is_partial,
    }
}
