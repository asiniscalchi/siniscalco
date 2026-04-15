use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tempfile::NamedTempFile;
use tokio::sync::Barrier;

use super::{
    AccountBalanceRecord, AccountId, AccountName, AccountRecord, AccountSummaryRecord,
    AccountSummaryStatus, AccountType, Amount, AssetId, AssetName, AssetPositionRecord,
    AssetQuantity, AssetRecord, AssetSymbol, AssetTransactionType, AssetType, AssetUnitPrice,
    CreateAccountInput, CreateAssetInput, CreateAssetTransactionInput, CreateCashMovementInput,
    CreateTransferInput, Currency, CurrencyRecord, FxRate, FxRateDetailRecord, FxRateRecord,
    FxRateSummaryItemRecord, FxRateSummaryRecord, PortfolioSnapshotRecord, StorageError, TradeDate,
    UpsertAssetPriceInput, UpsertFxRateInput, UpsertOutcome, create_account, create_asset,
    create_asset_transaction, create_cash_movement, delete_account, get_account,
    get_latest_fx_rate, insert_portfolio_snapshot_if_missing, list_account_balances,
    list_account_positions, list_account_summaries, list_accounts, list_asset_transactions,
    list_assets, list_currencies, list_fx_rate_summary, list_fx_rates, list_portfolio_snapshots,
    recalculate_snapshots_from_date, update_account, upsert_asset_price, upsert_fx_rate,
};
use super::{create_transfer, delete_transfer, list_transfers};
use crate::db::init_db;

fn amt(value: &str) -> Amount {
    Amount::try_from(value).expect("amount should parse")
}

fn fx_rate(value: &str) -> FxRate {
    FxRate::try_from(value).expect("rate should parse")
}

fn account_id(value: i64) -> AccountId {
    AccountId::try_from(value).expect("account id should parse")
}

fn account_name(value: &str) -> AccountName {
    AccountName::try_from(value).expect("account name should parse")
}

fn asset_id(value: i64) -> AssetId {
    AssetId::try_from(value).expect("asset id should parse")
}

fn asset_symbol(value: &str) -> AssetSymbol {
    AssetSymbol::try_from(value).expect("asset symbol should parse")
}

fn asset_name(value: &str) -> AssetName {
    AssetName::try_from(value).expect("asset name should parse")
}

fn asset_quantity(value: &str) -> AssetQuantity {
    AssetQuantity::try_from(value).expect("asset quantity should parse")
}

fn asset_unit_price(value: &str) -> AssetUnitPrice {
    AssetUnitPrice::try_from(value).expect("asset unit price should parse")
}

fn trade_date(value: &str) -> TradeDate {
    TradeDate::try_from(value).expect("trade date should parse")
}

async fn seed_balance(
    pool: &sqlx::SqlitePool,
    account_id: AccountId,
    currency: Currency,
    amount: Amount,
) {
    create_cash_movement(
        pool,
        CreateCashMovementInput {
            account_id,
            currency,
            amount,
            date: trade_date("2024-01-01"),
            notes: None,
        },
    )
    .await
    .expect("balance seed should succeed");
}

async fn test_pool() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .expect("in-memory sqlite connect options should parse")
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("in-memory sqlite pool should connect");

    init_db(&pool).await.expect("schema should initialize");
    pool
}

async fn file_backed_pool() -> sqlx::SqlitePool {
    let file = NamedTempFile::new().expect("temp db file should be created");
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", file.path().display()))
        .expect("sqlite file connect options should parse")
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("file sqlite pool should connect");

    init_db(&pool).await.expect("schema should initialize");
    std::mem::forget(file);
    pool
}

async fn legacy_v2_pool_with_fx_rate() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .expect("in-memory sqlite connect options should parse")
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("in-memory sqlite pool should connect");

    sqlx::raw_sql(include_str!("../../migrations/0001_schema.sql"))
        .execute(&pool)
        .await
        .expect("schema migration should apply");
    sqlx::raw_sql(include_str!(
        "../../migrations/0002_asset_quote_sources.sql"
    ))
    .execute(&pool)
    .await
    .expect("quote source migration should apply");

    sqlx::query(
        r#"
        CREATE TABLE _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            success BOOLEAN NOT NULL,
            checksum BLOB NOT NULL,
            execution_time BIGINT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("migration metadata table should be created");

    let migrator = sqlx::migrate::Migrator::new(Path::new("./migrations"))
        .await
        .expect("runtime migrator should load");
    for migration in migrator.iter().filter(|migration| migration.version <= 2) {
        sqlx::query(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
             VALUES (?, ?, TRUE, ?, 0)",
        )
        .bind(migration.version)
        .bind(migration.description.as_ref())
        .bind(migration.checksum.as_ref())
        .execute(&pool)
        .await
        .expect("migration metadata should insert");
    }

    sqlx::query(
        "INSERT INTO fx_rates (from_currency, to_currency, rate, updated_at) VALUES (?, ?, ?, ?)",
    )
    .bind(Currency::Usd.as_str())
    .bind(Currency::Eur.as_str())
    .bind(fx_rate("0.500000").as_scaled_i64())
    .bind("2025-01-01T00:00:00Z")
    .execute(&pool)
    .await
    .expect("legacy fx rate should insert");

    pool
}

#[tokio::test]
async fn creates_account_without_balance() {
    let pool = test_pool().await;

    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
        .fetch_one(&pool)
        .await
        .expect("account count query should succeed");

    assert_eq!(count, 1);
}

#[tokio::test]
async fn upsert_asset_price_records_history_and_exposes_previous_close() {
    let pool = test_pool().await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    // No history yet — previous_close should be None
    let assets = list_assets(&pool).await.expect("list should succeed");
    assert_eq!(assets[0].previous_close, None);

    // First price upsert: recorded_at is a past date, so it qualifies as previous_close
    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: asset_unit_price("90.000000"),
            currency: Currency::Usd,
            as_of: "2020-01-01T10:00:00Z".to_string(),
        },
    )
    .await
    .expect("first price upsert should succeed");

    let assets = list_assets(&pool).await.expect("list should succeed");
    assert_eq!(
        assets[0].previous_close,
        Some(asset_unit_price("90.000000"))
    );
    assert_eq!(assets[0].previous_close_currency, Some(Currency::Usd));

    // Second price upsert: more recent past date — previous_close should update to this one
    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: asset_unit_price("100.000000"),
            currency: Currency::Usd,
            as_of: "2020-01-02T10:00:00Z".to_string(),
        },
    )
    .await
    .expect("second price upsert should succeed");

    let history_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM asset_price_history WHERE asset_id = ?")
            .bind(asset_id.as_i64())
            .fetch_one(&pool)
            .await
            .expect("history count should succeed");
    assert_eq!(history_count, 2);

    let assets = list_assets(&pool).await.expect("list should succeed");
    assert_eq!(
        assets[0].previous_close,
        Some(asset_unit_price("100.000000"))
    );
}

#[tokio::test]
async fn lists_assets_in_symbol_order() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
            quote_symbol: None,
            isin: Some("US9229087690".to_string()),
        },
    )
    .await
    .expect("first asset insert should succeed");

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("second asset insert should succeed");

    let assets = list_assets(&pool).await.expect("asset list should succeed");

    assert_eq!(
        assets,
        vec![
            AssetRecord {
                id: asset_id(2),
                symbol: asset_symbol("AAPL"),
                name: asset_name("Apple Inc."),
                asset_type: AssetType::Stock,
                quote_symbol: None,
                isin: None,
                quote_source_symbol: None,
                quote_source_provider: None,
                quote_source_last_success_at: None,
                current_price: None,
                current_price_currency: None,
                current_price_as_of: None,
                total_quantity: None,
                avg_cost_basis: None,
                avg_cost_basis_currency: None,
                previous_close: None,
                previous_close_currency: None,
                created_at: assets[0].created_at.clone(),
                updated_at: assets[0].updated_at.clone(),
            },
            AssetRecord {
                id: asset_id(1),
                symbol: asset_symbol("VTI"),
                name: asset_name("Vanguard Total Stock Market ETF"),
                asset_type: AssetType::Etf,
                quote_symbol: None,
                isin: Some("US9229087690".to_string()),
                quote_source_symbol: None,
                quote_source_provider: None,
                quote_source_last_success_at: None,
                current_price: None,
                current_price_currency: None,
                current_price_as_of: None,
                total_quantity: None,
                avg_cost_basis: None,
                avg_cost_basis_currency: None,
                previous_close: None,
                previous_close_currency: None,
                created_at: assets[1].created_at.clone(),
                updated_at: assets[1].updated_at.clone(),
            },
        ]
    );
}

#[tokio::test]
async fn rejects_duplicate_asset_symbols() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: Some("US0378331005".to_string()),
        },
    )
    .await
    .expect("first asset insert should succeed");

    let error = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Common Stock"),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: Some("US0378331006".to_string()),
        },
    )
    .await
    .expect_err("duplicate asset symbol should be rejected");

    assert!(error.to_string().contains("UNIQUE constraint failed"));
}

#[tokio::test]
async fn rejects_duplicate_asset_isins() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
            quote_symbol: None,
            isin: Some("US9229087690".to_string()),
        },
    )
    .await
    .expect("first asset insert should succeed");

    let error = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VWCE"),
            name: asset_name("Vanguard FTSE All-World UCITS ETF"),
            asset_type: AssetType::Etf,
            quote_symbol: None,
            isin: Some("US9229087690".to_string()),
        },
    )
    .await
    .expect_err("duplicate asset isin should be rejected");

    assert!(error.to_string().contains("UNIQUE constraint failed"));
}

#[tokio::test]
async fn creates_asset_transactions_and_derives_positions() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    seed_balance(&pool, account_id, Currency::Usd, amt("2000.000000")).await;

    let aapl_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id: aapl_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("10"),
            unit_price: asset_unit_price("150.25"),
            currency_code: Currency::Usd,
            notes: Some("initial buy".to_string()),
        },
    )
    .await
    .expect("buy insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id: aapl_id,
            transaction_type: AssetTransactionType::Sell,
            trade_date: trade_date("2026-03-21"),
            quantity: asset_quantity("4"),
            unit_price: asset_unit_price("160"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("sell insert should succeed");

    let transactions = list_asset_transactions(&pool, account_id)
        .await
        .expect("transaction list should succeed");
    let positions = list_account_positions(&pool, account_id)
        .await
        .expect("position list should succeed");

    assert_eq!(transactions.len(), 2);
    assert_eq!(transactions[0].transaction_type, AssetTransactionType::Sell);
    assert_eq!(transactions[1].transaction_type, AssetTransactionType::Buy);
    assert_eq!(
        positions,
        vec![AssetPositionRecord {
            account_id,
            asset_id: aapl_id,
            quantity: super::AssetPosition::try_from(rust_decimal::Decimal::new(6, 0)).unwrap(),
        }]
    );
}

#[tokio::test]
async fn derives_positions_for_maximum_supported_transaction_quantities() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    seed_balance(
        &pool,
        account_id,
        Currency::Usd,
        amt("9223372036854.775807"),
    )
    .await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("BIG"),
            name: asset_name("Big Asset"),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-22"),
            quantity: asset_quantity("9223372036854.775807"),
            unit_price: asset_unit_price("1"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("buy insert should succeed");

    let positions = list_account_positions(&pool, account_id)
        .await
        .expect("position list should succeed");

    assert_eq!(
        positions,
        vec![AssetPositionRecord {
            account_id,
            asset_id,
            quantity: super::AssetPosition::try_from(
                rust_decimal::Decimal::from_str("9223372036854.775807").unwrap(),
            )
            .unwrap(),
        }]
    );
}

#[tokio::test]
async fn rejects_oversell_and_keeps_transactions_unchanged() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    seed_balance(&pool, account_id, Currency::Usd, amt("200000.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("BTC"),
            name: asset_name("Bitcoin"),
            asset_type: AssetType::Crypto,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("2"),
            unit_price: asset_unit_price("80000"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("buy insert should succeed");

    let error = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Sell,
            trade_date: trade_date("2026-03-21"),
            quantity: asset_quantity("3"),
            unit_price: asset_unit_price("81000"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect_err("oversell should fail");

    assert_eq!(
        error.to_string(),
        "sell transaction would make position negative"
    );

    let transactions = list_asset_transactions(&pool, account_id)
        .await
        .expect("transaction list should succeed");
    assert_eq!(transactions.len(), 1);
}

#[tokio::test]
async fn updating_asset_transaction_rejects_oversell_and_keeps_original_transaction() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    seed_balance(&pool, account_id, Currency::Usd, amt("500000.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("BTC"),
            name: asset_name("Bitcoin"),
            asset_type: AssetType::Crypto,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("5"),
            unit_price: asset_unit_price("80000"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("buy insert should succeed");

    let sell_transaction = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Sell,
            trade_date: trade_date("2026-03-21"),
            quantity: asset_quantity("2"),
            unit_price: asset_unit_price("81000"),
            currency_code: Currency::Usd,
            notes: Some("original sell".to_string()),
        },
    )
    .await
    .expect("sell insert should succeed");

    let error = super::update_asset_transaction(
        &pool,
        sell_transaction.id,
        super::CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Sell,
            trade_date: trade_date("2026-03-21"),
            quantity: asset_quantity("6"),
            unit_price: asset_unit_price("81000"),
            currency_code: Currency::Usd,
            notes: Some("updated sell".to_string()),
        },
    )
    .await
    .expect_err("oversell update should fail");

    assert_eq!(
        error.to_string(),
        "sell transaction would make position negative"
    );

    let transactions = list_asset_transactions(&pool, account_id)
        .await
        .expect("transaction list should succeed");

    assert_eq!(transactions.len(), 2);
    assert_eq!(transactions[0].id, sell_transaction.id);
    assert_eq!(transactions[0].quantity.to_string(), "2");
    assert_eq!(transactions[0].notes.as_deref(), Some("original sell"));
}

#[tokio::test]
async fn omits_zero_positions_and_isolates_accounts() {
    let pool = test_pool().await;

    let account_a = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("A"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("first account insert should succeed");
    let account_b = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("B"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("second account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    for (account_id, balance) in [(account_a, "300.000000"), (account_b, "700.000000")] {
        seed_balance(&pool, account_id, Currency::Usd, amt(balance)).await;
    }

    for input in [
        (account_a, AssetTransactionType::Buy, "3"),
        (account_a, AssetTransactionType::Sell, "3"),
        (account_b, AssetTransactionType::Buy, "7"),
    ] {
        create_asset_transaction(
            &pool,
            CreateAssetTransactionInput {
                account_id: input.0,
                asset_id,
                transaction_type: input.1,
                trade_date: trade_date("2026-03-20"),
                quantity: asset_quantity(input.2),
                unit_price: asset_unit_price("100"),
                currency_code: Currency::Usd,
                notes: None,
            },
        )
        .await
        .expect("transaction insert should succeed");
    }

    let account_a_positions = list_account_positions(&pool, account_a)
        .await
        .expect("first account positions should succeed");
    let account_b_positions = list_account_positions(&pool, account_b)
        .await
        .expect("second account positions should succeed");

    assert!(account_a_positions.is_empty());
    assert_eq!(account_b_positions.len(), 1);
    assert_eq!(account_b_positions[0].account_id, account_b);
    assert_eq!(account_b_positions[0].asset_id, asset_id);
    assert_eq!(account_b_positions[0].quantity.to_string(), "7");
}

#[tokio::test]
async fn prevents_concurrent_oversell() {
    let pool = file_backed_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    seed_balance(&pool, account_id, Currency::Usd, amt("1500.000000")).await;

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("10"),
            unit_price: asset_unit_price("150"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("seed buy should succeed");

    let barrier = Arc::new(Barrier::new(3));
    let sell_task = |pool: sqlx::SqlitePool, barrier: Arc<Barrier>| async move {
        barrier.wait().await;
        create_asset_transaction(
            &pool,
            CreateAssetTransactionInput {
                account_id,
                asset_id,
                transaction_type: AssetTransactionType::Sell,
                trade_date: trade_date("2026-03-21"),
                quantity: asset_quantity("7"),
                unit_price: asset_unit_price("155"),
                currency_code: Currency::Usd,
                notes: None,
            },
        )
        .await
    };

    let task_a = tokio::spawn(sell_task(pool.clone(), barrier.clone()));
    let task_b = tokio::spawn(sell_task(pool.clone(), barrier.clone()));
    barrier.wait().await;

    let result_a = task_a.await.expect("first sell task should complete");
    let result_b = task_b.await.expect("second sell task should complete");
    let successes = [result_a.is_ok(), result_b.is_ok()]
        .into_iter()
        .filter(|success| *success)
        .count();

    assert_eq!(successes, 1);

    let positions = list_account_positions(&pool, account_id)
        .await
        .expect("position list should succeed");
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].quantity.to_string(), "3");
}

#[tokio::test]
async fn buy_transaction_deducts_base_currency_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Usd, amt("1000.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    // BUY 5 × 100 = 500 USD; balance should drop from 1000 to 500
    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("5"),
            unit_price: asset_unit_price("100"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap();

    let balances = list_account_balances(&pool, account_id).await.unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].currency, Currency::Usd);
    assert_eq!(balances[0].amount, amt("500.000000"));
}

#[tokio::test]
async fn buy_transaction_with_fx_deducts_base_currency_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    // USD→EUR rate: 0.5
    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.500000"),
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Eur, amt("1000.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    // BUY 4 × 100 USD = 400 USD → 200 EUR deducted; balance: 1000 - 200 = 800 EUR
    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("4"),
            unit_price: asset_unit_price("100"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap();

    let balances = list_account_balances(&pool, account_id).await.unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].currency, Currency::Eur);
    assert_eq!(balances[0].amount, amt("800.000000"));
}

#[tokio::test]
async fn buy_transaction_allows_negative_cash_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Usd, amt("499.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    // Buy for 500 USD when only 499 is available: succeeds and drives balance negative
    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("5"),
            unit_price: asset_unit_price("100"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("buy with insufficient cash should succeed (overdraft allowed)");

    let balances = list_account_balances(&pool, account_id).await.unwrap();
    assert_eq!(balances[0].amount, amt("-1.000000"));
    let positions = list_account_positions(&pool, account_id).await.unwrap();
    assert_eq!(positions.len(), 1);
}

#[tokio::test]
async fn sell_transaction_credits_base_currency_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Usd, amt("1000.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    // BUY 10 × 100 = 1000 USD; balance: 1000 - 1000 = 0
    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("10"),
            unit_price: asset_unit_price("100"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap();

    // SELL 4 × 120 = 480 USD; balance: 0 + 480 = 480
    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Sell,
            trade_date: trade_date("2026-03-21"),
            quantity: asset_quantity("4"),
            unit_price: asset_unit_price("120"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap();

    let balances = list_account_balances(&pool, account_id).await.unwrap();
    assert_eq!(balances[0].currency, Currency::Usd);
    assert_eq!(balances[0].amount, amt("480.000000"));
}

#[tokio::test]
async fn deleting_buy_transaction_restores_cash_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Usd, amt("500.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    let tx = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("3"),
            unit_price: asset_unit_price("100"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap();

    // After BUY: 500 - 300 = 200
    let balances = list_account_balances(&pool, account_id).await.unwrap();
    assert_eq!(balances[0].amount, amt("200.000000"));

    super::delete_asset_transaction(&pool, tx.id).await.unwrap();

    // After DELETE: 200 + 300 = 500 (restored)
    let balances = list_account_balances(&pool, account_id).await.unwrap();
    assert_eq!(balances[0].amount, amt("500.000000"));
}

#[tokio::test]
async fn updating_transaction_adjusts_cash_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Usd, amt("1000.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    // BUY 5 × 100 = 500; balance: 1000 - 500 = 500
    let tx = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("5"),
            unit_price: asset_unit_price("100"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap();

    // UPDATE to BUY 3 × 100 = 300; reverses -500, applies -300; balance: 500 + 500 - 300 = 700
    super::update_asset_transaction(
        &pool,
        tx.id,
        super::CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("3"),
            unit_price: asset_unit_price("100"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap();

    let balances = list_account_balances(&pool, account_id).await.unwrap();
    assert_eq!(balances[0].amount, amt("700.000000"));
}

#[tokio::test]
async fn moving_cross_currency_transaction_to_account_with_different_base_currency_reprices_cash_impact()
 {
    let pool = test_pool().await;

    let usd_account = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("USD Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("usd account insert should succeed");

    let eur_account = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("EUR Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("eur account insert should succeed");

    seed_balance(&pool, usd_account, Currency::Usd, amt("1000.000000")).await;
    seed_balance(&pool, eur_account, Currency::Eur, amt("1000.000000")).await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.900000"),
        },
    )
    .await
    .expect("usd to eur rate should upsert");

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    let transaction = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id: usd_account,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("10"),
            unit_price: asset_unit_price("10"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    super::update_asset_transaction(
        &pool,
        transaction.id,
        CreateAssetTransactionInput {
            account_id: eur_account,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("10"),
            unit_price: asset_unit_price("10"),
            currency_code: Currency::Usd,
            notes: Some("moved to eur account".to_string()),
        },
    )
    .await
    .expect("moving the transaction should succeed");

    let usd_balances = list_account_balances(&pool, usd_account).await.unwrap();
    assert_eq!(usd_balances[0].currency, Currency::Usd);
    assert_eq!(usd_balances[0].amount, amt("1000.000000"));

    let eur_balances = list_account_balances(&pool, eur_account).await.unwrap();
    assert_eq!(eur_balances[0].currency, Currency::Eur);
    assert_eq!(eur_balances[0].amount, amt("910.000000"));
}

#[tokio::test]
async fn buy_transaction_blocked_without_fx_rate() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Eur, amt("1000.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    // No USD→EUR rate seeded — transaction must be blocked
    let error = create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: asset_quantity("1"),
            unit_price: asset_unit_price("100"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect_err("buy without fx rate should fail");

    assert_eq!(
        error.to_string(),
        "fx rate not available for transaction currency conversion"
    );
}

#[tokio::test]
async fn transfer_debits_source_and_credits_destination() {
    let pool = test_pool().await;

    let account_a = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Account A"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    let account_b = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Account B"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_a, Currency::Eur, amt("1000.000000")).await;

    seed_balance(&pool, account_b, Currency::Eur, amt("200.000000")).await;

    create_transfer(
        &pool,
        CreateTransferInput {
            from_account_id: account_a,
            to_account_id: account_b,
            from_currency: Currency::Eur,
            from_amount: amt("300.000000"),
            to_currency: Currency::Eur,
            to_amount: amt("300.000000"),
            transfer_date: trade_date("2026-03-29"),
            notes: None,
        },
    )
    .await
    .unwrap();

    let balances_a = list_account_balances(&pool, account_a).await.unwrap();
    assert_eq!(balances_a[0].amount, amt("700.000000"));

    let balances_b = list_account_balances(&pool, account_b).await.unwrap();
    assert_eq!(balances_b[0].amount, amt("500.000000"));
}

#[tokio::test]
async fn transfer_cross_currency_uses_specified_amounts() {
    let pool = test_pool().await;

    let account_eur = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("EUR Account"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    let account_usd = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("USD Account"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_eur, Currency::Eur, amt("500.000000")).await;

    seed_balance(&pool, account_usd, Currency::Usd, amt("0.000000")).await;

    // Send 200 EUR, receive 220 USD (user-specified rate)
    create_transfer(
        &pool,
        CreateTransferInput {
            from_account_id: account_eur,
            to_account_id: account_usd,
            from_currency: Currency::Eur,
            from_amount: amt("200.000000"),
            to_currency: Currency::Usd,
            to_amount: amt("220.000000"),
            transfer_date: trade_date("2026-03-29"),
            notes: None,
        },
    )
    .await
    .unwrap();

    let balances_eur = list_account_balances(&pool, account_eur).await.unwrap();
    assert_eq!(balances_eur[0].amount, amt("300.000000"));

    let balances_usd = list_account_balances(&pool, account_usd).await.unwrap();
    assert_eq!(balances_usd[0].amount, amt("220.000000"));
}

#[tokio::test]
async fn transfer_allows_source_overdraft() {
    let pool = test_pool().await;

    let account_a = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Account A"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    let account_b = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Account B"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_a, Currency::Eur, amt("100.000000")).await;

    // Transfer 200 when source only has 100: succeeds and drives source negative
    create_transfer(
        &pool,
        CreateTransferInput {
            from_account_id: account_a,
            to_account_id: account_b,
            from_currency: Currency::Eur,
            from_amount: amt("200.000000"),
            to_currency: Currency::Eur,
            to_amount: amt("200.000000"),
            transfer_date: trade_date("2026-03-29"),
            notes: None,
        },
    )
    .await
    .expect("transfer with insufficient balance should succeed (overdraft allowed)");

    let balances_a = list_account_balances(&pool, account_a).await.unwrap();
    assert_eq!(balances_a[0].amount, amt("-100.000000"));
    let balances_b = list_account_balances(&pool, account_b).await.unwrap();
    assert_eq!(balances_b[0].amount, amt("200.000000"));
}

#[tokio::test]
async fn deleting_transfer_reverses_balances() {
    let pool = test_pool().await;

    let account_a = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Account A"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    let account_b = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Account B"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_a, Currency::Eur, amt("1000.000000")).await;

    let transfer = create_transfer(
        &pool,
        CreateTransferInput {
            from_account_id: account_a,
            to_account_id: account_b,
            from_currency: Currency::Eur,
            from_amount: amt("400.000000"),
            to_currency: Currency::Eur,
            to_amount: amt("400.000000"),
            transfer_date: trade_date("2026-03-29"),
            notes: None,
        },
    )
    .await
    .unwrap();

    delete_transfer(&pool, transfer.id).await.unwrap();

    let balances_a = list_account_balances(&pool, account_a).await.unwrap();
    assert_eq!(balances_a[0].amount, amt("1000.000000"));

    let balances_b = list_account_balances(&pool, account_b).await.unwrap();
    assert_eq!(balances_b[0].amount, amt("0.000000"));
}

#[tokio::test]
async fn list_transfers_returns_in_date_descending_order() {
    let pool = test_pool().await;

    let account_a = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Account A"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    let account_b = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Account B"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_a, Currency::Eur, amt("2000.000000")).await;

    for (date, amount) in [
        ("2026-01-10", "100.000000"),
        ("2026-03-20", "200.000000"),
        ("2026-02-15", "50.000000"),
    ] {
        create_transfer(
            &pool,
            CreateTransferInput {
                from_account_id: account_a,
                to_account_id: account_b,
                from_currency: Currency::Eur,
                from_amount: amt(amount),
                to_currency: Currency::Eur,
                to_amount: amt(amount),
                transfer_date: trade_date(date),
                notes: None,
            },
        )
        .await
        .unwrap();
    }

    let transfers = list_transfers(&pool).await.unwrap();
    assert_eq!(transfers.len(), 3);
    assert_eq!(transfers[0].transfer_date.as_str(), "2026-03-20");
    assert_eq!(transfers[1].transfer_date.as_str(), "2026-02-15");
    assert_eq!(transfers[2].transfer_date.as_str(), "2026-01-10");
}

#[tokio::test]
async fn lists_currencies_in_code_order() {
    let pool = test_pool().await;

    let currencies = list_currencies(&pool)
        .await
        .expect("currency list should succeed");

    assert_eq!(
        currencies,
        vec![
            CurrencyRecord {
                code: Currency::Chf,
            },
            CurrencyRecord {
                code: Currency::Eur,
            },
            CurrencyRecord {
                code: Currency::Gbp,
            },
            CurrencyRecord {
                code: Currency::Usd,
            },
        ]
    );
}

#[tokio::test]
async fn reads_accounts_in_insert_order() {
    let pool = test_pool().await;

    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("first account insert should succeed");

    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("second account insert should succeed");

    let accounts = list_accounts(&pool)
        .await
        .expect("account list should succeed");

    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].name, account_name("Main Bank"));
    assert_eq!(accounts[0].account_type, AccountType::Bank);
    assert_eq!(accounts[1].name, account_name("IBKR"));
    assert_eq!(accounts[1].account_type, AccountType::Broker);
}

#[tokio::test]
async fn gets_single_account_by_id() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    let account = get_account(&pool, account_id)
        .await
        .expect("single account fetch should succeed");

    assert_eq!(account.id, account_id);
    assert_eq!(account.name, account_name("IBKR"));
    assert_eq!(account.account_type, AccountType::Broker);
    assert_eq!(account.base_currency, Currency::Eur);
}

#[tokio::test]
async fn updating_account_name_with_same_base_currency_succeeds() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    let updated = update_account(
        &pool,
        account_id,
        CreateAccountInput {
            name: account_name("Updated IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("updating non-currency account fields should succeed");

    assert_eq!(updated.id, account_id);
    assert_eq!(updated.name, account_name("Updated IBKR"));
    assert_eq!(updated.base_currency, Currency::Usd);
}

#[tokio::test]
async fn allows_multiple_currencies_per_account() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    for (currency, value) in [("EUR", "12000.000000"), ("USD", "3500.000000")] {
        seed_balance(
            &pool,
            account_id,
            Currency::try_from(currency).unwrap(),
            amt(value),
        )
        .await;
    }

    let balances = list_account_balances(&pool, account_id)
        .await
        .expect("balance list should succeed");

    assert_eq!(
        balances,
        vec![
            AccountBalanceRecord {
                account_id,
                currency: Currency::Eur,
                amount: amt("12000"),
                updated_at: balances[0].updated_at.clone(),
            },
            AccountBalanceRecord {
                account_id,
                currency: Currency::Usd,
                amount: amt("3500"),
                updated_at: balances[1].updated_at.clone(),
            }
        ]
    );
    assert_eq!(balances[0].updated_at.len(), 20);
    assert_eq!(balances[1].updated_at.len(), 20);
}

#[tokio::test]
async fn cash_movements_accumulate_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    // Two separate deposits: balance is their sum, not the last value
    seed_balance(&pool, account_id, Currency::Usd, amt("10.000000")).await;
    seed_balance(&pool, account_id, Currency::Usd, amt("12.000000")).await;

    let balances = list_account_balances(&pool, account_id)
        .await
        .expect("balance list should succeed");

    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].amount, amt("22.000000"));
    assert_eq!(balances[0].updated_at.len(), 20);
}

#[tokio::test]
async fn negative_cash_movement_reduces_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    seed_balance(&pool, account_id, Currency::Eur, amt("12000.000000")).await;

    create_cash_movement(
        &pool,
        CreateCashMovementInput {
            account_id,
            currency: Currency::Eur,
            amount: amt("-12000.000000"),
            date: trade_date("2024-01-02"),
            notes: None,
        },
    )
    .await
    .expect("withdrawal should succeed");

    let balances = list_account_balances(&pool, account_id)
        .await
        .expect("balance list should succeed");

    // Balance is zero; the account appears in the list with a zero sum
    assert_eq!(balances[0].amount, amt("0.000000"));
}

#[tokio::test]
async fn deletes_empty_account() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    delete_account(&pool, account_id)
        .await
        .expect("empty account delete should succeed");

    let account_error = get_account(&pool, account_id)
        .await
        .expect_err("deleted account should not exist");

    match account_error {
        StorageError::Database(sqlx::Error::RowNotFound) => {}
        other => panic!("expected RowNotFound, got {other}"),
    }
}

#[tokio::test]
async fn cannot_delete_account_with_ledger_entries() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    seed_balance(&pool, account_id, Currency::Eur, amt("12000.000000")).await;

    let error = delete_account(&pool, account_id)
        .await
        .expect_err("account with entries should not be deletable");

    match error {
        StorageError::Validation(_) => {}
        other => panic!("expected Validation error, got {other}"),
    }
}

#[tokio::test]
async fn deleting_missing_account_returns_not_found() {
    let pool = test_pool().await;

    let error = delete_account(&pool, account_id(999))
        .await
        .expect_err("missing account delete should fail");

    match error {
        StorageError::Database(sqlx::Error::RowNotFound) => {}
        other => panic!("expected RowNotFound, got {other}"),
    }
}

#[tokio::test]
async fn preserves_created_account_fields() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Joint Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Gbp,
        },
    )
    .await
    .expect("account insert should succeed");

    let accounts = list_accounts(&pool)
        .await
        .expect("account list should succeed");

    assert_eq!(
        accounts,
        vec![AccountRecord {
            id: account_id,
            name: account_name("Joint Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Gbp,
            created_at: accounts[0].created_at.clone(),
        }]
    );
    assert_eq!(accounts[0].created_at.len(), 20);
}

#[tokio::test]
async fn rejects_invalid_account_type_input() {
    let error = AccountType::try_from("cash").expect_err("unsupported account type should fail");

    assert_eq!(
        error.to_string(),
        "account_type must be one of: bank, broker, crypto"
    );
}

#[tokio::test]
async fn creates_crypto_account() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Kraken"),
            account_type: AccountType::Crypto,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("crypto account insert should succeed");

    let account = get_account(&pool, account_id)
        .await
        .expect("crypto account fetch should succeed");

    assert_eq!(account.account_type, AccountType::Crypto);
}

#[tokio::test]
async fn accepts_typed_account_currency_input() {
    let pool = test_pool().await;

    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("typed currency should succeed");
}

#[tokio::test]
async fn accepts_typed_balance_currency_input() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    seed_balance(&pool, account_id, Currency::Usd, amt("10.000000")).await;

    let balances = list_account_balances(&pool, account_id)
        .await
        .expect("balance list should succeed");
    assert_eq!(balances[0].amount, amt("10.000000"));
}

#[test]
fn rejects_invalid_typed_amount_input() {
    let error = Amount::try_from("1.1234567").expect_err("invalid amount should fail");

    assert_eq!(
        error.to_string(),
        "amount must match a signed 6-decimal value"
    );
}

#[tokio::test]
async fn rejects_balance_for_missing_account() {
    let pool = test_pool().await;

    let error = create_cash_movement(
        &pool,
        CreateCashMovementInput {
            account_id: account_id(999),
            currency: Currency::Usd,
            amount: amt("10.000000"),
            date: trade_date("2024-01-01"),
            notes: None,
        },
    )
    .await
    .expect_err("missing parent account should fail");

    match error {
        StorageError::Database(sqlx::Error::RowNotFound) => {}
        StorageError::Database(error) => {
            assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
        }
        StorageError::Validation(_) | StorageError::Internal(_) => {
            panic!("expected database error")
        }
    }
}

#[tokio::test]
async fn upserts_fx_rates() {
    let pool = test_pool().await;

    let outcome = upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.920000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    assert_eq!(outcome, UpsertOutcome::Created);
    assert_eq!(
        list_fx_rates(&pool).await.expect("fx rates should list"),
        vec![FxRateRecord {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.92"),
        }]
    );
}

#[tokio::test]
async fn gets_latest_fx_rate_with_timestamp() {
    let pool = test_pool().await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.920000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    let rate = get_latest_fx_rate(&pool, Currency::Usd, Currency::Eur)
        .await
        .expect("fx lookup should succeed")
        .expect("fx rate should exist");

    assert_eq!(
        rate,
        FxRateDetailRecord {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.92"),
            updated_at: rate.updated_at.clone(),
        }
    );
    assert_eq!(rate.updated_at.len(), 20);
}

#[tokio::test]
async fn updates_existing_fx_rate() {
    let pool = test_pool().await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.920000"),
        },
    )
    .await
    .expect("initial fx rate insert should succeed");

    let outcome = upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.910000"),
        },
    )
    .await
    .expect("fx rate update should succeed");

    assert_eq!(outcome, UpsertOutcome::Updated);
    assert_eq!(
        list_fx_rates(&pool).await.expect("fx rates should list"),
        vec![FxRateRecord {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.91"),
        }]
    );
}

#[test]
fn rejects_non_positive_fx_rates() {
    let error = FxRate::try_from("0.000000").expect_err("zero fx rate should fail");

    assert_eq!(error.to_string(), "rate must be greater than zero");
}

#[tokio::test]
async fn rejects_identity_fx_pairs() {
    let pool = test_pool().await;

    let error = upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Eur,
            to_currency: Currency::Eur,
            rate: fx_rate("1.000000"),
        },
    )
    .await
    .expect_err("identity fx pair should fail");

    assert_eq!(
        error.to_string(),
        "fx pair must contain two different currencies"
    );
}

#[tokio::test]
async fn rejects_non_eur_target_fx_pairs() {
    let pool = test_pool().await;

    let error = upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Gbp,
            rate: fx_rate("0.780000"),
        },
    )
    .await
    .expect_err("non-eur target pair should fail");

    assert_eq!(
        error.to_string(),
        "fx pair must convert a supported non-EUR currency into EUR"
    );
}

#[tokio::test]
async fn lists_fx_rate_summary_for_a_single_target_currency() {
    let pool = test_pool().await;

    for (from_currency, rate) in [
        (Currency::Usd, "0.920000"),
        (Currency::Gbp, "1.170000"),
        (Currency::Chf, "1.040000"),
    ] {
        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency,
                to_currency: Currency::Eur,
                rate: fx_rate(rate),
            },
        )
        .await
        .expect("fx rate insert should succeed");
    }

    for (from_currency, updated_at) in [
        ("USD", "2026-03-22 09:00:00"),
        ("GBP", "2026-03-22 10:00:00"),
        ("CHF", "2026-03-22 08:30:00"),
    ] {
        sqlx::query(
            "UPDATE fx_rates SET updated_at = ? WHERE from_currency = ? AND to_currency = 'EUR'",
        )
        .bind(updated_at)
        .bind(from_currency)
        .execute(&pool)
        .await
        .expect("timestamp update should succeed");
    }

    assert_eq!(
        list_fx_rate_summary(&pool, Currency::Eur)
            .await
            .expect("fx summary should succeed"),
        FxRateSummaryRecord {
            target_currency: Currency::Eur,
            rates: vec![
                FxRateSummaryItemRecord {
                    from_currency: Currency::Chf,
                    rate: fx_rate("1.04"),
                    updated_at: "2026-03-22 08:30:00".to_string(),
                },
                FxRateSummaryItemRecord {
                    from_currency: Currency::Gbp,
                    rate: fx_rate("1.17"),
                    updated_at: "2026-03-22 10:00:00".to_string(),
                },
                FxRateSummaryItemRecord {
                    from_currency: Currency::Usd,
                    rate: fx_rate("0.92"),
                    updated_at: "2026-03-22 09:00:00".to_string(),
                },
            ],
            last_updated: Some("2026-03-22 10:00:00".to_string()),
        }
    );
}

#[tokio::test]
async fn returns_empty_fx_rate_summary_when_target_has_no_rates() {
    let pool = test_pool().await;

    assert_eq!(
        list_fx_rate_summary(&pool, Currency::Eur)
            .await
            .expect("fx summary should succeed"),
        FxRateSummaryRecord {
            target_currency: Currency::Eur,
            rates: vec![],
            last_updated: None,
        }
    );
}

#[tokio::test]
async fn lists_account_summaries_with_zero_total_for_empty_accounts() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    assert_eq!(
        list_account_summaries(&pool)
            .await
            .expect("account summaries should succeed"),
        vec![AccountSummaryRecord {
            id: account_id,
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
            summary_status: AccountSummaryStatus::Ok,
            cash_total_amount: Some(amt("0.000000")),
            asset_total_amount: Some(amt("0.000000")),
            total_amount: Some(amt("0.000000")),
            total_currency: Some(Currency::Eur),
        }]
    );
}

#[tokio::test]
async fn lists_account_summaries_with_single_base_currency_balance() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Main Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .expect("account insert should succeed");

    seed_balance(&pool, account_id, Currency::Usd, amt("123.450000")).await;

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(summaries[0].summary_status, AccountSummaryStatus::Ok);
    assert_eq!(summaries[0].cash_total_amount, Some(amt("123.450000")));
    assert_eq!(summaries[0].asset_total_amount, Some(amt("0.000000")));
    assert_eq!(summaries[0].total_amount, Some(amt("123.450000")));
    assert_eq!(summaries[0].total_currency, Some(Currency::Usd));
}

#[tokio::test]
async fn lists_account_summaries_with_direct_fx_conversion() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    for (currency, value) in [
        (Currency::Eur, "10.000000"),
        (Currency::Usd, "20.000000"),
        (Currency::Gbp, "30.000000"),
    ] {
        seed_balance(&pool, account_id, currency, amt(value)).await;
    }

    for (from_currency, rate) in [(Currency::Usd, "0.500000"), (Currency::Gbp, "1.200000")] {
        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency,
                to_currency: Currency::Eur,
                rate: fx_rate(rate),
            },
        )
        .await
        .expect("fx rate insert should succeed");
    }

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(summaries[0].summary_status, AccountSummaryStatus::Ok);
    assert_eq!(summaries[0].cash_total_amount, Some(amt("56.000000")));
    assert_eq!(summaries[0].asset_total_amount, Some(amt("0.000000")));
    assert_eq!(summaries[0].total_amount, Some(amt("56.000000")));
    assert_eq!(summaries[0].total_currency, Some(Currency::Eur));
}

#[tokio::test]
async fn lists_account_summaries_with_separate_cash_and_asset_totals() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    // USD balance: kept as manual cash, not affected by the BUY below
    seed_balance(&pool, account_id, Currency::Usd, amt("20.000000")).await;

    // FX rate must be set up before the transaction so the cash impact can be computed
    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.500000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    // EUR balance: covers the cost of the BUY (2 × 80 USD × 0.5 = 80 EUR); after BUY it becomes 0
    seed_balance(&pool, account_id, Currency::Eur, amt("80.000000")).await;

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("BTC"),
            name: asset_name("Bitcoin"),
            asset_type: AssetType::Crypto,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-03-20"),
            quantity: AssetQuantity::try_from("2").unwrap(),
            unit_price: AssetUnitPrice::try_from("80").unwrap(),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: AssetUnitPrice::try_from("100").unwrap(),
            currency: Currency::Usd,
            as_of: "2026-03-22T10:00:00Z".to_string(),
        },
    )
    .await
    .expect("asset price insert should succeed");

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    // EUR balance is 0 after the BUY deduction; only the USD balance (20 × 0.5 = 10 EUR) remains
    assert_eq!(summaries[0].summary_status, AccountSummaryStatus::Ok);
    assert_eq!(summaries[0].cash_total_amount, Some(amt("10.000000")));
    assert_eq!(summaries[0].asset_total_amount, Some(amt("100.000000")));
    assert_eq!(summaries[0].total_amount, Some(amt("110.000000")));
    assert_eq!(summaries[0].total_currency, Some(Currency::Eur));
}

#[tokio::test]
async fn marks_summary_unavailable_when_direct_fx_rate_is_missing() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    seed_balance(&pool, account_id, Currency::Usd, amt("20.000000")).await;

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(
        summaries[0],
        AccountSummaryRecord {
            id: account_id,
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
            summary_status: AccountSummaryStatus::ConversionUnavailable,
            cash_total_amount: None,
            asset_total_amount: Some(amt("0.000000")),
            total_amount: None,
            total_currency: None,
        }
    );
}

#[tokio::test]
async fn does_not_use_inverse_fx_rates() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    seed_balance(&pool, account_id, Currency::Usd, amt("20.000000")).await;

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(
        summaries[0].summary_status,
        AccountSummaryStatus::ConversionUnavailable
    );
}

#[tokio::test]
async fn does_not_use_multi_hop_fx_rates() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Swiss Cash"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    seed_balance(&pool, account_id, Currency::Chf, amt("20.000000")).await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.800000"),
        },
    )
    .await
    .expect("direct usd eur rate insert should succeed");

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(
        summaries[0].summary_status,
        AccountSummaryStatus::ConversionUnavailable
    );
}

#[tokio::test]
async fn rounds_after_summing_converted_balances() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Precise FX"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    for currency in [Currency::Usd, Currency::Gbp] {
        seed_balance(&pool, account_id, currency, amt("1.000000")).await;
    }

    for (from_currency, rate) in [(Currency::Usd, "0.333333"), (Currency::Gbp, "0.333333")] {
        upsert_fx_rate(
            &pool,
            UpsertFxRateInput {
                from_currency,
                to_currency: Currency::Eur,
                rate: fx_rate(rate),
            },
        )
        .await
        .expect("fx rate insert should succeed");
    }

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(summaries[0].total_amount, Some(amt("0.666666")));
}

#[tokio::test]
async fn rejects_corrupt_account_rows_at_the_schema_level() {
    let pool = test_pool().await;

    let error = sqlx::query(
        "INSERT INTO accounts (name, account_type, base_currency, created_at) VALUES ('', 'bank', 'EUR', '2026-03-22 00:00:00')",
    )
    .execute(&pool)
    .await
    .expect_err("corrupt account row should be rejected");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[tokio::test]
async fn rejects_invalid_balance_amounts_at_the_schema_level() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    let error = sqlx::query(
        "INSERT INTO cash_entries (account_id, currency, amount, source, created_at) VALUES (?, 'USD', '1.1234567', 'deposit', '2026-03-22T00:00:00Z')",
    )
    .bind(account_id.as_i64())
    .execute(&pool)
    .await
    .expect_err("invalid cash entry amount should be rejected");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[tokio::test]
async fn rejects_invalid_fx_rates_at_the_schema_level() {
    let pool = test_pool().await;

    let format_error = sqlx::query(
        "INSERT INTO fx_rates (from_currency, to_currency, rate, updated_at) VALUES ('USD', 'EUR', '1.1234567', '2026-03-22 00:00:00')",
    )
    .execute(&pool)
    .await
    .expect_err("invalid fx rate format should be rejected");

    assert!(format_error.to_string().contains("CHECK constraint failed"));

    let zero_error = sqlx::query(
        "INSERT INTO fx_rates (from_currency, to_currency, rate, updated_at) VALUES ('USD', 'EUR', '0.000000', '2026-03-22 00:00:00')",
    )
    .execute(&pool)
    .await
    .expect_err("non-positive fx rate should be rejected");

    assert!(zero_error.to_string().contains("CHECK constraint failed"));
}

#[tokio::test]
async fn calculates_portfolio_summary_with_currency_breakdown_and_converted_amounts() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Portfolio"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");

    for (currency, value) in [(Currency::Eur, "100.000000"), (Currency::Usd, "100.000000")] {
        seed_balance(&pool, account_id, currency, amt(value)).await;
    }

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.900000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .expect("portfolio summary should succeed");

    assert_eq!(summary.total_value_amount, Some(amt("190.000000")));
    assert_eq!(summary.cash_by_currency.len(), 2);

    // EUR breakdown
    assert_eq!(summary.cash_by_currency[0].currency, Currency::Eur);
    assert_eq!(summary.cash_by_currency[0].amount, amt("100.000000"));
    assert_eq!(
        summary.cash_by_currency[0].converted_amount,
        Some(amt("100.000000"))
    );

    // USD breakdown
    assert_eq!(summary.cash_by_currency[1].currency, Currency::Usd);
    assert_eq!(summary.cash_by_currency[1].amount, amt("100.000000"));
    assert_eq!(
        summary.cash_by_currency[1].converted_amount,
        Some(amt("90.000000"))
    );
}

#[tokio::test]
async fn ensures_portfolio_total_matches_sum_of_cash_by_currency() {
    let pool = test_pool().await;

    // Use a rate that causes rounding after 6 digits
    // 0.333333 * 2 = 0.666666
    // If we have 2 accounts with 1 unit each, total is 0.666666
    // If we sum first: (1+1) * 0.333333 = 0.666666
    // To see a difference, we need something more subtle.
    // 0.1234567 would be rejected under the strict 6-decimal contract.

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.123456"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    // Account 1: 1.1 USD -> 0.1358016
    // Account 2: 1.1 USD -> 0.1358016
    // Sum of accounts: 0.2716032
    // Sum of currency: (1.1 + 1.1) * 0.123456 = 0.2716032

    // Let's try:
    // Rate: 0.111111
    // Acc 1: 0.5 USD -> 0.0555555 -> 0.055556
    // Acc 2: 0.5 USD -> 0.0555555 -> 0.055556
    // Sum accounts: 0.111112
    // Sum currency: (0.5 + 0.5) * 0.111111 = 0.111111

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.111111"),
        },
    )
    .await
    .ok(); // overwrite

    for name in ["A1", "A2"] {
        let account_id = create_account(
            &pool,
            CreateAccountInput {
                name: account_name(name),
                account_type: AccountType::Broker,
                base_currency: Currency::Eur,
            },
        )
        .await
        .unwrap();

        seed_balance(&pool, account_id, Currency::Usd, amt("0.5")).await;
    }

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .unwrap();

    let sum_of_currency = summary
        .cash_by_currency
        .iter()
        .map(|c| {
            c.converted_amount
                .as_ref()
                .map(|am| am.as_decimal())
                .unwrap_or(rust_decimal::Decimal::ZERO)
        })
        .sum::<rust_decimal::Decimal>();

    assert_eq!(
        summary.total_value_amount.unwrap().as_decimal(),
        sum_of_currency,
        "Portfolio total should match sum of converted currency amounts"
    );
}

#[tokio::test]
async fn allocation_totals_groups_cash_balances_into_cash_slice() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Eur, amt("500.000000")).await;

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .unwrap();

    assert_eq!(summary.allocation_totals.len(), 1);
    assert_eq!(summary.allocation_totals[0].label, "Cash");
    assert_eq!(summary.allocation_totals[0].amount, amt("500.000000"));
    assert!(!summary.allocation_is_partial);
}

#[tokio::test]
async fn allocation_totals_groups_asset_positions_by_type() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    // Pre-fund account to cover both BUYs (200 + 100 EUR)
    seed_balance(&pool, account_id, Currency::Eur, amt("300.000000")).await;

    // Stock asset: 10 shares at 20 EUR each = 200 EUR
    let stock_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("ACME"),
            name: asset_name("Acme Corp"),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id: stock_id,
            price: asset_unit_price("20.000000"),
            currency: Currency::Eur,
            as_of: "2026-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .unwrap();

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id: stock_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-01-01"),
            quantity: asset_quantity("10.000000"),
            unit_price: asset_unit_price("20.000000"),
            currency_code: Currency::Eur,
            notes: None,
        },
    )
    .await
    .unwrap();

    // Crypto asset: 2 coins at 50 EUR each = 100 EUR
    let crypto_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("COIN"),
            name: asset_name("Some Coin"),
            asset_type: AssetType::Crypto,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id: crypto_id,
            price: asset_unit_price("50.000000"),
            currency: Currency::Eur,
            as_of: "2026-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .unwrap();

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id: crypto_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-01-01"),
            quantity: asset_quantity("2.000000"),
            unit_price: asset_unit_price("50.000000"),
            currency_code: Currency::Eur,
            notes: None,
        },
    )
    .await
    .unwrap();

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .unwrap();

    let mut slices = summary.allocation_totals;
    slices.sort_by(|a, b| a.label.cmp(&b.label));

    assert_eq!(slices.len(), 2);
    assert_eq!(slices[0].label, "Crypto");
    assert_eq!(slices[0].amount, amt("100.000000"));
    assert_eq!(slices[1].label, "Stock");
    assert_eq!(slices[1].amount, amt("200.000000"));
    assert!(!summary.allocation_is_partial);
}

#[tokio::test]
async fn portfolio_summary_includes_daily_and_total_gain_amounts() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_id, Currency::Usd, amt("1000.000000")).await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.900000"),
        },
    )
    .await
    .unwrap();

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: asset_unit_price("100.000000"),
            currency: Currency::Usd,
            as_of: "2020-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .unwrap();

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: asset_unit_price("120.000000"),
            currency: Currency::Usd,
            as_of: "2999-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .unwrap();

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2024-01-01"),
            quantity: asset_quantity("10.000000"),
            unit_price: asset_unit_price("90.000000"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap();

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .unwrap();

    assert_eq!(summary.gain_24h_amount, Some(amt("180.000000")));
    assert_eq!(summary.total_gain_amount, Some(amt("270.000000")));
}

#[tokio::test]
async fn total_gain_is_available_when_same_asset_bought_in_different_currencies_across_accounts() {
    // Regression test: when the same asset is purchased with different transaction currencies
    // across accounts (e.g. USD in one, EUR in another), the global avg_cost_basis_currency
    // used to be NULL, causing total_gain_amount to return None ("Unavailable"). This test
    // verifies it is computed correctly using the per-account, per-transaction fx_rate instead.
    let pool = test_pool().await;

    let account_a = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("USD Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    let account_b = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("EUR Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    seed_balance(&pool, account_a, Currency::Usd, amt("10000.000000")).await;
    seed_balance(&pool, account_b, Currency::Eur, amt("10000.000000")).await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.900000"),
        },
    )
    .await
    .unwrap();

    // Previous close price (more than 24 hours old)
    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: asset_unit_price("100.000000"),
            currency: Currency::Usd,
            as_of: "2020-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .unwrap();

    // Current price
    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: asset_unit_price("120.000000"),
            currency: Currency::Usd,
            as_of: "2999-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .unwrap();

    // Account A (USD base) buys 10 AAPL @ $90 USD — fx_rate stored as 1.0 (USD→USD)
    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id: account_a,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2024-01-01"),
            quantity: asset_quantity("10.000000"),
            unit_price: asset_unit_price("90.000000"),
            currency_code: Currency::Usd,
            notes: None,
        },
    )
    .await
    .unwrap();

    // Account B (EUR base) buys 5 AAPL @ €95 EUR — fx_rate stored as 1.0 (EUR→EUR)
    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id: account_b,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2024-01-01"),
            quantity: asset_quantity("5.000000"),
            unit_price: asset_unit_price("95.000000"),
            currency_code: Currency::Eur,
            notes: None,
        },
    )
    .await
    .unwrap();

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .unwrap();

    // Account A: gain/share = (120 * 0.90) − (90 * 0.90) = 27 EUR → 270 EUR for 10 shares
    // Account B: gain/share = (120 * 0.90) − (95 * 1.00) = 13 EUR → 65 EUR for 5 shares
    // Total = 335 EUR
    assert_eq!(summary.total_gain_amount, Some(amt("335.000000")));
    // 24h gain: (120 − 100) * 0.90 * 15 = 270 EUR
    assert_eq!(summary.gain_24h_amount, Some(amt("270.000000")));
}

#[tokio::test]
async fn allocation_totals_marks_partial_when_asset_has_no_price() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    // Cash balance: fully chartable
    seed_balance(&pool, account_id, Currency::Eur, amt("100.000000")).await;

    // Asset with no price
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("NOPRICE"),
            name: asset_name("No Price Asset"),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2026-01-01"),
            quantity: asset_quantity("5.000000"),
            unit_price: asset_unit_price("10.000000"),
            currency_code: Currency::Eur,
            notes: None,
        },
    )
    .await
    .unwrap();

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .unwrap();

    // Cash slice still present; stock slice absent because no current price
    assert_eq!(summary.allocation_totals.len(), 1);
    assert_eq!(summary.allocation_totals[0].label, "Cash");
    assert!(summary.allocation_is_partial);
}

#[tokio::test]
async fn allocation_totals_marks_partial_when_fx_unavailable_for_cash() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Bank"),
            account_type: AccountType::Bank,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    // EUR balance: chartable (same as display)
    seed_balance(&pool, account_id, Currency::Eur, amt("200.000000")).await;

    // USD balance: no FX rate to EUR available
    seed_balance(&pool, account_id, Currency::Usd, amt("100.000000")).await;

    // No FX rate inserted — USD→EUR conversion unavailable
    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .unwrap();

    // Only the EUR cash balance is chartable
    assert_eq!(summary.allocation_totals.len(), 1);
    assert_eq!(summary.allocation_totals[0].label, "Cash");
    assert_eq!(summary.allocation_totals[0].amount, amt("200.000000"));
    assert!(summary.allocation_is_partial);
}

#[tokio::test]
async fn allocation_totals_is_empty_when_nothing_is_chartable() {
    let pool = test_pool().await;

    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::Usd,
        },
    )
    .await
    .unwrap();

    // USD cash balance but display currency is EUR and no FX rate
    seed_balance(&pool, account_id, Currency::Usd, amt("100.000000")).await;

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .unwrap();

    assert!(summary.allocation_totals.is_empty());
    assert!(summary.allocation_is_partial);
}

#[tokio::test]
async fn portfolio_holdings_aggregate_same_asset_across_accounts() {
    let pool = test_pool().await;

    let first_account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker One"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();
    let second_account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker Two"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .unwrap();

    for (account_id, amount) in [
        (first_account_id, "150.000000"),
        (second_account_id, "200.000000"),
    ] {
        seed_balance(&pool, account_id, Currency::Eur, amt(amount)).await;
    }

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VWCE"),
            name: asset_name("Vanguard FTSE All-World UCITS ETF"),
            asset_type: AssetType::Etf,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: asset_unit_price("100.000000"),
            currency: Currency::Eur,
            as_of: "2026-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .unwrap();

    for (account_id, quantity) in [
        (first_account_id, "1.500000"),
        (second_account_id, "2.000000"),
    ] {
        create_asset_transaction(
            &pool,
            CreateAssetTransactionInput {
                account_id,
                asset_id,
                transaction_type: AssetTransactionType::Buy,
                trade_date: trade_date("2026-01-01"),
                quantity: asset_quantity(quantity),
                unit_price: asset_unit_price("100.000000"),
                currency_code: Currency::Eur,
                notes: None,
            },
        )
        .await
        .unwrap();
    }

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .unwrap();

    assert_eq!(summary.holdings.len(), 1);
    assert_eq!(summary.holdings[0].asset_id, Some(asset_id));
    assert_eq!(summary.holdings[0].symbol, "VWCE");
    assert_eq!(
        summary.holdings[0].name,
        "Vanguard FTSE All-World UCITS ETF"
    );
    assert_eq!(summary.holdings[0].value, amt("350.000000"));
    assert!(!summary.holdings_is_partial);
}

#[tokio::test]
async fn insert_portfolio_snapshot_stores_one_entry_per_day() {
    let pool = test_pool().await;

    insert_portfolio_snapshot_if_missing(
        &pool,
        amt("1000.000000"),
        Currency::Eur,
        "2025-01-01T10:00:00Z",
    )
    .await
    .expect("first insert should succeed");

    // Second call on the same day is silently ignored (INSERT OR IGNORE)
    insert_portfolio_snapshot_if_missing(
        &pool,
        amt("2000.000000"),
        Currency::Eur,
        "2025-01-01T18:00:00Z",
    )
    .await
    .expect("duplicate insert should be ignored");

    let snapshots = list_portfolio_snapshots(&pool, Currency::Eur)
        .await
        .expect("list should succeed");

    assert_eq!(snapshots.len(), 1);
    assert_eq!(
        snapshots[0],
        PortfolioSnapshotRecord {
            total_value: amt("1000.000000"),
            currency: Currency::Eur,
            recorded_at: "2025-01-01T10:00:00Z".to_string(),
        }
    );
}

#[tokio::test]
async fn list_portfolio_snapshots_returns_snapshots_in_chronological_order() {
    let pool = test_pool().await;

    insert_portfolio_snapshot_if_missing(
        &pool,
        amt("1000.000000"),
        Currency::Eur,
        "2025-01-01T10:00:00Z",
    )
    .await
    .unwrap();

    insert_portfolio_snapshot_if_missing(
        &pool,
        amt("1100.000000"),
        Currency::Eur,
        "2025-01-02T10:00:00Z",
    )
    .await
    .unwrap();

    insert_portfolio_snapshot_if_missing(
        &pool,
        amt("900.000000"),
        Currency::Eur,
        "2025-01-03T10:00:00Z",
    )
    .await
    .unwrap();

    let snapshots = list_portfolio_snapshots(&pool, Currency::Eur)
        .await
        .expect("list should succeed");

    assert_eq!(snapshots.len(), 3);
    assert_eq!(snapshots[0].total_value, amt("1000.000000"));
    assert_eq!(snapshots[1].total_value, amt("1100.000000"));
    assert_eq!(snapshots[2].total_value, amt("900.000000"));
}

#[tokio::test]
async fn list_portfolio_snapshots_filters_by_currency() {
    let pool = test_pool().await;

    insert_portfolio_snapshot_if_missing(
        &pool,
        amt("1000.000000"),
        Currency::Eur,
        "2025-01-01T10:00:00Z",
    )
    .await
    .unwrap();

    insert_portfolio_snapshot_if_missing(
        &pool,
        amt("1100.000000"),
        Currency::Usd,
        "2025-01-01T10:00:00Z",
    )
    .await
    .unwrap();

    let eur = list_portfolio_snapshots(&pool, Currency::Eur)
        .await
        .expect("EUR list should succeed");
    let usd = list_portfolio_snapshots(&pool, Currency::Usd)
        .await
        .expect("USD list should succeed");

    assert_eq!(eur.len(), 1);
    assert_eq!(eur[0].total_value, amt("1000.000000"));
    assert_eq!(usd.len(), 1);
    assert_eq!(usd[0].total_value, amt("1100.000000"));
}

#[tokio::test]
async fn recalculate_snapshots_removes_snapshot_when_recompute_cannot_price_position() {
    let pool = test_pool().await;
    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");
    seed_balance(&pool, account_id, Currency::Eur, amt("100.000000")).await;
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("ACME"),
            name: asset_name("Acme Corp"),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2025-01-01"),
            quantity: asset_quantity("1.000000"),
            unit_price: asset_unit_price("10.000000"),
            currency_code: Currency::Eur,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");
    insert_portfolio_snapshot_if_missing(
        &pool,
        amt("100.000000"),
        Currency::Eur,
        "2025-01-02T10:00:00Z",
    )
    .await
    .expect("snapshot insert should succeed");

    recalculate_snapshots_from_date(&pool, "2025-01-01", Currency::Eur)
        .await
        .expect("recalculation should remove snapshot when replacement cannot be computed");

    let snapshots = list_portfolio_snapshots(&pool, Currency::Eur)
        .await
        .expect("snapshot list should succeed");
    assert_eq!(snapshots.len(), 0);
}

#[tokio::test]
async fn create_asset_transaction_recalculates_existing_snapshots_from_trade_date() {
    let pool = test_pool().await;
    let account_id = create_account(
        &pool,
        CreateAccountInput {
            name: account_name("IBKR"),
            account_type: AccountType::Broker,
            base_currency: Currency::Eur,
        },
    )
    .await
    .expect("account insert should succeed");
    seed_balance(&pool, account_id, Currency::Eur, amt("200.000000")).await;
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("ACME"),
            name: asset_name("Acme Corp"),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");
    upsert_asset_price(
        &pool,
        UpsertAssetPriceInput {
            asset_id,
            price: asset_unit_price("20.000000"),
            currency: Currency::Eur,
            as_of: "2025-01-01T00:00:00Z".to_string(),
        },
    )
    .await
    .expect("asset price insert should succeed");
    insert_portfolio_snapshot_if_missing(
        &pool,
        amt("200.000000"),
        Currency::Eur,
        "2025-01-02T10:00:00Z",
    )
    .await
    .expect("snapshot insert should succeed");

    create_asset_transaction(
        &pool,
        CreateAssetTransactionInput {
            account_id,
            asset_id,
            transaction_type: AssetTransactionType::Buy,
            trade_date: trade_date("2025-01-01"),
            quantity: asset_quantity("10.000000"),
            unit_price: asset_unit_price("10.000000"),
            currency_code: Currency::Eur,
            notes: None,
        },
    )
    .await
    .expect("transaction insert should succeed");

    let snapshots = list_portfolio_snapshots(&pool, Currency::Eur)
        .await
        .expect("snapshot list should succeed");

    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].total_value, amt("300.000000"));
}

#[tokio::test]
async fn fx_rate_history_migration_backfills_existing_fx_rates() {
    let pool = legacy_v2_pool_with_fx_rate().await;

    init_db(&pool)
        .await
        .expect("schema should migrate from v2 to latest");

    let history_rate: Option<i64> = sqlx::query_scalar(
        "SELECT rate FROM fx_rate_history WHERE from_currency = ? AND to_currency = ?",
    )
    .bind(Currency::Usd.as_str())
    .bind(Currency::Eur.as_str())
    .fetch_optional(&pool)
    .await
    .expect("history lookup should succeed");

    assert_eq!(history_rate, Some(fx_rate("0.500000").as_scaled_i64()));
}
