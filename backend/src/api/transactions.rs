use axum::{
    Json,
    extract::{
        Path as AxumPath, Query, State, rejection::JsonRejection, rejection::QueryRejection,
    },
    http::StatusCode,
};

use super::{
    ApiError, AppState, AssetTransactionListQuery, AssetTransactionResponse,
    CreateAssetTransactionRequest,
    common::{map_json_rejection, map_query_rejection, to_asset_transaction_response},
};
use crate::{
    AccountId, AssetId, AssetQuantity, AssetTransactionType, AssetUnitPrice,
    CreateAssetTransactionInput, Currency, TradeDate, UpdateAssetTransactionInput,
    create_asset_transaction, get_account, get_asset, get_transaction, list_asset_transactions,
    list_transactions, storage::StorageError, update_asset_transaction,
};

pub(crate) async fn create_asset_transaction_handler(
    State(state): State<AppState>,
    request: Result<Json<CreateAssetTransactionRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<AssetTransactionResponse>), ApiError> {
    let Json(request) = request.map_err(map_json_rejection)?;
    let input = parse_create_transaction_request(request)?;

    ensure_account_exists(&state, input.account_id).await?;
    ensure_asset_exists(&state, input.asset_id).await?;

    let transaction = create_asset_transaction(&state.pool, input)
        .await
        .map_err(ApiError::from)?;

    Ok((
        StatusCode::CREATED,
        Json(to_asset_transaction_response(transaction)),
    ))
}

pub(crate) async fn create_transaction_handler(
    state: State<AppState>,
    request: Result<Json<CreateAssetTransactionRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<AssetTransactionResponse>), ApiError> {
    create_asset_transaction_handler(state, request).await
}

pub(crate) async fn list_transactions_handler(
    State(state): State<AppState>,
    query: Result<Query<AssetTransactionListQuery>, QueryRejection>,
) -> Result<Json<Vec<AssetTransactionResponse>>, ApiError> {
    let Query(query) = query.map_err(map_query_rejection)?;
    let transactions = if let Some(account_id) = query.account_id {
        let account_id = AccountId::try_from(account_id).map_err(ApiError::from)?;

        ensure_account_exists(&state, account_id).await?;

        list_asset_transactions(&state.pool, account_id)
            .await
            .map_err(ApiError::from)?
    } else {
        list_transactions(&state.pool)
            .await
            .map_err(ApiError::from)?
    };

    Ok(Json(
        transactions
            .into_iter()
            .map(to_asset_transaction_response)
            .collect(),
    ))
}

pub(crate) async fn get_transaction_handler(
    State(state): State<AppState>,
    AxumPath((transaction_id,)): AxumPath<(i64,)>,
) -> Result<Json<AssetTransactionResponse>, ApiError> {
    let transaction = get_transaction(&state.pool, transaction_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Asset transaction not found")
            }
            other => ApiError::from(other),
        })?;

    Ok(Json(to_asset_transaction_response(transaction)))
}

pub(crate) async fn update_transaction_handler(
    State(state): State<AppState>,
    AxumPath((transaction_id,)): AxumPath<(i64,)>,
    request: Result<Json<CreateAssetTransactionRequest>, JsonRejection>,
) -> Result<Json<AssetTransactionResponse>, ApiError> {
    let Json(request) = request.map_err(map_json_rejection)?;
    let input = parse_update_transaction_request(request)?;

    ensure_account_exists(&state, input.account_id).await?;
    ensure_asset_exists(&state, input.asset_id).await?;

    let transaction = update_asset_transaction(&state.pool, transaction_id, input)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Asset transaction not found")
            }
            other => ApiError::from(other),
        })?;

    Ok(Json(to_asset_transaction_response(transaction)))
}

pub(crate) async fn delete_transaction_handler(
    State(state): State<AppState>,
    AxumPath((transaction_id,)): AxumPath<(i64,)>,
) -> Result<StatusCode, ApiError> {
    crate::delete_asset_transaction(&state.pool, transaction_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Asset transaction not found")
            }
            other => ApiError::from(other),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

fn parse_create_transaction_request(
    request: CreateAssetTransactionRequest,
) -> Result<CreateAssetTransactionInput, ApiError> {
    let parsed = parse_transaction_fields(request)?;

    Ok(CreateAssetTransactionInput {
        account_id: parsed.account_id,
        asset_id: parsed.asset_id,
        transaction_type: parsed.transaction_type,
        trade_date: parsed.trade_date,
        quantity: parsed.quantity,
        unit_price: parsed.unit_price,
        currency_code: parsed.currency_code,
        notes: parsed.notes,
    })
}

fn parse_update_transaction_request(
    request: CreateAssetTransactionRequest,
) -> Result<UpdateAssetTransactionInput, ApiError> {
    let parsed = parse_transaction_fields(request)?;

    Ok(UpdateAssetTransactionInput {
        account_id: parsed.account_id,
        asset_id: parsed.asset_id,
        transaction_type: parsed.transaction_type,
        trade_date: parsed.trade_date,
        quantity: parsed.quantity,
        unit_price: parsed.unit_price,
        currency_code: parsed.currency_code,
        notes: parsed.notes,
    })
}

fn parse_transaction_fields(
    request: CreateAssetTransactionRequest,
) -> Result<ParsedTransactionInput, ApiError> {
    let account_id = AccountId::try_from(request.account_id).map_err(ApiError::from)?;
    let asset_id = AssetId::try_from(request.asset_id).map_err(ApiError::from)?;
    let transaction_type = AssetTransactionType::try_from(request.transaction_type.as_str())
        .map_err(ApiError::from)?;
    let trade_date = TradeDate::try_from(request.trade_date.as_str()).map_err(ApiError::from)?;
    let quantity = AssetQuantity::try_from(request.quantity.as_str()).map_err(ApiError::from)?;
    let unit_price =
        AssetUnitPrice::try_from(request.unit_price.as_str()).map_err(ApiError::from)?;
    let currency_code =
        Currency::try_from(request.currency_code.as_str()).map_err(ApiError::from)?;

    Ok(ParsedTransactionInput {
        account_id,
        asset_id,
        transaction_type,
        trade_date,
        quantity,
        unit_price,
        currency_code,
        notes: request.notes,
    })
}

struct ParsedTransactionInput {
    account_id: AccountId,
    asset_id: AssetId,
    transaction_type: AssetTransactionType,
    trade_date: TradeDate,
    quantity: AssetQuantity,
    unit_price: AssetUnitPrice,
    currency_code: Currency,
    notes: Option<String>,
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

async fn ensure_asset_exists(state: &AppState, asset_id: AssetId) -> Result<(), ApiError> {
    get_asset(&state.pool, asset_id)
        .await
        .map(|_| ())
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Asset not found")
            }
            other => ApiError::from(other),
        })
}
