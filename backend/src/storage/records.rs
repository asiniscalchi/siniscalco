use time::OffsetDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;

use super::account_id::AccountId;
use super::account_name::AccountName;
use super::account_summary_status::AccountSummaryStatus;
use super::account_type::AccountType;
use super::amount::Amount;
use super::asset_id::AssetId;
use super::asset_name::AssetName;
use super::asset_position::AssetPosition;
use super::asset_quantity::AssetQuantity;
use super::asset_symbol::AssetSymbol;
use super::asset_transaction_type::AssetTransactionType;
use super::asset_type::AssetType;
use super::asset_unit_price::AssetUnitPrice;
use super::currency::Currency;
use super::fx_rate::FxRate;
use super::storage_error::StorageError;
use super::trade_date::TradeDate;
use super::transfer_id::TransferId;

pub const UTC_ISO8601_TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");

pub fn current_utc_timestamp() -> Result<String, StorageError> {
    OffsetDateTime::now_utc()
        .format(UTC_ISO8601_TIMESTAMP_FORMAT)
        .map_err(|_| StorageError::Validation("failed to generate UTC timestamp"))
}

pub struct CreateAccountInput {
    pub name: AccountName,
    pub account_type: AccountType,
    pub base_currency: Currency,
}

pub struct CreateAssetInput {
    pub symbol: AssetSymbol,
    pub name: AssetName,
    pub asset_type: AssetType,
    pub quote_symbol: Option<String>,
    pub isin: Option<String>,
}

pub struct CreateAssetTransactionInput {
    pub account_id: AccountId,
    pub asset_id: AssetId,
    pub transaction_type: AssetTransactionType,
    pub trade_date: TradeDate,
    pub quantity: AssetQuantity,
    pub unit_price: AssetUnitPrice,
    pub currency_code: Currency,
    pub notes: Option<String>,
}

pub struct CreateCashMovementInput {
    pub account_id: AccountId,
    pub currency: Currency,
    pub amount: Amount,
    pub date: TradeDate,
    pub notes: Option<String>,
}

pub struct CreateTodoInput {
    pub title: String,
    pub due_date: TradeDate,
    pub symbol: Option<String>,
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
    pub cash_total_amount: Option<Amount>,
    pub asset_total_amount: Option<Amount>,
    pub total_amount: Option<Amount>,
    pub total_currency: Option<Currency>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AccountValueSummaryRecord {
    pub summary_status: AccountSummaryStatus,
    pub cash_total_amount: Option<Amount>,
    pub asset_total_amount: Option<Amount>,
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
pub struct CashMovementRecord {
    pub id: i64,
    pub account_id: AccountId,
    pub currency: Currency,
    pub amount: Amount,
    pub date: TradeDate,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TodoRecord {
    pub id: i64,
    pub title: String,
    pub due_date: TradeDate,
    pub symbol: Option<String>,
    pub completed: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AssetRecord {
    pub id: AssetId,
    pub symbol: AssetSymbol,
    pub name: AssetName,
    pub asset_type: AssetType,
    pub quote_symbol: Option<String>,
    pub isin: Option<String>,
    pub quote_source_symbol: Option<String>,
    pub quote_source_provider: Option<String>,
    pub quote_source_last_success_at: Option<String>,
    pub current_price: Option<AssetUnitPrice>,
    pub current_price_currency: Option<Currency>,
    pub current_price_as_of: Option<String>,
    pub total_quantity: Option<AssetQuantity>,
    pub avg_cost_basis: Option<AssetUnitPrice>,
    pub avg_cost_basis_currency: Option<Currency>,
    pub previous_close: Option<AssetUnitPrice>,
    pub previous_close_currency: Option<Currency>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct UpsertAssetPriceInput {
    pub asset_id: AssetId,
    pub price: AssetUnitPrice,
    pub currency: Currency,
    pub as_of: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct UpsertAssetQuoteSourceInput {
    pub asset_id: AssetId,
    pub quote_symbol: String,
    pub provider: String,
    pub last_success_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AssetTransactionRecord {
    pub id: i64,
    pub account_id: AccountId,
    pub asset_id: AssetId,
    pub transaction_type: AssetTransactionType,
    pub trade_date: TradeDate,
    pub quantity: AssetQuantity,
    pub unit_price: AssetUnitPrice,
    pub currency_code: Currency,
    pub fx_rate: FxRate,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AssetPositionRecord {
    pub account_id: AccountId,
    pub asset_id: AssetId,
    pub quantity: AssetPosition,
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
pub struct PortfolioAllocationSliceRecord {
    pub label: String,
    pub amount: Amount,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PortfolioSummaryRecord {
    pub display_currency: Currency,
    pub total_value_status: AccountSummaryStatus,
    pub total_value_amount: Option<Amount>,
    pub gain_24h_amount: Option<Amount>,
    pub total_gain_amount: Option<Amount>,
    pub account_totals: Vec<PortfolioAccountTotalRecord>,
    pub cash_by_currency: Vec<PortfolioCashByCurrencyRecord>,
    pub fx_last_updated: Option<String>,
    pub allocation_totals: Vec<PortfolioAllocationSliceRecord>,
    pub allocation_is_partial: bool,
    pub holdings: Vec<PortfolioHoldingRecord>,
    pub holdings_is_partial: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PortfolioAccountTotalRecord {
    pub id: AccountId,
    pub name: AccountName,
    pub account_type: AccountType,
    pub summary_status: AccountSummaryStatus,
    pub cash_total_amount: Option<Amount>,
    pub asset_total_amount: Option<Amount>,
    pub total_amount: Option<Amount>,
    pub total_currency: Currency,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PortfolioCashByCurrencyRecord {
    pub currency: Currency,
    pub amount: Amount,
    pub converted_amount: Option<Amount>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PortfolioHoldingRecord {
    pub asset_id: Option<AssetId>,
    pub symbol: String,
    pub name: String,
    pub value: Amount,
}

pub struct CreateTransferInput {
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    pub from_currency: Currency,
    pub from_amount: Amount,
    pub to_currency: Currency,
    pub to_amount: Amount,
    pub transfer_date: TradeDate,
    pub notes: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TransferRecord {
    pub id: TransferId,
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    pub from_currency: Currency,
    pub from_amount: Amount,
    pub to_currency: Currency,
    pub to_amount: Amount,
    pub transfer_date: TradeDate,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PortfolioSnapshotRecord {
    pub total_value: Amount,
    pub currency: Currency,
    pub recorded_at: String,
}
