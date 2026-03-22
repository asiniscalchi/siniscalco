use std::{error::Error, fmt};

use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;

use super::account_id::AccountId;
use super::account_name::AccountName;
use super::amount::Amount;
use super::currency::Currency;
use super::fx_rate::FxRate;

pub const UTC_TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountType {
    Bank,
    Broker,
}

impl AccountType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bank => "bank",
            Self::Broker => "broker",
        }
    }
}

impl TryFrom<&str> for AccountType {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "bank" => Ok(Self::Bank),
            "broker" => Ok(Self::Broker),
            _ => Err(StorageError::Validation(
                "account_type must be one of: bank, broker",
            )),
        }
    }
}

#[derive(Debug)]
pub enum StorageError {
    Validation(&'static str),
    Internal(&'static str),
    Database(sqlx::Error),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => f.write_str(message),
            Self::Internal(message) => f.write_str(message),
            Self::Database(error) => write!(f, "{error}"),
        }
    }
}

impl Error for StorageError {}

impl From<sqlx::Error> for StorageError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

pub(crate) fn current_utc_timestamp() -> Result<String, StorageError> {
    OffsetDateTime::now_utc()
        .format(UTC_TIMESTAMP_FORMAT)
        .map_err(|_| StorageError::Validation("failed to generate UTC timestamp"))
}

pub struct CreateAccountInput {
    pub name: AccountName,
    pub account_type: AccountType,
    pub base_currency: Currency,
}

pub struct UpsertAccountBalanceInput {
    pub account_id: AccountId,
    pub currency: Currency,
    pub amount: Amount,
}

pub struct UpsertFxRateInput {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub rate: FxRate,
}

#[derive(Debug, Eq, PartialEq)]
pub enum UpsertOutcome {
    Created,
    Updated,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountRecord {
    pub id: AccountId,
    pub name: AccountName,
    pub account_type: AccountType,
    pub base_currency: Currency,
    pub created_at: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountSummaryStatus {
    Ok,
    ConversionUnavailable,
}

impl AccountSummaryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::ConversionUnavailable => "conversion_unavailable",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountSummaryRecord {
    pub id: AccountId,
    pub name: AccountName,
    pub account_type: AccountType,
    pub base_currency: Currency,
    pub summary_status: AccountSummaryStatus,
    pub total_amount: Option<Amount>,
    pub total_currency: Option<Currency>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountBalanceRecord {
    pub account_id: AccountId,
    pub currency: Currency,
    pub amount: Amount,
    pub updated_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CurrencyRecord {
    pub code: Currency,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FxRateRecord {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub rate: FxRate,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FxRateSummaryItemRecord {
    pub from_currency: Currency,
    pub rate: FxRate,
    pub updated_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FxRateSummaryRecord {
    pub target_currency: Currency,
    pub rates: Vec<FxRateSummaryItemRecord>,
    pub last_updated: Option<String>,
}
