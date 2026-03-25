use axum::{Json, extract::State};

use super::{ApiError, AppState, CurrencyResponse, common::to_currency_response};
use crate::list_currencies;

pub(crate) async fn list_currencies_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<CurrencyResponse>>, ApiError> {
    let currencies = list_currencies(&state.pool).await.map_err(ApiError::from)?;

    Ok(Json(
        currencies.into_iter().map(to_currency_response).collect(),
    ))
}
