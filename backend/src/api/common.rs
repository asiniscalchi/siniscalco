use axum::extract::rejection::{JsonRejection, QueryRejection};

use super::{
    AccountDetailResponse, AccountSummaryResponse, ApiError, AssetPositionResponse, AssetResponse,
    AssetTransactionResponse, BalanceResponse, CreateAssetApiError, CreatedAssetResponse,
    CurrencyResponse, FxRateDetailResponse, FxRateSummaryItemResponse, FxRateSummaryResponse,
    PortfolioAccountTotalResponse, PortfolioAllocationSliceResponse, PortfolioCashByCurrencyResponse,
    PortfolioSummaryResponse,
};
use crate::{
    AccountBalanceRecord, AccountSummaryRecord, AccountValueSummaryRecord, AssetPositionRecord,
    AssetRecord, AssetTransactionRecord, CurrencyRecord, FxRateDetailRecord,
    FxRateSummaryItemRecord, FxRateSummaryRecord, PortfolioAccountTotalRecord,
    PortfolioAllocationSliceRecord, PortfolioCashByCurrencyRecord, PortfolioSummaryRecord,
    compact_decimal_output, normalize_amount_output,
};

pub(super) fn map_json_rejection(rejection: JsonRejection) -> ApiError {
    match rejection {
        JsonRejection::JsonSyntaxError(_) | JsonRejection::JsonDataError(_) => {
            ApiError::validation("Malformed JSON body")
        }
        JsonRejection::MissingJsonContentType(_) => ApiError::validation("Expected JSON body"),
        _ => ApiError::validation("Invalid JSON body"),
    }
}

pub(super) fn map_create_asset_json_rejection(rejection: JsonRejection) -> CreateAssetApiError {
    match rejection {
        JsonRejection::JsonSyntaxError(_) | JsonRejection::JsonDataError(_) => {
            CreateAssetApiError::bad_request("Malformed JSON body")
        }
        JsonRejection::MissingJsonContentType(_) => {
            CreateAssetApiError::bad_request("Expected JSON body")
        }
        _ => CreateAssetApiError::bad_request("Invalid JSON body"),
    }
}

pub(super) fn map_query_rejection(_: QueryRejection) -> ApiError {
    ApiError::validation("Invalid query parameters")
}

pub(super) fn to_account_summary_response(account: AccountSummaryRecord) -> AccountSummaryResponse {
    AccountSummaryResponse {
        id: account.id.as_i64(),
        name: account.name.to_string(),
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        summary_status: account.summary_status.as_str().to_string(),
        cash_total_amount: account
            .cash_total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        asset_total_amount: account
            .asset_total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        total_amount: account
            .total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        total_currency: account.total_currency,
    }
}

pub(super) fn to_account_detail_response(
    account: crate::AccountRecord,
    balances: Vec<AccountBalanceRecord>,
    value_summary: AccountValueSummaryRecord,
) -> AccountDetailResponse {
    AccountDetailResponse {
        id: account.id.as_i64(),
        name: account.name.to_string(),
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        summary_status: value_summary.summary_status.as_str().to_string(),
        cash_total_amount: value_summary
            .cash_total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        asset_total_amount: value_summary
            .asset_total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        total_amount: value_summary
            .total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        total_currency: value_summary.total_currency,
        created_at: account.created_at,
        balances: balances.into_iter().map(to_balance_response).collect(),
    }
}

pub(super) fn to_balance_response(balance: AccountBalanceRecord) -> BalanceResponse {
    BalanceResponse {
        currency: balance.currency,
        amount: normalize_amount_output(&balance.amount.to_string()),
        updated_at: balance.updated_at,
    }
}

pub(super) fn to_currency_response(currency: CurrencyRecord) -> CurrencyResponse {
    CurrencyResponse {
        code: currency.code,
    }
}

pub(super) fn to_asset_response(asset: AssetRecord) -> AssetResponse {
    AssetResponse {
        id: asset.id.as_i64(),
        symbol: asset.symbol.to_string(),
        name: asset.name.to_string(),
        asset_type: asset.asset_type,
        quote_symbol: asset.quote_symbol,
        isin: asset.isin,
        current_price: asset
            .current_price
            .map(|price| normalize_amount_output(&price.to_string())),
        current_price_currency: asset.current_price_currency,
        current_price_as_of: asset.current_price_as_of,
    }
}

pub(super) fn to_created_asset_response(asset: AssetRecord) -> CreatedAssetResponse {
    CreatedAssetResponse {
        id: asset.id.as_i64(),
        symbol: asset.symbol.to_string(),
        name: asset.name.to_string(),
        asset_type: asset.asset_type,
        quote_symbol: asset.quote_symbol,
        isin: asset.isin,
        current_price: asset
            .current_price
            .map(|price| normalize_amount_output(&price.to_string())),
        current_price_currency: asset.current_price_currency,
        current_price_as_of: asset.current_price_as_of,
        created_at: asset.created_at,
        updated_at: asset.updated_at,
    }
}

pub(super) fn to_asset_transaction_response(
    transaction: AssetTransactionRecord,
) -> AssetTransactionResponse {
    AssetTransactionResponse {
        id: transaction.id,
        account_id: transaction.account_id.as_i64(),
        asset_id: transaction.asset_id.as_i64(),
        transaction_type: transaction.transaction_type.as_str().to_string(),
        trade_date: transaction.trade_date.to_string(),
        quantity: normalize_amount_output(&transaction.quantity.to_string()),
        unit_price: normalize_amount_output(&transaction.unit_price.to_string()),
        currency_code: transaction.currency_code,
        notes: transaction.notes,
        created_at: transaction.created_at,
        updated_at: transaction.updated_at,
    }
}

pub(super) fn to_asset_position_response(position: AssetPositionRecord) -> AssetPositionResponse {
    AssetPositionResponse {
        account_id: position.account_id.as_i64(),
        asset_id: position.asset_id.as_i64(),
        quantity: normalize_amount_output(&position.quantity.to_string()),
    }
}

pub(super) fn to_fx_rate_summary_response(
    summary: FxRateSummaryRecord,
    refresh_status: String,
    refresh_error: Option<String>,
) -> FxRateSummaryResponse {
    FxRateSummaryResponse {
        target_currency: summary.target_currency,
        rates: summary
            .rates
            .into_iter()
            .map(to_fx_rate_summary_item_response)
            .collect(),
        last_updated: summary.last_updated,
        refresh_status,
        refresh_error,
    }
}

pub(super) fn to_fx_rate_detail_response(
    rate: FxRateDetailRecord,
    refresh_status: String,
    refresh_error: Option<String>,
) -> FxRateDetailResponse {
    FxRateDetailResponse {
        from_currency: rate.from_currency,
        to_currency: rate.to_currency,
        rate: compact_decimal_output(&rate.rate.to_string()),
        updated_at: rate.updated_at,
        refresh_status,
        refresh_error,
    }
}

pub(super) fn to_portfolio_summary_response(
    summary: PortfolioSummaryRecord,
    refresh_status: String,
    refresh_error: Option<String>,
) -> PortfolioSummaryResponse {
    PortfolioSummaryResponse {
        display_currency: summary.display_currency,
        total_value_status: summary.total_value_status.as_str().to_string(),
        total_value_amount: summary
            .total_value_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        account_totals: summary
            .account_totals
            .into_iter()
            .map(to_portfolio_account_total_response)
            .collect(),
        cash_by_currency: summary
            .cash_by_currency
            .into_iter()
            .map(to_portfolio_cash_by_currency_response)
            .collect(),
        fx_last_updated: summary.fx_last_updated,
        fx_refresh_status: refresh_status,
        fx_refresh_error: refresh_error,
        allocation_totals: summary
            .allocation_totals
            .into_iter()
            .map(to_portfolio_allocation_slice_response)
            .collect(),
        allocation_is_partial: summary.allocation_is_partial,
    }
}

fn to_portfolio_allocation_slice_response(
    slice: PortfolioAllocationSliceRecord,
) -> PortfolioAllocationSliceResponse {
    PortfolioAllocationSliceResponse {
        label: slice.label,
        amount: normalize_amount_output(&slice.amount.to_string()),
    }
}

fn to_fx_rate_summary_item_response(rate: FxRateSummaryItemRecord) -> FxRateSummaryItemResponse {
    FxRateSummaryItemResponse {
        currency: rate.from_currency,
        rate: compact_decimal_output(&rate.rate.to_string()),
    }
}

fn to_portfolio_account_total_response(
    account: PortfolioAccountTotalRecord,
) -> PortfolioAccountTotalResponse {
    PortfolioAccountTotalResponse {
        id: account.id.as_i64(),
        name: account.name.to_string(),
        account_type: account.account_type.as_str().to_string(),
        summary_status: account.summary_status.as_str().to_string(),
        cash_total_amount: account
            .cash_total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        asset_total_amount: account
            .asset_total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        total_amount: account
            .total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        total_currency: account.total_currency,
    }
}

fn to_portfolio_cash_by_currency_response(
    balance: PortfolioCashByCurrencyRecord,
) -> PortfolioCashByCurrencyResponse {
    PortfolioCashByCurrencyResponse {
        currency: balance.currency,
        amount: normalize_amount_output(&balance.amount.to_string()),
        converted_amount: balance
            .converted_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
    }
}
