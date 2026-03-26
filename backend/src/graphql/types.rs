use async_graphql::SimpleObject;

#[derive(SimpleObject)]
pub struct AccountSummary {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub base_currency: String,
    pub summary_status: String,
    pub cash_total_amount: Option<String>,
    pub asset_total_amount: Option<String>,
    pub total_amount: Option<String>,
    pub total_currency: Option<String>,
}

#[derive(SimpleObject)]
pub struct AccountDetail {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub base_currency: String,
    pub summary_status: String,
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
    pub asset_type: String,
    pub quote_symbol: Option<String>,
    pub isin: Option<String>,
    pub current_price: Option<String>,
    pub current_price_currency: Option<String>,
    pub current_price_as_of: Option<String>,
    pub total_quantity: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(SimpleObject)]
pub struct Transaction {
    pub id: i64,
    pub account_id: i64,
    pub asset_id: i64,
    pub transaction_type: String,
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
    pub refresh_status: String,
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
    pub total_value_status: String,
    pub total_value_amount: Option<String>,
    pub account_totals: Vec<PortfolioAccountTotal>,
    pub cash_by_currency: Vec<PortfolioCashByCurrency>,
    pub fx_last_updated: Option<String>,
    pub fx_refresh_status: String,
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
    pub account_type: String,
    pub summary_status: String,
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
