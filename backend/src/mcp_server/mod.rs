mod resources;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        prompt::PromptContext,
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::{
        CallToolResult, Content, GetPromptRequestParams, GetPromptResult, Implementation,
        InitializeResult, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult,
        PaginatedRequestParams, PromptMessage, PromptMessageRole, ReadResourceRequestParams,
        ReadResourceResult, ServerCapabilities, ServerInfo,
    },
    prompt, prompt_router,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    fmt_amount, fmt_opt_amount,
    storage::{StorageError, list_account_summaries, list_assets, list_transactions},
};

pub use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

// ── Tool argument types ───────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoArgs {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LimitArgs {
    /// Maximum number of rows to return (default 50, max 200).
    limit: Option<u32>,
}

// ── Prompt argument types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AccountReviewArgs {
    /// Numeric account ID as returned by list_accounts.
    pub account_id: i64,
}

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
            prompt_router: Self::prompt_router(),
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

    #[tool(
        description = "List recent asset transactions (buys, sells, dividends) across all accounts, newest first. Accepts an optional limit (default 50, max 200)."
    )]
    async fn list_transactions(&self, Parameters(args): Parameters<LimitArgs>) -> CallToolResult {
        let limit = args.limit.unwrap_or(50).min(200) as usize;

        match list_transactions(&self.pool).await {
            Ok(transactions) => {
                if transactions.is_empty() {
                    return CallToolResult::success(vec![Content::text("No transactions found.")]);
                }

                let shown = transactions.iter().take(limit);
                let mut lines = vec![format!(
                    "Transactions (showing up to {limit} of {}):",
                    transactions.len()
                )];
                for t in shown {
                    lines.push(format!(
                        "  [{}] {} {} qty={} price={} {} (account={} asset={})",
                        t.id,
                        t.trade_date.as_str(),
                        t.transaction_type.as_str(),
                        fmt_amount(&t.quantity),
                        fmt_amount(&t.unit_price),
                        t.currency_code.as_str(),
                        t.account_id.as_i64(),
                        t.asset_id.as_i64(),
                    ));
                }

                CallToolResult::success(vec![Content::text(lines.join("\n"))])
            }
            Err(e) => tool_error(e),
        }
    }
}

#[prompt_router]
impl PortfolioServer {
    #[prompt(
        name = "portfolio_recap",
        description = "Write a concise recap of the user's portfolio anchored on the current summary and allocation."
    )]
    async fn portfolio_recap_prompt(&self) -> Vec<PromptMessage> {
        let text = "Write a concise recap of the user's portfolio.\n\
                    \n\
                    1. Read `portfolio://summary` to get the total value, 24h gain, and top \
                    holdings.\n\
                    2. Read `portfolio://allocation` to get the breakdown by asset class.\n\
                    3. Produce 4-6 sentences covering total value, biggest holdings, dominant \
                    allocation, and any 24h movement worth flagging."
            .to_string();
        vec![PromptMessage::new_text(PromptMessageRole::User, text)]
    }

    #[prompt(
        name = "account_review",
        description = "Review a single investment account: cash balances, positions, and recent activity."
    )]
    async fn account_review_prompt(
        &self,
        Parameters(args): Parameters<AccountReviewArgs>,
    ) -> Vec<PromptMessage> {
        let id = args.account_id;
        let text = format!(
            "Review account [{id}].\n\
             \n\
             1. Read `account://{id}` for the account's cash balances, current positions, and \
             transfer history.\n\
             2. Call `list_transactions` (default limit) and filter to entries with \
             account={id}.\n\
             3. Produce a short report: cash by currency, the largest positions, and anything \
             unusual in recent transactions or transfers."
        );
        vec![PromptMessage::new_text(PromptMessageRole::User, text)]
    }

    #[prompt(
        name = "allocation_drift_check",
        description = "Surface concentration risk or imbalances in the current asset-class allocation."
    )]
    async fn allocation_drift_check_prompt(&self) -> Vec<PromptMessage> {
        let text = "Inspect the user's current asset-class allocation for concentration risk.\n\
                    \n\
                    1. Read `portfolio://allocation` for the breakdown by asset class with \
                    weights.\n\
                    2. Flag any single class above 70% or below 5% relative to a typical \
                    diversified portfolio, and note whether the allocation is marked partial.\n\
                    3. Suggest one or two concrete rebalance moves if drift is significant; \
                    otherwise confirm the allocation looks reasonable."
            .to_string();
        vec![PromptMessage::new_text(PromptMessageRole::User, text)]
    }
}

#[tool_handler]
impl ServerHandler for PortfolioServer {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new(
            "siniscalco-portfolio",
            option_env!("GIT_VERSION").unwrap_or("unknown"),
        ))
        .with_instructions(
            "Portfolio server tools, resources, and prompts. \
             Tools (formatted text): list_accounts — all accounts with totals; \
             list_assets — all tracked assets with price and quantity; \
             list_transactions(limit?) — recent buy/sell/dividend records. \
             Resources (JSON): account://{id} — account cash/positions/transfers; \
             asset://{id} — single asset with price, cost basis, ISIN; \
             portfolio://summary — overall value, 24h gain, holdings; \
             portfolio://snapshots — daily portfolio value time series; \
             portfolio://allocation — breakdown by asset class with weights. \
             Prompts: portfolio_recap, account_review(account_id), allocation_drift_check.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = self.list_resources_inner().await?;
        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult::with_all_items(
            resources::build_resource_templates(),
        ))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        self.read_resource_by_uri(&request.uri).await
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            prompts: self.prompt_router.list_all(),
            meta: None,
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let prompt_context = PromptContext::new(self, request.name, request.arguments, context);
        self.prompt_router.get_prompt(prompt_context).await
    }
}

// ── Service factory ───────────────────────────────────────────────────────────

pub fn build_mcp_service(
    pool: SqlitePool,
) -> StreamableHttpService<PortfolioServer, LocalSessionManager> {
    StreamableHttpService::new(
        move || Ok(PortfolioServer::new(pool.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .disable_allowed_hosts()
            .with_stateful_mode(false),
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tool_error(err: StorageError) -> CallToolResult {
    tracing::error!(error = %err, "MCP tool error");
    CallToolResult::error(vec![Content::text(format!("Error: {err}"))])
}

#[cfg(test)]
mod tests;
