use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ErrorCode, PromptMessage, PromptMessageRole, ResourceContents};

use super::resources::{
    RESOURCE_MIME_TYPE, ResourceRef, build_resource_templates, parse_resource_uri,
};
use super::*;
use crate::PRODUCT_BASE_CURRENCY;
use crate::storage::{
    AccountId, AccountName, AccountType, AssetId, AssetName, AssetSymbol, AssetType,
    CreateAccountInput, CreateAssetInput, Currency,
};
use crate::{init_db, storage::create_account, storage::create_asset};

async fn test_pool() -> SqlitePool {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    init_db(&pool).await.unwrap();
    pool
}

fn account_name(s: &str) -> AccountName {
    AccountName::try_from(s).unwrap()
}

fn text_of(result: &CallToolResult) -> &str {
    &result.content[0].as_text().expect("text content").text
}

async fn seed_account(pool: &SqlitePool, name: &str) -> i64 {
    let id = create_account(
        pool,
        CreateAccountInput {
            name: account_name(name),
            account_type: AccountType::Broker,
            base_currency: Currency::try_from("EUR").unwrap(),
        },
    )
    .await
    .unwrap();
    id.as_i64()
}

async fn seed_asset(pool: &SqlitePool, symbol: &str) -> i64 {
    let id = create_asset(
        pool,
        CreateAssetInput {
            symbol: AssetSymbol::try_from(symbol).unwrap(),
            name: AssetName::try_from("Vanguard FTSE All-World").unwrap(),
            asset_type: AssetType::Etf,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();
    id.as_i64()
}

async fn seed_opening_transaction(pool: &SqlitePool, account_id: i64, asset_id: i64) -> i64 {
    let server = PortfolioServer::new(pool.clone());
    let result = server
        .create_transaction(Parameters(CreateTransactionArgs {
            account_id,
            asset_id,
            transaction_type: "OPENING".to_string(),
            trade_date: "2026-01-02".to_string(),
            quantity: "10".to_string(),
            unit_price: "100.00".to_string(),
            currency_code: "EUR".to_string(),
            notes: None,
        }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    // Parse the id out of "Transaction created with id N. ..."
    let text = text_of(&result);
    let after = text.split_once("id ").unwrap().1;
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap()
}

#[tokio::test]
async fn list_tools_returns_remaining_tool_set() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let tools = server.tool_router.list_all();
    let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "create_account",
            "create_asset",
            "create_cash_movement",
            "create_transaction",
            "create_transfer",
            "delete_account",
            "delete_asset",
            "delete_transaction",
            "list_accounts",
            "list_assets",
            "list_transactions",
            "update_transaction",
        ]
    );
}

#[tokio::test]
async fn list_accounts_empty_db() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let result = server.list_accounts().await;
    assert!(!result.is_error.unwrap_or(false));
    let text = &result.content[0].as_text().expect("text content").text;
    assert_eq!(text, "No accounts found.");
}

#[tokio::test]
async fn list_accounts_with_data() {
    let pool = test_pool().await;
    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Test Broker"),
            account_type: AccountType::Broker,
            base_currency: Currency::try_from("EUR").unwrap(),
        },
    )
    .await
    .unwrap();
    let server = PortfolioServer::new(pool);
    let result = server.list_accounts().await;
    assert!(!result.is_error.unwrap_or(false));
    let text = &result.content[0].as_text().expect("text content").text;
    assert!(text.contains("Test Broker"), "{text}");
}

#[tokio::test]
async fn list_transactions_empty_db() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let result = server
        .list_transactions(Parameters(LimitArgs { limit: None }))
        .await;
    assert!(!result.is_error.unwrap_or(false));
    let text = &result.content[0].as_text().expect("text content").text;
    assert_eq!(text, "No transactions found.");
}

#[test]
fn parse_resource_uri_recognises_supported_schemes() {
    assert_eq!(
        parse_resource_uri("account://1"),
        Some(ResourceRef::Account(AccountId::try_from(1).unwrap()))
    );
    assert_eq!(
        parse_resource_uri("asset://7"),
        Some(ResourceRef::Asset(AssetId::try_from(7).unwrap()))
    );
    assert_eq!(
        parse_resource_uri("portfolio://summary"),
        Some(ResourceRef::PortfolioSummary)
    );
    assert_eq!(
        parse_resource_uri("portfolio://snapshots"),
        Some(ResourceRef::PortfolioSnapshots)
    );
    assert_eq!(
        parse_resource_uri("portfolio://allocation"),
        Some(ResourceRef::PortfolioAllocation)
    );
}

#[test]
fn parse_resource_uri_rejects_unknown_or_malformed_uris() {
    assert!(parse_resource_uri("account://not-a-number").is_none());
    assert!(parse_resource_uri("account://0").is_none()); // AccountId rejects 0
    assert!(parse_resource_uri("asset://-1").is_none());
    assert!(parse_resource_uri("portfolio://unknown").is_none());
    assert!(parse_resource_uri("file:///etc/passwd").is_none());
}

#[tokio::test]
async fn list_resources_includes_accounts_assets_and_portfolio_singletons() {
    let pool = test_pool().await;
    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker A"),
            account_type: AccountType::Broker,
            base_currency: Currency::try_from("EUR").unwrap(),
        },
    )
    .await
    .unwrap();
    create_asset(
        &pool,
        CreateAssetInput {
            symbol: AssetSymbol::try_from("AAPL").unwrap(),
            name: AssetName::try_from("Apple Inc.").unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: None,
        },
    )
    .await
    .unwrap();

    let server = PortfolioServer::new(pool);
    let resources = server.list_resources_inner().await.unwrap();
    let uris: Vec<&str> = resources.iter().map(|r| r.uri.as_str()).collect();

    assert!(uris.contains(&"account://1"), "{uris:?}");
    assert!(uris.contains(&"asset://1"), "{uris:?}");
    assert!(uris.contains(&"portfolio://summary"), "{uris:?}");
    assert!(uris.contains(&"portfolio://snapshots"), "{uris:?}");
    assert!(uris.contains(&"portfolio://allocation"), "{uris:?}");
}

#[tokio::test]
async fn read_resource_returns_account_json() {
    let pool = test_pool().await;
    create_account(
        &pool,
        CreateAccountInput {
            name: account_name("Broker A"),
            account_type: AccountType::Broker,
            base_currency: Currency::try_from("EUR").unwrap(),
        },
    )
    .await
    .unwrap();
    let server = PortfolioServer::new(pool);
    let result = server.read_resource_by_uri("account://1").await.unwrap();
    let ResourceContents::TextResourceContents {
        text, mime_type, ..
    } = &result.contents[0]
    else {
        panic!("expected text contents");
    };
    assert_eq!(mime_type.as_deref(), Some(RESOURCE_MIME_TYPE));
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["id"], 1);
    assert_eq!(parsed["name"], "Broker A");
    assert_eq!(parsed["account_type"], "broker");
    assert_eq!(parsed["base_currency"], "EUR");
    assert!(parsed["balances"].is_array());
    assert!(parsed["positions"].is_array());
    assert!(parsed["transfers"].is_array());
}

#[tokio::test]
async fn read_resource_returns_asset_json() {
    let pool = test_pool().await;
    create_asset(
        &pool,
        CreateAssetInput {
            symbol: AssetSymbol::try_from("AAPL").unwrap(),
            name: AssetName::try_from("Apple Inc.").unwrap(),
            asset_type: AssetType::Stock,
            quote_symbol: None,
            isin: Some("US0378331005".to_string()),
        },
    )
    .await
    .unwrap();
    let server = PortfolioServer::new(pool);
    let result = server.read_resource_by_uri("asset://1").await.unwrap();
    let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] else {
        panic!("expected text contents");
    };
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["id"], 1);
    assert_eq!(parsed["symbol"], "AAPL");
    assert_eq!(parsed["name"], "Apple Inc.");
    assert_eq!(parsed["isin"], "US0378331005");
}

#[tokio::test]
async fn read_resource_portfolio_summary_returns_json_with_currency() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let result = server
        .read_resource_by_uri("portfolio://summary")
        .await
        .unwrap();
    let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] else {
        panic!("expected text contents");
    };
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["display_currency"], PRODUCT_BASE_CURRENCY.as_str());
    assert!(parsed["account_totals"].is_array());
}

#[tokio::test]
async fn read_resource_portfolio_snapshots_returns_empty_array_when_no_data() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let result = server
        .read_resource_by_uri("portfolio://snapshots")
        .await
        .unwrap();
    let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] else {
        panic!("expected text contents");
    };
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["currency"], PRODUCT_BASE_CURRENCY.as_str());
    assert_eq!(parsed["snapshots"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn read_resource_portfolio_allocation_returns_json() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let result = server
        .read_resource_by_uri("portfolio://allocation")
        .await
        .unwrap();
    let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] else {
        panic!("expected text contents");
    };
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["currency"], PRODUCT_BASE_CURRENCY.as_str());
    assert!(parsed["slices"].is_array());
}

#[tokio::test]
async fn read_resource_missing_account_returns_invalid_params() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let err = server
        .read_resource_by_uri("account://999")
        .await
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn read_resource_unknown_uri_returns_invalid_params() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let err = server
        .read_resource_by_uri("nope://thing")
        .await
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    assert!(err.message.contains("nope://thing"));
}

#[test]
fn resource_templates_cover_account_and_asset() {
    let templates = build_resource_templates();
    let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"account"));
    assert!(names.contains(&"asset"));
    let uris: Vec<&str> = templates.iter().map(|t| t.uri_template.as_str()).collect();
    assert!(uris.contains(&"account://{id}"));
    assert!(uris.contains(&"asset://{id}"));
}

#[tokio::test]
async fn get_info_advertises_resources_and_prompts_capability() {
    let server = PortfolioServer::new(test_pool().await);
    let info = server.get_info();
    assert!(info.capabilities.resources.is_some());
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.prompts.is_some());
}

#[tokio::test]
async fn prompt_router_lists_expected_prompts() {
    let server = PortfolioServer::new(test_pool().await);
    let prompts = server.prompt_router.list_all();
    let mut names: Vec<&str> = prompts.iter().map(|p| p.name.as_str()).collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "account_review",
            "allocation_drift_check",
            "portfolio_recap"
        ]
    );
}

#[tokio::test]
async fn portfolio_recap_prompt_emits_user_message_referencing_resources() {
    let server = PortfolioServer::new(test_pool().await);
    let messages = server.portfolio_recap_prompt().await;
    assert_eq!(messages.len(), 1);
    let PromptMessage {
        role,
        content: rmcp::model::PromptMessageContent::Text { text },
        ..
    } = &messages[0]
    else {
        panic!("expected text content");
    };
    assert!(matches!(role, PromptMessageRole::User));
    assert!(text.contains("portfolio://summary"));
    assert!(text.contains("portfolio://allocation"));
}

#[tokio::test]
async fn account_review_prompt_substitutes_account_id() {
    let server = PortfolioServer::new(test_pool().await);
    let messages = server
        .account_review_prompt(Parameters(AccountReviewArgs { account_id: 42 }))
        .await;
    let PromptMessage {
        content: rmcp::model::PromptMessageContent::Text { text },
        ..
    } = &messages[0]
    else {
        panic!("expected text content");
    };
    assert!(text.contains("account://42"));
    assert!(text.contains("account=42"));
}

#[tokio::test]
async fn allocation_drift_check_prompt_references_allocation_resource() {
    let server = PortfolioServer::new(test_pool().await);
    let messages = server.allocation_drift_check_prompt().await;
    let PromptMessage {
        content: rmcp::model::PromptMessageContent::Text { text },
        ..
    } = &messages[0]
    else {
        panic!("expected text content");
    };
    assert!(text.contains("portfolio://allocation"));
}

// ── Write tool tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn create_account_tool_creates_and_lists() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool.clone());

    let result = server
        .create_account(Parameters(CreateAccountArgs {
            name: "Interactive Brokers".to_string(),
            account_type: "Broker".to_string(),
            base_currency: "EUR".to_string(),
        }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    assert!(text_of(&result).contains("Account created with id"));

    let listing = server.list_accounts().await;
    let text = text_of(&listing);
    assert!(text.contains("Interactive Brokers"), "{text}");
}

#[tokio::test]
async fn create_account_tool_rejects_invalid_type() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let result = server
        .create_account(Parameters(CreateAccountArgs {
            name: "Bad".to_string(),
            account_type: "savings".to_string(),
            base_currency: "EUR".to_string(),
        }))
        .await;
    assert!(result.is_error.unwrap_or(false));
    assert!(text_of(&result).contains("invalid account_type"));
}

#[tokio::test]
async fn create_asset_tool_uppercases_symbol() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let result = server
        .create_asset(Parameters(CreateAssetArgs {
            symbol: "vwce".to_string(),
            name: "Vanguard FTSE All-World".to_string(),
            asset_type: "ETF".to_string(),
            quote_symbol: None,
            isin: Some("IE00BK5BQT80".to_string()),
        }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    assert!(text_of(&result).contains("Asset created with id"));

    let listing = server.list_assets().await;
    let text = text_of(&listing);
    assert!(text.contains("VWCE"), "{text}");
}

#[tokio::test]
async fn create_asset_tool_rejects_invalid_type() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool);
    let result = server
        .create_asset(Parameters(CreateAssetArgs {
            symbol: "VWCE".to_string(),
            name: "X".to_string(),
            asset_type: "COMMODITY".to_string(),
            quote_symbol: None,
            isin: None,
        }))
        .await;
    assert!(result.is_error.unwrap_or(false));
    assert!(text_of(&result).contains("invalid asset_type"));
}

#[tokio::test]
async fn create_transaction_tool_roundtrip() {
    let pool = test_pool().await;
    let account_id = seed_account(&pool, "Broker").await;
    let asset_id = seed_asset(&pool, "VWCE").await;
    let server = PortfolioServer::new(pool.clone());

    let result = server
        .create_transaction(Parameters(CreateTransactionArgs {
            account_id,
            asset_id,
            transaction_type: "BUY".to_string(),
            trade_date: "2026-01-05".to_string(),
            quantity: "12.5".to_string(),
            unit_price: "128.43".to_string(),
            currency_code: "EUR".to_string(),
            notes: Some("monthly buy".to_string()),
        }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    assert!(text_of(&result).contains("Transaction created with id"));

    let listing = server
        .list_transactions(Parameters(LimitArgs { limit: None }))
        .await;
    let text = text_of(&listing);
    assert!(text.contains("BUY"), "{text}");
    assert!(text.contains("qty=12.5"), "{text}");
}

#[tokio::test]
async fn create_transaction_tool_rejects_invalid_type_and_date() {
    let pool = test_pool().await;
    let account_id = seed_account(&pool, "Broker").await;
    let asset_id = seed_asset(&pool, "VWCE").await;
    let server = PortfolioServer::new(pool);

    let result = server
        .create_transaction(Parameters(CreateTransactionArgs {
            account_id,
            asset_id,
            transaction_type: "DIVIDEND".to_string(),
            trade_date: "2026-01-05".to_string(),
            quantity: "1".to_string(),
            unit_price: "1".to_string(),
            currency_code: "EUR".to_string(),
            notes: None,
        }))
        .await;
    assert!(result.is_error.unwrap_or(false));
    assert!(text_of(&result).contains("invalid transaction_type"));

    let result = server
        .create_transaction(Parameters(CreateTransactionArgs {
            account_id,
            asset_id,
            transaction_type: "BUY".to_string(),
            trade_date: "05/01/2026".to_string(),
            quantity: "1".to_string(),
            unit_price: "1".to_string(),
            currency_code: "EUR".to_string(),
            notes: None,
        }))
        .await;
    assert!(result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn update_transaction_tool_replaces_fields() {
    let pool = test_pool().await;
    let account_id = seed_account(&pool, "Broker").await;
    let asset_id = seed_asset(&pool, "VWCE").await;
    let tx_id = seed_opening_transaction(&pool, account_id, asset_id).await;
    let server = PortfolioServer::new(pool);

    let result = server
        .update_transaction(Parameters(UpdateTransactionArgs {
            transaction_id: tx_id,
            account_id,
            asset_id,
            transaction_type: "OPENING".to_string(),
            trade_date: "2026-01-03".to_string(),
            quantity: "20".to_string(),
            unit_price: "95.50".to_string(),
            currency_code: "EUR".to_string(),
            notes: Some("corrected".to_string()),
        }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    let text = text_of(&result);
    assert!(
        text.contains(&format!("Transaction {tx_id} updated.")),
        "{text}"
    );
    assert!(text.contains("qty=20"), "{text}");
}

#[tokio::test]
async fn update_transaction_tool_rejects_update_violating_position() {
    let pool = test_pool().await;
    let account_id = seed_account(&pool, "Broker").await;
    let asset_id = seed_asset(&pool, "VWCE").await;
    let tx_id = seed_opening_transaction(&pool, account_id, asset_id).await;
    let server = PortfolioServer::new(pool);

    // Quantity above the current position should be rejected on SELL.
    let result = server
        .update_transaction(Parameters(UpdateTransactionArgs {
            transaction_id: tx_id,
            account_id,
            asset_id,
            transaction_type: "SELL".to_string(),
            trade_date: "2026-01-03".to_string(),
            quantity: "999".to_string(),
            unit_price: "100.00".to_string(),
            currency_code: "EUR".to_string(),
            notes: None,
        }))
        .await;
    assert!(result.is_error.unwrap_or(false), "{}", text_of(&result));
}

#[tokio::test]
async fn delete_transaction_tool_removes_transaction() {
    let pool = test_pool().await;
    let account_id = seed_account(&pool, "Broker").await;
    let asset_id = seed_asset(&pool, "VWCE").await;
    let tx_id = seed_opening_transaction(&pool, account_id, asset_id).await;
    let server = PortfolioServer::new(pool.clone());

    let result = server
        .delete_transaction(Parameters(DeleteByIdArgs { id: tx_id }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    assert!(text_of(&result).contains(&format!("Transaction {tx_id} deleted.")));

    let listing = server
        .list_transactions(Parameters(LimitArgs { limit: None }))
        .await;
    assert_eq!(text_of(&listing), "No transactions found.");

    // Deleting again should fail (already gone).
    let result = server
        .delete_transaction(Parameters(DeleteByIdArgs { id: tx_id }))
        .await;
    assert!(result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn create_transfer_tool_moves_cash_between_accounts() {
    let pool = test_pool().await;
    let from = seed_account(&pool, "Bank").await;
    let to = seed_account(&pool, "Broker").await;
    let server = PortfolioServer::new(pool);

    let result = server
        .create_transfer(Parameters(CreateTransferArgs {
            from_account_id: from,
            to_account_id: to,
            from_currency: "EUR".to_string(),
            from_amount: "1000.00".to_string(),
            to_currency: "USD".to_string(),
            to_amount: "1080.50".to_string(),
            transfer_date: "2026-01-05".to_string(),
            notes: None,
        }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    assert!(text_of(&result).contains("Transfer created with id"));

    // Same-account transfer should be rejected.
    let result = server
        .create_transfer(Parameters(CreateTransferArgs {
            from_account_id: from,
            to_account_id: from,
            from_currency: "EUR".to_string(),
            from_amount: "10".to_string(),
            to_currency: "EUR".to_string(),
            to_amount: "10".to_string(),
            transfer_date: "2026-01-05".to_string(),
            notes: None,
        }))
        .await;
    assert!(result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn create_cash_movement_tool_records_deposit() {
    let pool = test_pool().await;
    let account_id = seed_account(&pool, "Bank").await;
    let server = PortfolioServer::new(pool);

    let result = server
        .create_cash_movement(Parameters(CreateCashMovementArgs {
            account_id,
            currency: "EUR".to_string(),
            amount: "500.00".to_string(),
            date: "2026-01-05".to_string(),
            notes: Some("salary".to_string()),
        }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    assert!(text_of(&result).contains("Cash movement recorded"));
}

#[tokio::test]
async fn create_cash_movement_tool_rejects_bad_date() {
    let pool = test_pool().await;
    let account_id = seed_account(&pool, "Bank").await;
    let server = PortfolioServer::new(pool);
    let result = server
        .create_cash_movement(Parameters(CreateCashMovementArgs {
            account_id,
            currency: "EUR".to_string(),
            amount: "100".to_string(),
            date: "yesterday".to_string(),
            notes: None,
        }))
        .await;
    assert!(result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn delete_account_tool_removes_empty_account_only() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool.clone());
    let account_id = seed_account(&pool, "Temp").await;

    let result = server
        .delete_account(Parameters(DeleteByIdArgs { id: account_id }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    assert!(text_of(&result).contains(&format!("Account {account_id} deleted.")));

    let listing = server.list_accounts().await;
    assert_eq!(text_of(&listing), "No accounts found.");
}

#[tokio::test]
async fn delete_account_tool_fails_when_ledger_entries_exist() {
    let pool = test_pool().await;
    let account_id = seed_account(&pool, "Broker").await;
    let server = PortfolioServer::new(pool.clone());
    server
        .create_cash_movement(Parameters(CreateCashMovementArgs {
            account_id,
            currency: "EUR".to_string(),
            amount: "100.00".to_string(),
            date: "2026-01-05".to_string(),
            notes: None,
        }))
        .await;

    let result = server
        .delete_account(Parameters(DeleteByIdArgs { id: account_id }))
        .await;
    assert!(result.is_error.unwrap_or(false), "{}", text_of(&result));
    assert!(text_of(&result).contains("cannot delete an account that has ledger entries"));
}

#[tokio::test]
async fn delete_asset_tool_removes_unused_asset_only() {
    let pool = test_pool().await;
    let server = PortfolioServer::new(pool.clone());
    let asset_id = seed_asset(&pool, "VWCE").await;

    let result = server
        .delete_asset(Parameters(DeleteByIdArgs { id: asset_id }))
        .await;
    assert!(!result.is_error.unwrap_or(false), "{}", text_of(&result));
    assert!(text_of(&result).contains(&format!("Asset {asset_id} deleted.")));

    let listing = server.list_assets().await;
    assert_eq!(text_of(&listing), "No assets found.");

    // Deleting an unknown asset should error, not silently succeed.
    let result = server
        .delete_asset(Parameters(DeleteByIdArgs { id: 999_999 }))
        .await;
    assert!(result.is_error.unwrap_or(false));
}

#[tokio::test]
async fn delete_asset_tool_fails_when_transactions_exist() {
    let pool = test_pool().await;
    let account_id = seed_account(&pool, "Broker").await;
    let asset_id = seed_asset(&pool, "VWCE").await;
    seed_opening_transaction(&pool, account_id, asset_id).await;
    let server = PortfolioServer::new(pool);

    let result = server
        .delete_asset(Parameters(DeleteByIdArgs { id: asset_id }))
        .await;
    assert!(result.is_error.unwrap_or(false), "{}", text_of(&result));
}
