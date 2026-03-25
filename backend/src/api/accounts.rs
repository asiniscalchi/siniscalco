use axum::{
    Json,
    extract::{Path as AxumPath, State, rejection::JsonRejection},
    http::StatusCode,
};

use super::{
    AccountDetailResponse, AccountSummaryResponse, ApiError, AppState, BalanceResponse,
    CreateAccountRequest,
    common::{
        map_json_rejection, to_account_detail_response, to_account_summary_response,
        to_balance_response,
    },
};
use crate::{
    AccountId, AccountName, AccountType, Amount, CreateAccountInput, Currency, UpdateAccountInput,
    UpsertAccountBalanceInput, UpsertOutcome, delete_account, delete_account_balance, get_account,
    get_account_value_summary, list_account_balances, list_account_positions,
    list_account_summaries, storage::StorageError, update_account, upsert_account_balance,
};

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
    let value_summary = get_account_value_summary(&state.pool, &account)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(to_account_detail_response(
        account,
        balances,
        value_summary,
    )))
}

pub(crate) async fn update_account_handler(
    State(state): State<AppState>,
    AxumPath((account_id,)): AxumPath<(i64,)>,
    request: Result<Json<CreateAccountRequest>, JsonRejection>,
) -> Result<Json<AccountDetailResponse>, ApiError> {
    let Json(request) = request.map_err(map_json_rejection)?;
    let account_id = AccountId::try_from(account_id).map_err(ApiError::from)?;
    let name = AccountName::try_from(request.name.as_str()).map_err(ApiError::from)?;
    let account_type =
        AccountType::try_from(request.account_type.as_str()).map_err(ApiError::from)?;
    let base_currency =
        Currency::try_from(request.base_currency.as_str()).map_err(ApiError::from)?;

    let account = update_account(
        &state.pool,
        account_id,
        UpdateAccountInput {
            name,
            account_type,
            base_currency,
        },
    )
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
    let value_summary = get_account_value_summary(&state.pool, &account)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(to_account_detail_response(
        account,
        balances,
        value_summary,
    )))
}

pub(crate) async fn list_account_balances_handler(
    State(state): State<AppState>,
    AxumPath((account_id,)): AxumPath<(i64,)>,
) -> Result<Json<Vec<BalanceResponse>>, ApiError> {
    let account_id = AccountId::try_from(account_id).map_err(ApiError::from)?;

    ensure_account_exists(&state, account_id).await?;

    let balances = list_account_balances(&state.pool, account_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(
        balances.into_iter().map(to_balance_response).collect(),
    ))
}

pub(crate) async fn list_account_positions_handler(
    State(state): State<AppState>,
    AxumPath((account_id,)): AxumPath<(i64,)>,
) -> Result<Json<Vec<super::AssetPositionResponse>>, ApiError> {
    let account_id = AccountId::try_from(account_id).map_err(ApiError::from)?;

    ensure_account_exists(&state, account_id).await?;

    let positions = list_account_positions(&state.pool, account_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(
        positions
            .into_iter()
            .map(super::common::to_asset_position_response)
            .collect(),
    ))
}

pub(crate) async fn upsert_account_balance_handler(
    State(state): State<AppState>,
    AxumPath((account_id, currency)): AxumPath<(i64, String)>,
    request: Result<Json<super::UpsertBalanceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<BalanceResponse>), ApiError> {
    let Json(request) = request.map_err(map_json_rejection)?;
    let account_id = AccountId::try_from(account_id).map_err(ApiError::from)?;
    let currency = Currency::try_from(currency.as_str()).map_err(ApiError::from)?;
    let amount = Amount::try_from(request.amount.as_str()).map_err(ApiError::from)?;

    ensure_account_exists(&state, account_id).await?;

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

    ensure_account_exists(&state, account_id).await?;

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

async fn ensure_account_exists(state: &AppState, account_id: AccountId) -> Result<(), ApiError> {
    get_account(&state.pool, account_id)
        .await
        .map(|_| ())
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Account not found")
            }
            other => ApiError::from(other),
        })
}

fn to_created_account_summary_response(account: crate::AccountRecord) -> AccountSummaryResponse {
    AccountSummaryResponse {
        id: account.id.as_i64(),
        name: account.name.to_string(),
        account_type: account.account_type.as_str().to_string(),
        base_currency: account.base_currency,
        summary_status: crate::AccountSummaryStatus::Ok.as_str().to_string(),
        cash_total_amount: Some("0.000000".to_string()),
        asset_total_amount: Some("0.000000".to_string()),
        total_amount: Some("0.000000".to_string()),
        total_currency: Some(account.base_currency),
    }
}
