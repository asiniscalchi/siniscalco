mod api;
mod db;
mod format;
mod fx_refresh;
mod logging;
mod storage;

pub use api::{ApiError, ApiErrorResponse, AppState, build_router, build_router_with_state};
pub use db::{connect_db, connect_db_file, init_db};
pub use format::{compact_decimal_output, format_decimal_amount, normalize_amount_output};
pub use fx_refresh::{
    FxRefreshAvailability, FxRefreshConfig, FxRefreshStatus, PRODUCT_BASE_CURRENCY,
    SharedFxRefreshStatus, fetch_frankfurter_rates, new_shared_fx_refresh_status, refresh_fx_rates,
    spawn_fx_refresh_task,
};
pub use logging::{default_log_filter, init_tracing};
pub use storage::{
    AccountBalanceRecord, AccountId, AccountName, AccountRecord, AccountSummaryRecord,
    AccountSummaryStatus, AccountType, Amount, AssetId, AssetName, AssetPosition,
    AssetPositionRecord, AssetQuantity, AssetRecord, AssetSymbol, AssetTransactionRecord,
    AssetTransactionType, AssetType, AssetUnitPrice, CreateAccountInput, CreateAssetInput,
    CreateAssetTransactionInput, Currency, CurrencyRecord, FxRate, FxRateDetailRecord,
    FxRateRecord, FxRateSummaryItemRecord, FxRateSummaryRecord, PortfolioAccountTotalRecord,
    PortfolioCashByCurrencyRecord, PortfolioSummaryRecord, TradeDate, UpsertAccountBalanceInput,
    UpsertFxRateInput, UpsertOutcome, create_account, create_asset, create_asset_transaction,
    current_utc_timestamp, current_utc_timestamp_iso8601, delete_account, delete_account_balance,
    get_account, get_asset, get_latest_fx_rate, get_portfolio_summary, list_account_balances,
    list_account_positions, list_account_summaries, list_accounts, list_asset_transactions,
    list_assets, list_currencies, list_fx_rate_summary, list_fx_rates, replace_fx_rates,
    upsert_account_balance, upsert_fx_rate,
};
