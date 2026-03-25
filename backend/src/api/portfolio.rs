use axum::{Json, extract::State};

use super::{
    ApiError, AppState, PortfolioSummaryResponse, common::to_portfolio_summary_response,
    read_fx_refresh_status,
};
use crate::{PRODUCT_BASE_CURRENCY, get_portfolio_summary};

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
