mod asset_price_refresh;
pub mod assistant;
pub mod chat_threads;
mod config;
mod db;
mod format;
mod fx_refresh;
mod graphql;
mod logging;
pub mod mcp;
mod portfolio_snapshot_job;
pub mod storage;

// ── Configuration & startup ──────────────────────────────────────────────────

pub use config::Config;
pub use db::{connect_db, connect_db_file, init_db};
pub use logging::{default_log_filter, init_tracing};

// ── HTTP / GraphQL ───────────────────────────────────────────────────────────

pub use graphql::{
    AppState, AssistantState, build_full_router_with_state, build_router, build_router_with_state,
    schema_sdl,
};

// ── Formatting helpers ───────────────────────────────────────────────────────

pub use format::{
    compact_decimal_output, fmt_amount, fmt_opt_amount, format_decimal_amount,
    normalize_amount_output,
};

// ── FX refresh ───────────────────────────────────────────────────────────────

pub use fx_refresh::{
    FxRefreshAvailability, FxRefreshConfig, FxRefreshStatus, PRODUCT_BASE_CURRENCY,
    SharedFxRefreshStatus, fetch_frankfurter_rates, new_shared_fx_refresh_status, refresh_fx_rates,
    spawn_fx_refresh_task,
};

// ── Asset price refresh ──────────────────────────────────────────────────────

pub use asset_price_refresh::{
    AssetPriceRefreshConfig, fetch_alpha_vantage_quote, fetch_coingecko_quote, fetch_eodhd_quote,
    fetch_finnhub_quote, fetch_fmp_quote, fetch_marketstack_quote, fetch_openfigi_tickers,
    fetch_polygon_quote, fetch_tiingo_quote, fetch_twelve_data_quote, refresh_asset_prices,
    refresh_single_asset_price, spawn_asset_price_refresh_task,
};

// ── Portfolio snapshots ──────────────────────────────────────────────────────

pub use portfolio_snapshot_job::spawn_portfolio_snapshot_task;

// ── MCP ──────────────────────────────────────────────────────────────────────

pub use mcp::SharedMcpClient;

// ── Storage: domain types ────────────────────────────────────────────────────

pub use storage::{
    AccountId, AccountName, AccountSummaryStatus, AccountType, Amount, AssetId, AssetName,
    AssetPosition, AssetQuantity, AssetSymbol, AssetTransactionType, AssetType, AssetUnitPrice,
    Currency, FxRate, TradeDate, TransferId,
};

// ── Storage: records ─────────────────────────────────────────────────────────

pub use storage::{
    AccountBalanceRecord, AccountRecord, AccountSummaryRecord, AccountValueSummaryRecord,
    AssetPositionRecord, AssetRecord, AssetTransactionRecord, CashMovementRecord, CurrencyRecord,
    FxRateDetailRecord, FxRateRecord, FxRateSummaryItemRecord, FxRateSummaryRecord,
    PortfolioAccountTotalRecord, PortfolioAllocationSliceRecord, PortfolioCashByCurrencyRecord,
    PortfolioHoldingRecord, PortfolioSnapshotRecord, PortfolioSummaryRecord, TransferRecord,
};

// ── Storage: input types ─────────────────────────────────────────────────────

pub use storage::{
    CreateAccountInput, CreateAssetInput, CreateAssetTransactionInput, CreateCashMovementInput,
    CreateTransferInput, UpsertAssetPriceInput, UpsertAssetQuoteSourceInput, UpsertFxRateInput,
    UpsertOutcome,
};

// ── Storage: operations ──────────────────────────────────────────────────────

pub use storage::{
    compute_portfolio_value_at, convert_asset_total_value_in_currency, create_account,
    create_asset, create_asset_transaction, create_cash_movement, create_transfer,
    current_utc_timestamp, delete_account, delete_asset, delete_asset_transaction, delete_transfer,
    get_account, get_account_value_summary, get_asset, get_latest_fx_rate, get_portfolio_summary,
    get_transaction, insert_portfolio_snapshot_if_missing, list_account_balances,
    list_account_positions, list_account_summaries, list_accounts, list_all_cash_movements,
    list_asset_transactions, list_assets, list_cash_movements, list_currencies,
    list_fx_rate_summary, list_fx_rates, list_portfolio_snapshots, list_transactions,
    list_transfers, list_transfers_by_account, replace_fx_rates, update_account, update_asset,
    update_asset_transaction, upsert_asset_price, upsert_asset_quote_source, upsert_fx_rate,
};
