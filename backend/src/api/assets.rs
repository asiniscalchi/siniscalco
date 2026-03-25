use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path as AxumPath, State, rejection::JsonRejection},
    http::StatusCode,
};
use tracing::warn;

use super::{
    ApiError, AppState, AssetResponse, CreateAssetApiError, CreateAssetRequest,
    CreatedAssetResponse,
    common::{map_create_asset_json_rejection, to_asset_response, to_created_asset_response},
};
use crate::{
    AssetId, AssetType, CreateAssetInput, UpdateAssetInput, create_asset, get_asset, list_assets,
    refresh_single_asset_price, storage::StorageError, update_asset,
};

pub(crate) async fn list_assets_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<AssetResponse>>, ApiError> {
    let assets = list_assets(&state.pool).await.map_err(ApiError::from)?;

    Ok(Json(assets.into_iter().map(to_asset_response).collect()))
}

pub(crate) async fn create_asset_handler(
    State(state): State<AppState>,
    request: Result<Json<CreateAssetRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreatedAssetResponse>), CreateAssetApiError> {
    let Json(request) = request.map_err(map_create_asset_json_rejection)?;
    let input = validate_create_asset_request(request)?;

    let asset_id = create_asset(&state.pool, input)
        .await
        .map_err(CreateAssetApiError::from)?;
    refresh_asset_price_on_write(&state, asset_id).await;
    let asset = get_asset(&state.pool, asset_id)
        .await
        .map_err(CreateAssetApiError::from)?;

    Ok((StatusCode::CREATED, Json(to_created_asset_response(asset))))
}

pub(crate) async fn get_asset_handler(
    State(state): State<AppState>,
    AxumPath((asset_id,)): AxumPath<(i64,)>,
) -> Result<Json<CreatedAssetResponse>, ApiError> {
    let asset_id = AssetId::try_from(asset_id).map_err(ApiError::from)?;
    let asset = get_asset(&state.pool, asset_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Asset not found")
            }
            other => ApiError::from(other),
        })?;

    Ok(Json(to_created_asset_response(asset)))
}

pub(crate) async fn update_asset_handler(
    State(state): State<AppState>,
    AxumPath((asset_id,)): AxumPath<(i64,)>,
    request: Result<Json<CreateAssetRequest>, JsonRejection>,
) -> Result<Json<CreatedAssetResponse>, CreateAssetApiError> {
    let Json(request) = request.map_err(map_create_asset_json_rejection)?;
    let asset_id = AssetId::try_from(asset_id).map_err(CreateAssetApiError::from)?;
    let input = validate_create_asset_request(request)?;

    update_asset(
        &state.pool,
        asset_id,
        UpdateAssetInput {
            symbol: input.symbol,
            name: input.name,
            asset_type: input.asset_type,
            quote_symbol: input.quote_symbol,
            isin: input.isin,
        },
    )
    .await
    .map_err(|error| match error {
        StorageError::Database(sqlx::Error::RowNotFound) => {
            CreateAssetApiError::not_found("Asset not found")
        }
        other => CreateAssetApiError::from(other),
    })?;
    refresh_asset_price_on_write(&state, asset_id).await;
    let asset = get_asset(&state.pool, asset_id)
        .await
        .map_err(CreateAssetApiError::from)?;

    Ok(Json(to_created_asset_response(asset)))
}

pub(crate) async fn delete_asset_handler(
    State(state): State<AppState>,
    AxumPath((asset_id,)): AxumPath<(i64,)>,
) -> Result<StatusCode, ApiError> {
    let asset_id = AssetId::try_from(asset_id).map_err(ApiError::from)?;

    crate::delete_asset(&state.pool, asset_id)
        .await
        .map_err(|error| match error {
            StorageError::Database(sqlx::Error::RowNotFound) => {
                ApiError::not_found("Asset not found")
            }
            StorageError::Database(sqlx::Error::Database(database_error))
                if database_error
                    .message()
                    .contains("FOREIGN KEY constraint failed") =>
            {
                ApiError::conflict("Asset has transactions and cannot be deleted")
            }
            other => ApiError::from(other),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn refresh_asset_price_on_write(state: &AppState, asset_id: AssetId) {
    let client = reqwest::Client::new();
    if let Err(error) = refresh_single_asset_price(
        &state.pool,
        &client,
        &state.asset_price_refresh_config,
        asset_id,
    )
    .await
    {
        warn!(
            asset_id = asset_id.as_i64(),
            error = %error,
            "failed to refresh immediate asset price"
        );
    }
}

fn validate_create_asset_request(
    request: CreateAssetRequest,
) -> Result<CreateAssetInput, CreateAssetApiError> {
    let mut field_errors = BTreeMap::new();

    let symbol = request.symbol.trim().to_uppercase();
    if symbol.is_empty() {
        field_errors.insert("symbol".to_string(), vec!["Symbol is required".to_string()]);
    }

    let name = request.name.trim().to_string();
    if name.is_empty() {
        field_errors.insert("name".to_string(), vec!["Name is required".to_string()]);
    }

    let asset_type_value = request.asset_type.trim().to_string();
    if asset_type_value.is_empty() {
        field_errors.insert(
            "asset_type".to_string(),
            vec!["Asset type is required".to_string()],
        );
    }

    let normalized_quote_symbol = request.quote_symbol.and_then(|quote_symbol| {
        let trimmed = quote_symbol.trim().to_uppercase();
        (!trimmed.is_empty()).then_some(trimmed)
    });

    let normalized_isin = request.isin.and_then(|isin| {
        let trimmed = isin.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    });

    let asset_type = match AssetType::try_from(asset_type_value.as_str()) {
        Ok(asset_type) => Some(asset_type),
        Err(_) if !asset_type_value.is_empty() => {
            field_errors.insert(
                "asset_type".to_string(),
                vec![format!(
                    "Asset type must be one of: STOCK, ETF, BOND, CRYPTO, CASH_EQUIVALENT, OTHER"
                )],
            );
            None
        }
        Err(_) => None,
    };

    if !field_errors.is_empty() {
        return Err(CreateAssetApiError::validation(field_errors));
    }

    Ok(CreateAssetInput {
        symbol: symbol
            .as_str()
            .try_into()
            .map_err(CreateAssetApiError::from)?,
        name: name
            .as_str()
            .try_into()
            .map_err(CreateAssetApiError::from)?,
        asset_type: asset_type.expect("validated asset type should exist"),
        quote_symbol: normalized_quote_symbol,
        isin: normalized_isin,
    })
}
