mod api;
mod db;
mod format;
mod fx_refresh;
mod storage;

pub use api::{ApiError, ApiErrorResponse, AppState, build_router, build_router_with_state};
pub use db::{connect_db, connect_db_file, init_db};
pub use format::{compact_decimal_output, format_decimal_amount, normalize_amount_output};
pub use fx_refresh::{
    FxRefreshAvailability, FxRefreshConfig, FxRefreshStatus, PRODUCT_BASE_CURRENCY,
    SharedFxRefreshStatus, fetch_frankfurter_rates, new_shared_fx_refresh_status, refresh_fx_rates,
    spawn_fx_refresh_task,
};
pub use storage::{
    AccountBalanceRecord, AccountId, AccountName, AccountRecord, AccountSummaryRecord,
    AccountSummaryStatus, AccountType, Amount, CreateAccountInput, Currency, CurrencyRecord,
    FxRate, FxRateDetailRecord, FxRateRecord, FxRateSummaryItemRecord, FxRateSummaryRecord,
    PortfolioAccountTotalRecord, PortfolioCashByCurrencyRecord, PortfolioSummaryRecord,
    UpsertAccountBalanceInput, UpsertFxRateInput, UpsertOutcome, create_account,
    current_utc_timestamp, delete_account, delete_account_balance, get_account, get_latest_fx_rate,
    get_portfolio_summary, list_account_balances, list_account_summaries, list_accounts,
    list_currencies, list_fx_rate_summary, list_fx_rates, replace_fx_rates, upsert_account_balance,
    upsert_fx_rate,
};
