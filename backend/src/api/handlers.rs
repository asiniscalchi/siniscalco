use axum::{
    Json,
    extract::{Path as AxumPath, State, rejection::JsonRejection},
    http::StatusCode,
};

use crate::api::models::*;
use crate::{
    AccountBalanceRecord, AccountId, AccountName, AccountRecord, AccountSummaryRecord,
    AccountSummaryStatus, AccountType, Amount, CreateAccountInput, Currency, CurrencyRecord,
    FxRateDetailRecord, FxRateSummaryItemRecord, FxRateSummaryRecord, PRODUCT_BASE_CURRENCY,
    PortfolioAccountTotalRecord, PortfolioCashByCurrencyRecord, PortfolioSummaryRecord,
    UpsertAccountBalanceInput, UpsertOutcome, compact_decimal_output, delete_account,
    delete_account_balance, get_account, get_latest_fx_rate, get_portfolio_summary,
    list_account_balances, list_account_summaries, list_currencies, list_fx_rate_summary,
    normalize_amount_output, storage::StorageError, upsert_account_balance,
};

pub(crate) async fn health() -> &'static str {
    "ok"
}

pub(crate) async fn create_account_handler(
    State(state): State<AppState>,
    request: Result<Json<CreateAccountRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<AccountSummaryResponse>), ApiError> {
    let Json(request) = request.map_err(map_json_rejection)?;
    let name = AccountName::try_from(request.name.as_str()).map_err(ApiError::from)?;
    let account_type =
        AccountType::try_from(request.account_type.as_str()).map_err(ApiError::from)?;
    let base_currency =
        Currency::try_from(request.base_currency.as_str()).map_err(ApiError::from)?;

    let account_id = crate::create_account(
        &state.pool,
        CreateAccountInput {
            name,
            account_type,
            base_currency,
        },
    )
    .await
    .map_err(ApiError::from)?;

    let account = get_account(&state.pool, account_id)
        .await
        .map_err(ApiError::from)?;

    Ok((
        StatusCode::CREATED,
        Json(to_created_account_summary_response(account)),
    ))
}

pub(crate) async fn list_currencies_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<CurrencyResponse>>, ApiError> {
    let currencies = list_currencies(&state.pool).await.map_err(ApiError::from)?;

    Ok(Json(
        currencies.into_iter().map(to_currency_response).collect(),
    ))
}

pub(crate) async fn list_accounts_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<AccountSummaryResponse>>, ApiError> {
    let accounts = list_account_summaries(&state.pool)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(
        accounts
            .into_iter()
            .map(to_account_summary_response)
            .collect(),
    ))
}

pub(crate) async fn get_fx_rate_summary_handler(
    State(state): State<AppState>,
) -> Result<Json<FxRateSummaryResponse>, ApiError> {
    let summary = list_fx_rate_summary(&state.pool, PRODUCT_BASE_CURRENCY)
        .await
        .map_err(ApiError::from)?;
    let (refresh_status, refresh_error) = read_fx_refresh_status(&state.fx_refresh_status).await;

    Ok(Json(to_fx_rate_summary_response(
        summary,
        refresh_status,
        refresh_error,
    )))
}

pub(crate) async fn get_fx_rate_handler(
    State(state): State<AppState>,
    AxumPath((from_currency, to_currency)): AxumPath<(String, String)>,
) -> Result<Json<FxRateDetailResponse>, ApiError> {
    let from_currency = Currency::try_from(from_currency.as_str()).map_err(ApiError::from)?;
    let to_currency = Currency::try_from(to_currency.as_str()).map_err(ApiError::from)?;
    let rate = get_latest_fx_rate(&state.pool, from_currency, to_currency)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found("FX rate not found"))?;
    let (refresh_status, refresh_error) = read_fx_refresh_status(&state.fx_refresh_status).await;

    Ok(Json(to_fx_rate_detail_response(
        rate,
        refresh_status,
        refresh_error,
    )))
}

pub(crate) async fn get_portfolio_summary_handler(
    State(state): State<AppState>,
) -> Result<Json<PortfolioSummaryResponse>, ApiError> {
    let summary = get_portfolio_summary(&state.pool, PRODUCT_BASE_CURRENCY)
        .await
        .map_err(ApiError::from)?;
    let (refresh_status, refresh_error) = read_fx_refresh_status(&state.fx_refresh_status).await;

    Ok(Json(to_portfolio_summary_response(
        summary,
        refresh_status,
        refresh_error,
    )))
}

pub(crate) async fn get_account_handler(
    State(state): State<AppState>,
    AxumPath((account_id,)): AxumPath<(i64,)>,
) -> Result<Json<AccountDetailResponse>, ApiError> {
    let account_id = AccountId::try_from(account_id).map_err(ApiError::from)?;

    let account = get_account(&state.pool, account_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Account not found")
            }
            other => ApiError::from(other),
        })?;
    let balances = list_account_balances(&state.pool, account_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(to_account_detail_response(account, balances)))
}

pub(crate) async fn upsert_account_balance_handler(
    State(state): State<AppState>,
    AxumPath((account_id, currency)): AxumPath<(i64, String)>,
    request: Result<Json<UpsertBalanceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<BalanceResponse>), ApiError> {
    let Json(request) = request.map_err(map_json_rejection)?;
    let account_id = AccountId::try_from(account_id).map_err(ApiError::from)?;
    let currency = Currency::try_from(currency.as_str()).map_err(ApiError::from)?;
    let amount = Amount::try_from(request.amount.as_str()).map_err(ApiError::from)?;

    get_account(&state.pool, account_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Account not found")
            }
            other => ApiError::from(other),
        })?;

    let outcome = upsert_account_balance(
        &state.pool,
        UpsertAccountBalanceInput {
            account_id,
            currency,
            amount,
        },
    )
    .await
    .map_err(ApiError::from)?;

    let balance = list_account_balances(&state.pool, account_id)
        .await
        .map_err(ApiError::from)?
        .into_iter()
        .find(|balance| balance.currency == currency)
        .ok_or_else(ApiError::internal_server_error)?;

    let status = match outcome {
        UpsertOutcome::Created => StatusCode::CREATED,
        UpsertOutcome::Updated => StatusCode::OK,
    };

    Ok((status, Json(to_balance_response(balance))))
}

pub(crate) async fn delete_account_balance_handler(
    State(state): State<AppState>,
    AxumPath((account_id, currency)): AxumPath<(i64, String)>,
) -> Result<StatusCode, ApiError> {
    let account_id = AccountId::try_from(account_id).map_err(ApiError::from)?;
    let currency = Currency::try_from(currency.as_str()).map_err(ApiError::from)?;

    get_account(&state.pool, account_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Account not found")
            }
            other => ApiError::from(other),
        })?;

    delete_account_balance(&state.pool, account_id, currency)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Balance not found")
            }
            other => ApiError::from(other),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn delete_account_handler(
    State(state): State<AppState>,
    AxumPath((account_id,)): AxumPath<(i64,)>,
) -> Result<StatusCode, ApiError> {
    let account_id = AccountId::try_from(account_id).map_err(ApiError::from)?;

    delete_account(&state.pool, account_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Account not found")
            }
            other => ApiError::from(other),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

fn to_account_summary_response(account: AccountSummaryRecord) -> AccountSummaryResponse {
    AccountSummaryResponse {
        id: account.id.as_i64(),
        name: account.name.to_string(),
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        summary_status: account.summary_status.as_str().to_string(),
        total_amount: account
            .total_amount
            .map(|amount| normalize_amount_output(&amount.to_string())),
        total_currency: account.total_currency,
    }
}

fn to_created_account_summary_response(account: AccountRecord) -> AccountSummaryResponse {
    AccountSummaryResponse {
        id: account.id.as_i64(),
        name: account.name.to_string(),
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        summary_status: AccountSummaryStatus::Ok.as_str().to_string(),
        total_amount: Some("0.00000000".to_string()),
        total_currency: Some(account.base_currency),
    }
}

fn to_account_detail_response(
    account: AccountRecord,
    balances: Vec<AccountBalanceRecord>,
) -> AccountDetailResponse {
    AccountDetailResponse {
        id: account.id.as_i64(),
        name: account.name.to_string(),
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        created_at: account.created_at,
        balances: balances.into_iter().map(to_balance_response).collect(),
    }
}

fn to_balance_response(balance: AccountBalanceRecord) -> BalanceResponse {
    BalanceResponse {
        currency: balance.currency,
        amount: normalize_amount_output(&balance.amount.to_string()),
        updated_at: balance.updated_at,
    }
}

fn to_currency_response(currency: CurrencyRecord) -> CurrencyResponse {
    CurrencyResponse {
        code: currency.code,
    }
}

fn to_fx_rate_summary_response(
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

fn to_fx_rate_detail_response(
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

fn to_portfolio_summary_response(
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
    }
}

fn map_json_rejection(rejection: JsonRejection) -> ApiError {
    match rejection {
        JsonRejection::JsonSyntaxError(_) | JsonRejection::JsonDataError(_) => {
            ApiError::validation("Malformed JSON body")
        }
        JsonRejection::MissingJsonContentType(_) => ApiError::validation("Expected JSON body"),
        _ => ApiError::validation("Invalid JSON body"),
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
    }
}
