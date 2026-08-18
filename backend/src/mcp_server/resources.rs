use rmcp::{
    ErrorData as McpError,
    model::{
        Annotated, ErrorCode, RawResource, RawResourceTemplate, ReadResourceResult,
        ResourceContents,
    },
};
use sqlx::SqlitePool;

use crate::{
    PRODUCT_BASE_CURRENCY,
    storage::{
        AccountId, AssetId, StorageError, get_account, get_asset, get_portfolio_summary,
        list_account_balances, list_account_positions, list_accounts, list_assets,
        list_portfolio_allocation, list_portfolio_snapshots, list_transfers_by_account,
    },
};

use super::PortfolioServer;

pub(super) const RESOURCE_MIME_TYPE: &str = "application/json";

impl PortfolioServer {
    /// Build the list of concrete resources. Extracted from `list_resources`
    /// so unit tests can call it without constructing a `RequestContext`
    /// (whose constructor is `pub(crate)` in rmcp).
    pub(super) async fn list_resources_inner(
        &self,
    ) -> Result<Vec<rmcp::model::Resource>, McpError> {
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
    pub(super) async fn read_resource_by_uri(
        &self,
        uri: &str,
    ) -> Result<ReadResourceResult, McpError> {
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

pub(super) fn build_resource_templates() -> Vec<rmcp::model::ResourceTemplate> {
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
pub(super) enum ResourceRef {
    Account(AccountId),
    Asset(AssetId),
    PortfolioSummary,
    PortfolioSnapshots,
    PortfolioAllocation,
}

pub(super) fn parse_resource_uri(uri: &str) -> Option<ResourceRef> {
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
