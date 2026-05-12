use rmcp::{
    ServerHandler,
    handler::server::router::{prompt::PromptRouter, tool::ToolRouter},
    model::{
        CallToolResult, Content, Implementation, InitializeResult, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    PRODUCT_BASE_CURRENCY, fmt_amount, fmt_opt_amount,
    storage::{StorageError, get_portfolio_summary, list_account_summaries, list_assets},
};

pub use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

// ── Tool argument types ───────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoArgs {}

// ── Server ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PortfolioServer {
    pool: SqlitePool,
    #[allow(dead_code)]
    tool_router: ToolRouter<PortfolioServer>,
    #[allow(dead_code)]
    prompt_router: PromptRouter<PortfolioServer>,
}

#[tool_router]
impl PortfolioServer {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            tool_router: Self::tool_router(),
            prompt_router: PromptRouter::default(),
        }
    }

    #[tool(
        description = "Get the overall portfolio summary including total value, 24h gain, and top holdings in the base currency (EUR)."
    )]
    async fn get_portfolio_summary(&self) -> CallToolResult {
        match get_portfolio_summary(&self.pool, PRODUCT_BASE_CURRENCY).await {
            Ok(summary) => {
                let total = fmt_opt_amount(summary.total_value_amount.as_ref());
                let gain_24h = fmt_opt_amount(summary.gain_24h_amount.as_ref());
                let total_gain = fmt_opt_amount(summary.total_gain_amount.as_ref());
                let currency = summary.display_currency.as_str().to_string();

                let mut lines = vec![
                    format!("Portfolio Summary ({currency})"),
                    format!(
                        "Total value: {}",
                        total.unwrap_or_else(|| "n/a".to_string())
                    ),
                    format!(
                        "24h gain:    {}",
                        gain_24h.unwrap_or_else(|| "n/a".to_string())
                    ),
                    format!(
                        "Total gain:  {}",
                        total_gain.unwrap_or_else(|| "n/a".to_string())
                    ),
                ];

                if !summary.holdings.is_empty() {
                    lines.push(String::new());
                    lines.push("Top holdings:".to_string());
                    for h in &summary.holdings {
                        let value = fmt_amount(&h.value);
                        lines.push(format!("  {} ({}) — {value} {currency}", h.name, h.symbol));
                    }
                }

                CallToolResult::success(vec![Content::text(lines.join("\n"))])
            }
            Err(e) => tool_error(e),
        }
    }

    #[tool(
        description = "List all tracked assets with symbol, name, current price, quantity, and total value in EUR."
    )]
    async fn list_assets(&self) -> CallToolResult {
        match list_assets(&self.pool).await {
            Ok(assets) => {
                if assets.is_empty() {
                    return CallToolResult::success(vec![Content::text("No assets found.")]);
                }

                let mut lines = vec![format!("Assets ({} total):", assets.len())];
                for asset in &assets {
                    let price = fmt_opt_amount(asset.current_price.as_ref())
                        .map(|p| {
                            let ccy = asset
                                .current_price_currency
                                .as_ref()
                                .map(|c| c.as_str().to_string())
                                .unwrap_or_default();
                            format!("{p} {ccy}")
                        })
                        .unwrap_or_else(|| "price n/a".to_string());
                    let qty = fmt_opt_amount(asset.total_quantity.as_ref())
                        .unwrap_or_else(|| "qty n/a".to_string());
                    lines.push(format!(
                        "  [{}] {} ({}) qty={qty} price={price}",
                        asset.id.as_i64(),
                        asset.name,
                        asset.symbol,
                    ));
                }

                CallToolResult::success(vec![Content::text(lines.join("\n"))])
            }
            Err(e) => tool_error(e),
        }
    }

    #[tool(
        description = "List all investment accounts with their type, base currency, cash total, and asset total."
    )]
    async fn list_accounts(&self) -> CallToolResult {
        match list_account_summaries(&self.pool).await {
            Ok(accounts) => {
                if accounts.is_empty() {
                    return CallToolResult::success(vec![Content::text("No accounts found.")]);
                }

                let mut lines = vec![format!("Accounts ({} total):", accounts.len())];
                for acc in &accounts {
                    let cash = fmt_opt_amount(acc.cash_total_amount.as_ref())
                        .unwrap_or_else(|| "n/a".to_string());
                    let assets_total = fmt_opt_amount(acc.asset_total_amount.as_ref())
                        .unwrap_or_else(|| "n/a".to_string());
                    let total = fmt_opt_amount(acc.total_amount.as_ref())
                        .unwrap_or_else(|| "n/a".to_string());
                    let ccy = acc
                        .total_currency
                        .as_ref()
                        .map(|c| c.as_str().to_string())
                        .unwrap_or_default();
                    lines.push(format!(
                        "  [{}] {} ({:?}, base={}) cash={cash} assets={assets_total} total={total} {ccy}",
                        acc.id.as_i64(),
                        acc.name,
                        acc.account_type,
                        acc.base_currency.as_str(),
                    ));
                }

                CallToolResult::success(vec![Content::text(lines.join("\n"))])
            }
            Err(e) => tool_error(e),
        }
    }
}

#[tool_handler]
impl ServerHandler for PortfolioServer {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "siniscalco-portfolio",
                option_env!("GIT_VERSION").unwrap_or("unknown"),
            ))
            .with_instructions(
                "Portfolio server: use get_portfolio_summary for an overview, \
                 list_assets for individual holdings, list_accounts for account details.",
            )
    }
}

// ── Service factory ───────────────────────────────────────────────────────────

pub fn build_mcp_service(
    pool: SqlitePool,
) -> StreamableHttpService<PortfolioServer, LocalSessionManager> {
    StreamableHttpService::new(
        move || Ok(PortfolioServer::new(pool.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tool_error(err: StorageError) -> CallToolResult {
    CallToolResult::error(vec![Content::text(format!("Error: {err}"))])
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    use super::*;
    use crate::storage::{AccountName, AccountType, CreateAccountInput, Currency};
    use crate::{init_db, storage::create_account};

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

    #[tokio::test]
    async fn list_tools_returns_three_tools() {
        let pool = test_pool().await;
        let server = PortfolioServer::new(pool);
        let tools = server.tool_router.list_all();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"get_portfolio_summary"), "{names:?}");
        assert!(names.contains(&"list_assets"), "{names:?}");
        assert!(names.contains(&"list_accounts"), "{names:?}");
        assert_eq!(tools.len(), 3);
    }

    #[tokio::test]
    async fn get_portfolio_summary_empty_db() {
        let pool = test_pool().await;
        let server = PortfolioServer::new(pool);
        let result = server.get_portfolio_summary().await;
        assert!(!result.is_error.unwrap_or(false));
        let text = &result.content[0].as_text().expect("text content").text;
        assert!(text.contains("Portfolio Summary"), "{text}");
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
}
