use axum::{
    Json,
    extract::{Path as AxumPath, State},
};

use super::{
    ApiError, AppState, FxRateDetailResponse, FxRateSummaryResponse,
    common::{to_fx_rate_detail_response, to_fx_rate_summary_response},
    read_fx_refresh_status,
};
use crate::{Currency, PRODUCT_BASE_CURRENCY, get_latest_fx_rate, list_fx_rate_summary};

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
