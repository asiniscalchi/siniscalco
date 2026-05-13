use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        prompt::PromptContext,
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::{
        Annotated, CallToolResult, Content, ErrorCode, GetPromptRequestParams, GetPromptResult,
        Implementation, InitializeResult, ListPromptsResult, ListResourceTemplatesResult,
        ListResourcesResult, PaginatedRequestParams, PromptMessage, PromptMessageRole, RawResource,
        RawResourceTemplate, ReadResourceRequestParams, ReadResourceResult, ResourceContents,
        ServerCapabilities, ServerInfo,
    },
    prompt, prompt_router,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    PRODUCT_BASE_CURRENCY, fmt_amount, fmt_opt_amount,
    storage::{
        AccountId, AssetId, StorageError, get_account, get_asset, get_portfolio_summary,
        list_account_balances, list_account_positions, list_account_summaries, list_accounts,
        list_assets, list_portfolio_allocation, list_portfolio_snapshots, list_transactions,
        list_transfers_by_account,
    },
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
            build_resource_templates(),
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

impl PortfolioServer {
    /// Build the list of concrete resources. Extracted from `list_resources`
    /// so unit tests can call it without constructing a `RequestContext`
    /// (whose constructor is `pub(crate)` in rmcp).
    async fn list_resources_inner(&self) -> Result<Vec<rmcp::model::Resource>, McpError> {
        let mut resources = Vec::new();

        for account in list_accounts(&self.pool).await.map_err(storage_to_mcp)? {
            resources.push(Annotated::new(
                RawResource::new(
                    format!("account://{}", account.id.as_i64()),
                    account.name.as_str().to_string(),
                )
                .with_title(format!(
                    "Account [{}]: {}",
                    account.id.as_i64(),
                    account.name.as_str()
                ))
                .with_mime_type(RESOURCE_MIME_TYPE),
                None,
            ));
        }

        for asset in list_assets(&self.pool).await.map_err(storage_to_mcp)? {
            resources.push(Annotated::new(
                RawResource::new(
                    format!("asset://{}", asset.id.as_i64()),
                    asset.symbol.as_str().to_string(),
                )
                .with_title(format!(
                    "Asset [{}]: {} ({})",
                    asset.id.as_i64(),
                    asset.name.as_str(),
                    asset.symbol.as_str()
                ))
                .with_mime_type(RESOURCE_MIME_TYPE),
                None,
            ));
        }

        for (uri, name, title) in [
            (
                "portfolio://summary",
                "portfolio_summary",
                "Portfolio summary",
            ),
            (
                "portfolio://snapshots",
                "portfolio_snapshots",
                "Portfolio snapshots time series",
            ),
            (
                "portfolio://allocation",
                "portfolio_allocation",
                "Portfolio allocation by asset class",
            ),
        ] {
            resources.push(Annotated::new(
                RawResource::new(uri, name)
                    .with_title(title)
                    .with_mime_type(RESOURCE_MIME_TYPE),
                None,
            ));
        }

        Ok(resources)
    }

    /// Read a resource by URI. Extracted from `read_resource` so it can be
    /// unit-tested without constructing a `RequestContext` (whose constructor
    /// is `pub(crate)` in rmcp).
    async fn read_resource_by_uri(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        let parsed = parse_resource_uri(uri)
            .ok_or_else(|| McpError::invalid_params(format!("unknown uri: {uri}"), None))?;
        let payload = match parsed {
            ResourceRef::Account(id) => read_account_resource(&self.pool, id).await?,
            ResourceRef::Asset(id) => read_asset_resource(&self.pool, id).await?,
            ResourceRef::PortfolioSummary => read_portfolio_summary_resource(&self.pool).await?,
            ResourceRef::PortfolioSnapshots => {
                read_portfolio_snapshots_resource(&self.pool).await?
            }
            ResourceRef::PortfolioAllocation => {
                read_portfolio_allocation_resource(&self.pool).await?
            }
        };
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(payload, uri.to_string()).with_mime_type(RESOURCE_MIME_TYPE),
        ]))
    }
}

// ── Resource URIs ─────────────────────────────────────────────────────────────

const RESOURCE_MIME_TYPE: &str = "application/json";

fn build_resource_templates() -> Vec<rmcp::model::ResourceTemplate> {
    vec![
        Annotated::new(
            RawResourceTemplate::new("account://{id}", "account")
                .with_title("Account by id")
                .with_description(
                    "Account details (cash balances, positions, transfers) by numeric id \
                     (e.g. account://1).",
                )
                .with_mime_type(RESOURCE_MIME_TYPE),
            None,
        ),
        Annotated::new(
            RawResourceTemplate::new("asset://{id}", "asset")
                .with_title("Asset by id")
                .with_description(
                    "Asset details (price, quantity, cost basis, ISIN) by numeric id \
                     (e.g. asset://1).",
                )
                .with_mime_type(RESOURCE_MIME_TYPE),
            None,
        ),
    ]
}

#[derive(Debug, PartialEq, Eq)]
enum ResourceRef {
    Account(AccountId),
    Asset(AssetId),
    PortfolioSummary,
    PortfolioSnapshots,
    PortfolioAllocation,
}

fn parse_resource_uri(uri: &str) -> Option<ResourceRef> {
    if let Some(rest) = uri.strip_prefix("account://") {
        return rest
            .parse::<i64>()
            .ok()
            .and_then(|n| AccountId::try_from(n).ok())
            .map(ResourceRef::Account);
    }
    if let Some(rest) = uri.strip_prefix("asset://") {
        return rest
            .parse::<i64>()
            .ok()
            .and_then(|n| AssetId::try_from(n).ok())
            .map(ResourceRef::Asset);
    }
    match uri {
        "portfolio://summary" => Some(ResourceRef::PortfolioSummary),
        "portfolio://snapshots" => Some(ResourceRef::PortfolioSnapshots),
        "portfolio://allocation" => Some(ResourceRef::PortfolioAllocation),
        _ => None,
    }
}

// ── Resource readers ──────────────────────────────────────────────────────────

async fn read_account_resource(pool: &SqlitePool, id: AccountId) -> Result<String, McpError> {
    let account = get_account(pool, id).await.map_err(storage_to_mcp)?;
    let balances = list_account_balances(pool, id)
        .await
        .map_err(storage_to_mcp)?;
    let positions = list_account_positions(pool, id)
        .await
        .map_err(storage_to_mcp)?;
    let transfers = list_transfers_by_account(pool, id)
        .await
        .map_err(storage_to_mcp)?;

    let balances_json: Vec<serde_json::Value> = balances
        .into_iter()
        .map(|b| {
            serde_json::json!({
                "currency": b.currency.as_str(),
                "amount": b.amount.to_string(),
                "updated_at": b.updated_at,
            })
        })
        .collect();

    let positions_json: Vec<serde_json::Value> = positions
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "asset_id": p.asset_id.as_i64(),
                "quantity": p.quantity.to_string(),
            })
        })
        .collect();

    let transfers_json: Vec<serde_json::Value> = transfers
        .into_iter()
        .map(|t| {
            let direction = if t.from_account_id == id { "out" } else { "in" };
            serde_json::json!({
                "id": t.id.as_i64(),
                "direction": direction,
                "from_account_id": t.from_account_id.as_i64(),
                "to_account_id": t.to_account_id.as_i64(),
                "from_currency": t.from_currency.as_str(),
                "from_amount": t.from_amount.to_string(),
                "to_currency": t.to_currency.as_str(),
                "to_amount": t.to_amount.to_string(),
                "transfer_date": t.transfer_date.as_str(),
                "notes": t.notes,
            })
        })
        .collect();

    let payload = serde_json::json!({
        "id": account.id.as_i64(),
        "name": account.name.as_str(),
        "account_type": account.account_type.as_str(),
        "base_currency": account.base_currency.as_str(),
        "created_at": account.created_at,
        "balances": balances_json,
        "positions": positions_json,
        "transfers": transfers_json,
    });
    Ok(payload.to_string())
}

async fn read_asset_resource(pool: &SqlitePool, id: AssetId) -> Result<String, McpError> {
    let asset = get_asset(pool, id).await.map_err(storage_to_mcp)?;
    let payload = serde_json::json!({
        "id": asset.id.as_i64(),
        "symbol": asset.symbol.as_str(),
        "name": asset.name.as_str(),
        "asset_type": asset.asset_type.as_str(),
        "isin": asset.isin,
        "current_price": asset.current_price.map(|p| p.to_string()),
        "current_price_currency": asset.current_price_currency.map(|c| c.as_str().to_string()),
        "current_price_as_of": asset.current_price_as_of,
        "previous_close": asset.previous_close.map(|p| p.to_string()),
        "previous_close_currency": asset.previous_close_currency.map(|c| c.as_str().to_string()),
        "total_quantity": asset.total_quantity.map(|q| q.to_string()),
        "avg_cost_basis": asset.avg_cost_basis.map(|p| p.to_string()),
        "avg_cost_basis_currency": asset.avg_cost_basis_currency.map(|c| c.as_str().to_string()),
        "quote_source_provider": asset.quote_source_provider,
        "quote_source_symbol": asset.quote_source_symbol,
        "quote_source_last_success_at": asset.quote_source_last_success_at,
        "created_at": asset.created_at,
        "updated_at": asset.updated_at,
    });
    Ok(payload.to_string())
}

async fn read_portfolio_summary_resource(pool: &SqlitePool) -> Result<String, McpError> {
    let summary = get_portfolio_summary(pool, PRODUCT_BASE_CURRENCY)
        .await
        .map_err(storage_to_mcp)?;

    let account_totals: Vec<serde_json::Value> = summary
        .account_totals
        .into_iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id.as_i64(),
                "name": a.name.as_str(),
                "account_type": a.account_type.as_str(),
                "cash_total_amount": a.cash_total_amount.map(|x| x.to_string()),
                "asset_total_amount": a.asset_total_amount.map(|x| x.to_string()),
                "total_amount": a.total_amount.map(|x| x.to_string()),
                "total_currency": a.total_currency.as_str(),
            })
        })
        .collect();

    let cash_by_currency: Vec<serde_json::Value> = summary
        .cash_by_currency
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "currency": c.currency.as_str(),
                "amount": c.amount.to_string(),
                "converted_amount": c.converted_amount.map(|x| x.to_string()),
            })
        })
        .collect();

    let allocation_totals: Vec<serde_json::Value> = summary
        .allocation_totals
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "label": s.label,
                "amount": s.amount.to_string(),
            })
        })
        .collect();

    let holdings: Vec<serde_json::Value> = summary
        .holdings
        .into_iter()
        .map(|h| {
            serde_json::json!({
                "asset_id": h.asset_id.map(|id| id.as_i64()),
                "symbol": h.symbol,
                "name": h.name,
                "value": h.value.to_string(),
                "gain_24h_amount": h.gain_24h_amount.map(|x| x.to_string()),
            })
        })
        .collect();

    let payload = serde_json::json!({
        "display_currency": summary.display_currency.as_str(),
        "total_value_amount": summary.total_value_amount.map(|x| x.to_string()),
        "gain_24h_amount": summary.gain_24h_amount.map(|x| x.to_string()),
        "total_gain_amount": summary.total_gain_amount.map(|x| x.to_string()),
        "fx_last_updated": summary.fx_last_updated,
        "allocation_is_partial": summary.allocation_is_partial,
        "holdings_is_partial": summary.holdings_is_partial,
        "account_totals": account_totals,
        "cash_by_currency": cash_by_currency,
        "allocation_totals": allocation_totals,
        "holdings": holdings,
    });
    Ok(payload.to_string())
}

async fn read_portfolio_snapshots_resource(pool: &SqlitePool) -> Result<String, McpError> {
    let snapshots = list_portfolio_snapshots(pool, PRODUCT_BASE_CURRENCY)
        .await
        .map_err(storage_to_mcp)?;
    let items: Vec<serde_json::Value> = snapshots
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "recorded_at": s.recorded_at,
                "currency": s.currency.as_str(),
                "total_value": s.total_value.to_string(),
            })
        })
        .collect();
    let payload = serde_json::json!({
        "currency": PRODUCT_BASE_CURRENCY.as_str(),
        "snapshots": items,
    });
    Ok(payload.to_string())
}

async fn read_portfolio_allocation_resource(pool: &SqlitePool) -> Result<String, McpError> {
    let (slices, is_partial) = list_portfolio_allocation(pool, PRODUCT_BASE_CURRENCY)
        .await
        .map_err(storage_to_mcp)?;
    let total: rust_decimal::Decimal = slices.iter().map(|s| s.amount.as_decimal()).sum();
    let items: Vec<serde_json::Value> = slices
        .into_iter()
        .map(|s| {
            let weight = if total.is_zero() {
                rust_decimal::Decimal::ZERO
            } else {
                (s.amount.as_decimal() / total * rust_decimal::Decimal::ONE_HUNDRED).round_dp(1)
            };
            serde_json::json!({
                "label": s.label,
                "amount": s.amount.to_string(),
                "weight_pct": weight.to_string(),
            })
        })
        .collect();
    let payload = serde_json::json!({
        "currency": PRODUCT_BASE_CURRENCY.as_str(),
        "is_partial": is_partial,
        "slices": items,
    });
    Ok(payload.to_string())
}

fn storage_to_mcp(err: StorageError) -> McpError {
    tracing::error!(error = %err, "MCP resource read failed");
    match err {
        StorageError::Validation(msg) => McpError::invalid_params(msg.to_string(), None),
        StorageError::Database(sqlx::Error::RowNotFound) => {
            McpError::new(ErrorCode::INVALID_PARAMS, "resource not found", None)
        }
        other => McpError::internal_error(format!("storage error: {other}"), None),
    }
}

// ── Service factory ───────────────────────────────────────────────────────────

pub fn build_mcp_service(
    pool: SqlitePool,
) -> StreamableHttpService<PortfolioServer, LocalSessionManager> {
    StreamableHttpService::new(
        move || Ok(PortfolioServer::new(pool.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().disable_allowed_hosts(),
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tool_error(err: StorageError) -> CallToolResult {
    tracing::error!(error = %err, "MCP tool error");
    CallToolResult::error(vec![Content::text(format!("Error: {err}"))])
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    use rmcp::handler::server::wrapper::Parameters;

    use super::*;
    use crate::storage::{
        AccountName, AccountType, AssetName, AssetSymbol, AssetType, CreateAccountInput,
        CreateAssetInput, Currency,
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

    #[tokio::test]
    async fn list_tools_returns_remaining_tool_set() {
        let pool = test_pool().await;
        let server = PortfolioServer::new(pool);
        let tools = server.tool_router.list_all();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        names.sort();
        assert_eq!(
            names,
            vec!["list_accounts", "list_assets", "list_transactions"]
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
}
