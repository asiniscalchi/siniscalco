use std::str::FromStr;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tempfile::NamedTempFile;
use tokio::sync::Barrier;

use super::{
    AccountBalanceRecord, AccountId, AccountName, AccountRecord, AccountSummaryRecord,
    AccountSummaryStatus, AccountType, Amount, AssetId, AssetName, AssetPositionRecord,
    AssetQuantity, AssetRecord, AssetSymbol, AssetTransactionType, AssetType, AssetUnitPrice,
    CreateAccountInput, CreateAssetInput, CreateAssetTransactionInput, Currency, CurrencyRecord,
    FxRate, FxRateDetailRecord, FxRateRecord, FxRateSummaryItemRecord, FxRateSummaryRecord,
    StorageError, TradeDate, UpsertAccountBalanceInput, UpsertFxRateInput, UpsertOutcome,
    create_account, create_asset, create_asset_transaction, delete_account, delete_account_balance,
    get_account, get_latest_fx_rate, list_account_balances, list_account_positions,
    list_account_summaries, list_accounts, list_asset_transactions, list_assets, list_currencies,
    list_fx_rate_summary, list_fx_rates, upsert_account_balance, upsert_fx_rate,
};
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
async fn lists_assets_in_symbol_order() {
    let pool = test_pool().await;

    create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("VTI"),
            name: asset_name("Vanguard Total Stock Market ETF"),
            asset_type: AssetType::Etf,
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
                isin: None,
                created_at: assets[0].created_at.clone(),
                updated_at: assets[0].updated_at.clone(),
            },
            AssetRecord {
                id: asset_id(1),
                symbol: asset_symbol("VTI"),
                name: asset_name("Vanguard Total Stock Market ETF"),
                asset_type: AssetType::Etf,
                isin: Some("US9229087690".to_string()),
                created_at: assets[1].created_at.clone(),
                updated_at: assets[1].updated_at.clone(),
            },
        ]
    );
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

    let aapl_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("AAPL"),
            name: asset_name("Apple Inc."),
            asset_type: AssetType::Stock,
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

    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("BIG"),
            name: asset_name("Big Asset"),
            asset_type: AssetType::Stock,
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
            quantity: asset_quantity("999999999999.12345678"),
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
                rust_decimal::Decimal::from_str("999999999999.12345678").unwrap(),
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
    let asset_id = create_asset(
        &pool,
        CreateAssetInput {
            symbol: asset_symbol("BTC"),
            name: asset_name("Bitcoin"),
            asset_type: AssetType::Crypto,
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
            isin: None,
        },
    )
    .await
    .expect("asset insert should succeed");

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

    for (currency, value) in [("EUR", "12000.00000000"), ("USD", "3500.00000000")] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: Currency::try_from(currency).unwrap(),
                amount: amt(value),
            },
        )
        .await
        .expect("balance insert should succeed");
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
    assert_eq!(balances[0].updated_at.len(), 19);
    assert_eq!(balances[1].updated_at.len(), 19);
}

#[tokio::test]
async fn upsert_updates_existing_balance() {
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

    let first_outcome = upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("10.00000000"),
        },
    )
    .await
    .expect("first balance insert should succeed");
    assert_eq!(first_outcome, UpsertOutcome::Created);

    let second_outcome = upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("12.00000000"),
        },
    )
    .await
    .expect("upsert should update the existing balance");
    assert_eq!(second_outcome, UpsertOutcome::Updated);

    let balances = list_account_balances(&pool, account_id)
        .await
        .expect("balance list should succeed");

    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].amount, amt("12"));
    assert_eq!(balances[0].updated_at.len(), 19);
}

#[tokio::test]
async fn deletes_single_balance() {
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

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Eur,
            amount: amt("12000.00000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    delete_account_balance(&pool, account_id, Currency::Eur)
        .await
        .expect("balance delete should succeed");

    let balances = list_account_balances(&pool, account_id)
        .await
        .expect("balance list should succeed");

    assert!(balances.is_empty());
}

#[tokio::test]
async fn deleting_missing_balance_returns_not_found() {
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

    let error = delete_account_balance(&pool, account_id, Currency::Usd)
        .await
        .expect_err("missing balance delete should fail");

    match error {
        StorageError::Database(sqlx::Error::RowNotFound) => {}
        other => panic!("expected RowNotFound, got {other}"),
    }
}

#[tokio::test]
async fn deletes_account_and_cascades_balances() {
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

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Eur,
            amount: amt("12000.00000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    delete_account(&pool, account_id)
        .await
        .expect("account delete should succeed");

    let account_error = get_account(&pool, account_id)
        .await
        .expect_err("deleted account should not exist");
    let balances = list_account_balances(&pool, account_id)
        .await
        .expect("balance list should still succeed");

    match account_error {
        StorageError::Database(sqlx::Error::RowNotFound) => {}
        other => panic!("expected RowNotFound, got {other}"),
    }
    assert!(balances.is_empty());
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
    assert_eq!(accounts[0].created_at.len(), 19);
}

#[tokio::test]
async fn rejects_invalid_account_type_input() {
    let error = AccountType::try_from("cash").expect_err("unsupported account type should fail");

    assert_eq!(
        error.to_string(),
        "account_type must be one of: bank, broker"
    );
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

    let outcome = upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("10.00000000"),
        },
    )
    .await
    .expect("typed currency should succeed");

    assert_eq!(outcome, UpsertOutcome::Created);
}

#[test]
fn rejects_invalid_typed_amount_input() {
    let error = Amount::try_from("1.123456789").expect_err("invalid amount should fail");

    assert_eq!(error.to_string(), "amount must match DECIMAL(20,8)");
}

#[tokio::test]
async fn rejects_balance_for_missing_account() {
    let pool = test_pool().await;

    let error = upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id: account_id(999),
            currency: Currency::Usd,
            amount: amt("10.00000000"),
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
            rate: fx_rate("0.92000000"),
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
            rate: fx_rate("0.92000000"),
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
    assert_eq!(rate.updated_at.len(), 19);
}

#[tokio::test]
async fn updates_existing_fx_rate() {
    let pool = test_pool().await;

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.92000000"),
        },
    )
    .await
    .expect("initial fx rate insert should succeed");

    let outcome = upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.91000000"),
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
    let error = FxRate::try_from("0.00000000").expect_err("zero fx rate should fail");

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
            rate: fx_rate("1.00000000"),
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
            rate: fx_rate("0.78000000"),
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
        (Currency::Usd, "0.92000000"),
        (Currency::Gbp, "1.17000000"),
        (Currency::Chf, "1.04000000"),
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
            total_amount: Some(amt("0.00000000")),
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

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("123.45000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    let summaries = list_account_summaries(&pool)
        .await
        .expect("account summaries should succeed");

    assert_eq!(summaries[0].summary_status, AccountSummaryStatus::Ok);
    assert_eq!(summaries[0].total_amount, Some(amt("123.45000000")));
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
        (Currency::Eur, "10.00000000"),
        (Currency::Usd, "20.00000000"),
        (Currency::Gbp, "30.00000000"),
    ] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt(value),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    for (from_currency, rate) in [(Currency::Usd, "0.50000000"), (Currency::Gbp, "1.20000000")] {
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
    assert_eq!(summaries[0].total_amount, Some(amt("56.00000000")));
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

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("20.00000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

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

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Usd,
            amount: amt("20.00000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

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

    upsert_account_balance(
        &pool,
        UpsertAccountBalanceInput {
            account_id,
            currency: Currency::Chf,
            amount: amt("20.00000000"),
        },
    )
    .await
    .expect("balance insert should succeed");

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.80000000"),
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
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt("1.00000000"),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    for (from_currency, rate) in [(Currency::Usd, "0.33333333"), (Currency::Gbp, "0.33333333")] {
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

    assert_eq!(summaries[0].total_amount, Some(amt("0.66666666")));
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
        "INSERT INTO account_balances (account_id, currency, amount, updated_at) VALUES (?, 'USD', '1.123456789', '2026-03-22 00:00:00')",
    )
    .bind(account_id.as_i64())
    .execute(&pool)
    .await
    .expect_err("invalid balance amount should be rejected");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[tokio::test]
async fn rejects_invalid_fx_rates_at_the_schema_level() {
    let pool = test_pool().await;

    let format_error = sqlx::query(
        "INSERT INTO fx_rates (from_currency, to_currency, rate, updated_at) VALUES ('USD', 'EUR', '1.123456789', '2026-03-22 00:00:00')",
    )
    .execute(&pool)
    .await
    .expect_err("invalid fx rate format should be rejected");

    assert!(format_error.to_string().contains("CHECK constraint failed"));

    let zero_error = sqlx::query(
        "INSERT INTO fx_rates (from_currency, to_currency, rate, updated_at) VALUES ('USD', 'EUR', '0.00000000', '2026-03-22 00:00:00')",
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

    for (currency, value) in [
        (Currency::Eur, "100.00000000"),
        (Currency::Usd, "100.00000000"),
    ] {
        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency,
                amount: amt(value),
            },
        )
        .await
        .expect("balance insert should succeed");
    }

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.90000000"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    let summary = super::get_portfolio_summary(&pool, Currency::Eur)
        .await
        .expect("portfolio summary should succeed");

    assert_eq!(summary.total_value_amount, Some(amt("190.00000000")));
    assert_eq!(summary.cash_by_currency.len(), 2);

    // EUR breakdown
    assert_eq!(summary.cash_by_currency[0].currency, Currency::Eur);
    assert_eq!(summary.cash_by_currency[0].amount, amt("100.00000000"));
    assert_eq!(
        summary.cash_by_currency[0].converted_amount,
        Some(amt("100.00000000"))
    );

    // USD breakdown
    assert_eq!(summary.cash_by_currency[1].currency, Currency::Usd);
    assert_eq!(summary.cash_by_currency[1].amount, amt("100.00000000"));
    assert_eq!(
        summary.cash_by_currency[1].converted_amount,
        Some(amt("90.00000000"))
    );
}

#[tokio::test]
async fn ensures_portfolio_total_matches_sum_of_cash_by_currency() {
    let pool = test_pool().await;

    // Use a rate that causes rounding after 8 digits
    // 0.33333333 * 2 = 0.66666666
    // If we have 2 accounts with 1 unit each, total is 0.66666666
    // If we sum first: (1+1) * 0.33333333 = 0.66666666
    // To see a difference, we need something more subtle.
    // 0.123456789 -> 0.12345679

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.12345678"),
        },
    )
    .await
    .expect("fx rate insert should succeed");

    // Account 1: 1.1 USD -> 0.135802458 -> 0.13580246
    // Account 2: 1.1 USD -> 0.135802458 -> 0.13580246
    // Sum of accounts: 0.27160492
    // Sum of currency: (1.1 + 1.1) * 0.12345678 = 2.2 * 0.12345678 = 0.271604916 -> 0.27160492

    // Let's try:
    // Rate: 0.11111111
    // Acc 1: 0.5 USD -> 0.055555555 -> 0.05555556
    // Acc 2: 0.5 USD -> 0.055555555 -> 0.05555556
    // Sum accounts: 0.11111112
    // Sum currency: (0.5 + 0.5) * 0.11111111 = 0.11111111

    upsert_fx_rate(
        &pool,
        UpsertFxRateInput {
            from_currency: Currency::Usd,
            to_currency: Currency::Eur,
            rate: fx_rate("0.11111111"),
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

        upsert_account_balance(
            &pool,
            UpsertAccountBalanceInput {
                account_id,
                currency: Currency::Usd,
                amount: amt("0.5"),
            },
        )
        .await
        .unwrap();
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
