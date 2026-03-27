use async_graphql::{Enum, InputObject, SimpleObject};

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum AccountType {
    #[graphql(name = "BANK")]
    Bank,
    #[graphql(name = "BROKER")]
    Broker,
    #[graphql(name = "CRYPTO")]
    Crypto,
}

impl From<crate::AccountType> for AccountType {
    fn from(t: crate::AccountType) -> Self {
        match t {
            crate::AccountType::Bank => Self::Bank,
            crate::AccountType::Broker => Self::Broker,
            crate::AccountType::Crypto => Self::Crypto,
        }
    }
}

impl From<AccountType> for crate::AccountType {
    fn from(t: AccountType) -> Self {
        match t {
            AccountType::Bank => Self::Bank,
            AccountType::Broker => Self::Broker,
            AccountType::Crypto => Self::Crypto,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum AssetType {
    #[graphql(name = "STOCK")]
    Stock,
    #[graphql(name = "ETF")]
    Etf,
    #[graphql(name = "BOND")]
    Bond,
    #[graphql(name = "CRYPTO")]
    Crypto,
    #[graphql(name = "CASH_EQUIVALENT")]
    CashEquivalent,
    #[graphql(name = "OTHER")]
    Other,
}

impl From<crate::AssetType> for AssetType {
    fn from(t: crate::AssetType) -> Self {
        match t {
            crate::AssetType::Stock => Self::Stock,
            crate::AssetType::Etf => Self::Etf,
            crate::AssetType::Bond => Self::Bond,
            crate::AssetType::Crypto => Self::Crypto,
            crate::AssetType::CashEquivalent => Self::CashEquivalent,
            crate::AssetType::Other => Self::Other,
        }
    }
}

impl From<AssetType> for crate::AssetType {
    fn from(t: AssetType) -> Self {
        match t {
            AssetType::Stock => Self::Stock,
            AssetType::Etf => Self::Etf,
            AssetType::Bond => Self::Bond,
            AssetType::Crypto => Self::Crypto,
            AssetType::CashEquivalent => Self::CashEquivalent,
            AssetType::Other => Self::Other,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum TransactionType {
    #[graphql(name = "BUY")]
    Buy,
    #[graphql(name = "SELL")]
    Sell,
}

impl From<crate::AssetTransactionType> for TransactionType {
    fn from(t: crate::AssetTransactionType) -> Self {
        match t {
            crate::AssetTransactionType::Buy => Self::Buy,
            crate::AssetTransactionType::Sell => Self::Sell,
        }
    }
}

impl From<TransactionType> for crate::AssetTransactionType {
    fn from(t: TransactionType) -> Self {
        match t {
            TransactionType::Buy => Self::Buy,
            TransactionType::Sell => Self::Sell,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum SummaryStatus {
    #[graphql(name = "OK")]
    Ok,
    #[graphql(name = "CONVERSION_UNAVAILABLE")]
    ConversionUnavailable,
}

impl From<crate::AccountSummaryStatus> for SummaryStatus {
    fn from(s: crate::AccountSummaryStatus) -> Self {
        match s {
            crate::AccountSummaryStatus::Ok => Self::Ok,
            crate::AccountSummaryStatus::ConversionUnavailable => Self::ConversionUnavailable,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum RefreshAvailability {
    #[graphql(name = "AVAILABLE")]
    Available,
    #[graphql(name = "UNAVAILABLE")]
    Unavailable,
}

impl From<crate::FxRefreshAvailability> for RefreshAvailability {
    fn from(a: crate::FxRefreshAvailability) -> Self {
        match a {
            crate::FxRefreshAvailability::Available => Self::Available,
            crate::FxRefreshAvailability::Unavailable => Self::Unavailable,
        }
    }
}

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct AccountInput {
    pub name: String,
    pub account_type: AccountType,
    pub base_currency: String,
}

#[derive(InputObject)]
pub struct UpsertBalanceInput {
    pub currency: String,
    pub amount: String,
}

#[derive(InputObject)]
pub struct AssetInput {
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub quote_symbol: Option<String>,
    pub isin: Option<String>,
}

#[derive(InputObject)]
pub struct TransactionInput {
    pub account_id: i64,
    pub asset_id: i64,
    pub transaction_type: TransactionType,
    pub trade_date: String,
    pub quantity: String,
    pub unit_price: String,
    pub currency_code: String,
    pub notes: Option<String>,
}

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct AccountSummary {
    pub id: i64,
    pub name: String,
    pub account_type: AccountType,
    pub base_currency: String,
    pub summary_status: SummaryStatus,
    pub cash_total_amount: Option<String>,
    pub asset_total_amount: Option<String>,
    pub total_amount: Option<String>,
    pub total_currency: Option<String>,
}

#[derive(SimpleObject)]
pub struct AccountDetail {
    pub id: i64,
    pub name: String,
    pub account_type: AccountType,
    pub base_currency: String,
    pub summary_status: SummaryStatus,
    pub cash_total_amount: Option<String>,
    pub asset_total_amount: Option<String>,
    pub total_amount: Option<String>,
    pub total_currency: Option<String>,
    pub created_at: String,
    pub balances: Vec<Balance>,
}

#[derive(SimpleObject)]
pub struct Balance {
    pub currency: String,
    pub amount: String,
    pub updated_at: String,
}

#[derive(SimpleObject)]
pub struct AssetPosition {
    pub account_id: i64,
    pub asset_id: i64,
    pub quantity: String,
}

#[derive(SimpleObject)]
pub struct Asset {
    pub id: i64,
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub quote_symbol: Option<String>,
    pub isin: Option<String>,
    pub current_price: Option<String>,
    pub current_price_currency: Option<String>,
    pub current_price_as_of: Option<String>,
    pub total_quantity: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(SimpleObject)]
pub struct Transaction {
    pub id: i64,
    pub account_id: i64,
    pub asset_id: i64,
    pub transaction_type: TransactionType,
    pub trade_date: String,
    pub quantity: String,
    pub unit_price: String,
    pub currency_code: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(SimpleObject)]
pub struct FxRateSummary {
    pub target_currency: String,
    pub rates: Vec<FxRateSummaryItem>,
    pub last_updated: Option<String>,
    pub refresh_status: RefreshAvailability,
    pub refresh_error: Option<String>,
}

#[derive(SimpleObject)]
pub struct FxRateSummaryItem {
    pub currency: String,
    pub rate: String,
}

#[derive(SimpleObject)]
pub struct PortfolioSummary {
    pub display_currency: String,
    pub total_value_status: SummaryStatus,
    pub total_value_amount: Option<String>,
    pub account_totals: Vec<PortfolioAccountTotal>,
    pub cash_by_currency: Vec<PortfolioCashByCurrency>,
    pub fx_last_updated: Option<String>,
    pub fx_refresh_status: RefreshAvailability,
    pub fx_refresh_error: Option<String>,
    pub allocation_totals: Vec<PortfolioAllocationSlice>,
    pub allocation_is_partial: bool,
    pub holdings: Vec<PortfolioHolding>,
    pub holdings_is_partial: bool,
}

#[derive(SimpleObject)]
pub struct PortfolioAccountTotal {
    pub id: i64,
    pub name: String,
    pub account_type: AccountType,
    pub summary_status: SummaryStatus,
    pub cash_total_amount: Option<String>,
    pub asset_total_amount: Option<String>,
    pub total_amount: Option<String>,
    pub total_currency: String,
}

#[derive(SimpleObject)]
pub struct PortfolioCashByCurrency {
    pub currency: String,
    pub amount: String,
    pub converted_amount: Option<String>,
}

#[derive(SimpleObject)]
pub struct PortfolioAllocationSlice {
    pub label: String,
    pub amount: String,
}

#[derive(SimpleObject)]
pub struct PortfolioHolding {
    pub asset_id: i64,
    pub symbol: String,
    pub name: String,
    pub value: String,
}
