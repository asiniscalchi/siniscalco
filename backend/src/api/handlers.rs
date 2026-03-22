use axum::{
    Json,
    extract::{Path as AxumPath, State},
    http::StatusCode,
};

use crate::api::models::*;
use crate::{
    AccountBalanceRecord, AccountRecord, AccountSummaryRecord, AccountSummaryStatus, AccountType,
    CreateAccountInput, Currency, CurrencyRecord, FxRateSummaryItemRecord, FxRateSummaryRecord,
    UpsertAccountBalanceInput, UpsertOutcome, delete_account, delete_account_balance, get_account,
    list_account_balances, list_account_summaries, list_currencies, list_fx_rate_summary,
    normalize_amount_output, storage::StorageError, upsert_account_balance,
};

pub(crate) async fn health() -> &'static str {
    "ok"
}

pub(crate) async fn create_account_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<AccountSummaryResponse>), ApiError> {
    let account_type =
        AccountType::try_from(request.account_type.as_str()).map_err(ApiError::from)?;
    let base_currency =
        Currency::try_from(request.base_currency.as_str()).map_err(ApiError::from)?;

    let account_id = crate::create_account(
        &state.pool,
        CreateAccountInput {
            name: &request.name,
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
    let summary = list_fx_rate_summary(&state.pool, Currency::Eur)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(to_fx_rate_summary_response(summary)))
}

pub(crate) async fn get_account_handler(
    State(state): State<AppState>,
    AxumPath((account_id,)): AxumPath<(i64,)>,
) -> Result<Json<AccountDetailResponse>, ApiError> {
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
    Json(request): Json<UpsertBalanceRequest>,
) -> Result<(StatusCode, Json<BalanceResponse>), ApiError> {
    let currency = Currency::try_from(currency.as_str()).map_err(ApiError::from)?;

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
            amount: &request.amount,
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
        id: account.id,
        name: account.name,
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        summary_status: account.summary_status.as_str().to_string(),
        total_amount: account.total_amount,
        total_currency: account.total_currency,
    }
}

fn to_created_account_summary_response(account: AccountRecord) -> AccountSummaryResponse {
    AccountSummaryResponse {
        id: account.id,
        name: account.name,
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
        id: account.id,
        name: account.name,
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        created_at: account.created_at,
        balances: balances.into_iter().map(to_balance_response).collect(),
    }
}

fn to_balance_response(balance: AccountBalanceRecord) -> BalanceResponse {
    BalanceResponse {
        currency: balance.currency,
        amount: normalize_amount_output(&balance.amount),
        updated_at: balance.updated_at,
    }
}

fn to_currency_response(currency: CurrencyRecord) -> CurrencyResponse {
    CurrencyResponse {
        code: currency.code,
    }
}

fn to_fx_rate_summary_response(summary: FxRateSummaryRecord) -> FxRateSummaryResponse {
    FxRateSummaryResponse {
        target_currency: summary.target_currency,
        rates: summary
            .rates
            .into_iter()
            .map(to_fx_rate_summary_item_response)
            .collect(),
        last_updated: summary.last_updated,
    }
}

fn to_fx_rate_summary_item_response(rate: FxRateSummaryItemRecord) -> FxRateSummaryItemResponse {
    FxRateSummaryItemResponse {
        currency: rate.from_currency,
        rate: rate.rate,
    }
}
