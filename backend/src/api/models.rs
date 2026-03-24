use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::collections::BTreeMap;

use crate::{AssetType, Currency, SharedFxRefreshStatus, storage::StorageError};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub fx_refresh_status: SharedFxRefreshStatus,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct CreateAccountRequest {
    pub name: String,
    pub account_type: String,
    pub base_currency: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct CreateAssetRequest {
    pub symbol: String,
    pub name: String,
    pub asset_type: String,
    pub isin: Option<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct UpsertBalanceRequest {
    pub amount: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct CreateAssetTransactionRequest {
    pub account_id: i64,
    pub asset_id: i64,
    pub transaction_type: String,
    pub trade_date: String,
    pub quantity: String,
    pub unit_price: String,
    pub currency_code: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct AssetTransactionListQuery {
    pub account_id: Option<i64>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AccountSummaryResponse {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub base_currency: Currency,
    pub summary_status: String,
    pub total_amount: Option<String>,
    pub total_currency: Option<Currency>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct BalanceResponse {
    pub currency: Currency,
    pub amount: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct CurrencyResponse {
    pub code: Currency,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AssetResponse {
    pub id: i64,
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub isin: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct CreatedAssetResponse {
    pub id: i64,
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub isin: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AssetTransactionResponse {
    pub id: i64,
    pub account_id: i64,
    pub asset_id: i64,
    pub transaction_type: String,
    pub trade_date: String,
    pub quantity: String,
    pub unit_price: String,
    pub currency_code: Currency,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AssetPositionResponse {
    pub account_id: i64,
    pub asset_id: i64,
    pub quantity: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct FxRateSummaryItemResponse {
    pub currency: Currency,
    pub rate: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct FxRateSummaryResponse {
    pub target_currency: Currency,
    pub rates: Vec<FxRateSummaryItemResponse>,
    pub last_updated: Option<String>,
    pub refresh_status: String,
    pub refresh_error: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct FxRateDetailResponse {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub rate: String,
    pub updated_at: String,
    pub refresh_status: String,
    pub refresh_error: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PortfolioSummaryResponse {
    pub display_currency: Currency,
    pub total_value_status: String,
    pub total_value_amount: Option<String>,
    pub account_totals: Vec<PortfolioAccountTotalResponse>,
    pub cash_by_currency: Vec<PortfolioCashByCurrencyResponse>,
    pub fx_last_updated: Option<String>,
    pub fx_refresh_status: String,
    pub fx_refresh_error: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PortfolioAccountTotalResponse {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub summary_status: String,
    pub total_amount: Option<String>,
    pub total_currency: Currency,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PortfolioCashByCurrencyResponse {
    pub currency: Currency,
    pub amount: String,
    pub converted_amount: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AccountDetailResponse {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub base_currency: Currency,
    pub created_at: String,
    pub balances: Vec<BalanceResponse>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct ApiErrorResponse {
    pub error: &'static str,
    pub message: &'static str,
}

pub struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) body: ApiErrorResponse,
}

pub struct CreateAssetApiError {
    pub(crate) status: StatusCode,
    pub(crate) body: Value,
}

impl ApiError {
    pub(crate) fn validation(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ApiErrorResponse {
                error: "validation_error",
                message,
            },
        }
    }

    pub(crate) fn not_found(message: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            body: ApiErrorResponse {
                error: "not_found",
                message,
            },
        }
    }

    pub(crate) fn conflict(message: &'static str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            body: ApiErrorResponse {
                error: "conflict",
                message,
            },
        }
    }

    pub(crate) fn internal_server_error() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: ApiErrorResponse {
                error: "internal_error",
                message: "Internal server error",
            },
        }
    }
}

impl CreateAssetApiError {
    pub(crate) fn validation(field_errors: BTreeMap<String, Vec<String>>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            body: json!({
                "message": "Asset validation failed",
                "field_errors": field_errors,
            }),
        }
    }

    pub(crate) fn bad_request(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: json!({ "message": message }),
        }
    }

    pub(crate) fn not_found(message: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            body: json!({ "message": message }),
        }
    }

    pub(crate) fn internal_server_error() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: json!({ "message": "Failed to create asset" }),
        }
    }

    pub(crate) fn duplicate(field: &str, message: &str) -> Self {
        let mut field_errors = BTreeMap::new();
        field_errors.insert(field.to_string(), vec![message.to_string()]);
        Self::validation(field_errors)
    }
}

impl From<StorageError> for ApiError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::Validation(message) => Self::validation(message),
            StorageError::Internal(_) => Self::internal_server_error(),
            StorageError::Database(sqlx::Error::RowNotFound) => {
                Self::not_found("Resource not found")
            }
            StorageError::Database(_) => Self::internal_server_error(),
        }
    }
}

impl From<StorageError> for CreateAssetApiError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::Validation(_) => Self::internal_server_error(),
            StorageError::Internal(_) => Self::internal_server_error(),
            StorageError::Database(sqlx::Error::Database(error)) => {
                let message = error.message();

                if message.contains("UNIQUE constraint failed: assets.symbol") {
                    return Self::duplicate("symbol", "Symbol must be unique");
                }

                if message.contains("UNIQUE constraint failed: assets.isin") {
                    return Self::duplicate("isin", "ISIN must be unique");
                }

                Self::internal_server_error()
            }
            StorageError::Database(_) => Self::internal_server_error(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl IntoResponse for CreateAssetApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

pub async fn read_fx_refresh_status(status: &SharedFxRefreshStatus) -> (String, Option<String>) {
    let status = status.read().await;
    (
        status.availability.as_str().to_string(),
        status.last_error.clone(),
    )
}
