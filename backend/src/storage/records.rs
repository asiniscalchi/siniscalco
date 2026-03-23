use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;

use super::account_id::AccountId;
use super::account_name::AccountName;
use super::account_summary_status::AccountSummaryStatus;
use super::account_type::AccountType;
use super::amount::Amount;
use super::currency::Currency;
use super::fx_rate::FxRate;
use super::storage_error::StorageError;

pub const UTC_TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

pub fn current_utc_timestamp() -> Result<String, StorageError> {
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
pub struct FxRateDetailRecord {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub rate: FxRate,
    pub updated_at: String,
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

#[derive(Debug, Eq, PartialEq)]
pub struct PortfolioSummaryRecord {
    pub display_currency: Currency,
    pub total_value_status: AccountSummaryStatus,
    pub total_value_amount: Option<Amount>,
    pub account_totals: Vec<PortfolioAccountTotalRecord>,
    pub cash_by_currency: Vec<PortfolioCashByCurrencyRecord>,
    pub fx_last_updated: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PortfolioAccountTotalRecord {
    pub id: AccountId,
    pub name: AccountName,
    pub account_type: AccountType,
    pub summary_status: AccountSummaryStatus,
    pub total_amount: Option<Amount>,
    pub total_currency: Currency,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PortfolioCashByCurrencyRecord {
    pub currency: Currency,
    pub amount: Amount,
}
