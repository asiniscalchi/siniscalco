use std::{error::Error, fmt};

use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Currency {
    Eur,
    Usd,
    Gbp,
    Chf,
}

impl Currency {
    pub const fn all() -> [Self; 4] {
        [Self::Chf, Self::Eur, Self::Gbp, Self::Usd]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eur => "EUR",
            Self::Usd => "USD",
            Self::Gbp => "GBP",
            Self::Chf => "CHF",
        }
    }
}

impl TryFrom<&str> for Currency {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "EUR" => Ok(Self::Eur),
            "USD" => Ok(Self::Usd),
            "GBP" => Ok(Self::Gbp),
            "CHF" => Ok(Self::Chf),
            _ => Err(StorageError::Validation(
                "currency must be one of: EUR, USD, GBP, CHF",
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

pub struct CreateAccountInput<'a> {
    pub name: &'a str,
    pub account_type: AccountType,
    pub base_currency: Currency,
}

pub struct UpsertAccountBalanceInput<'a> {
    pub account_id: i64,
    pub currency: Currency,
    pub amount: &'a str,
}

pub struct UpsertFxRateInput<'a> {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub rate: &'a str,
}

#[derive(Debug, Eq, PartialEq)]
pub enum UpsertOutcome {
    Created,
    Updated,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountRecord {
    pub id: i64,
    pub name: String,
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
    pub id: i64,
    pub name: String,
    pub account_type: AccountType,
    pub base_currency: Currency,
    pub summary_status: AccountSummaryStatus,
    pub total_amount: Option<String>,
    pub total_currency: Option<Currency>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountBalanceRecord {
    pub account_id: i64,
    pub currency: Currency,
    pub amount: String,
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
    pub rate: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FxRateSummaryItemRecord {
    pub from_currency: Currency,
    pub rate: String,
    pub updated_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FxRateSummaryRecord {
    pub target_currency: Currency,
    pub rates: Vec<FxRateSummaryItemRecord>,
    pub last_updated: Option<String>,
}
