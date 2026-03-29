use std::collections::BTreeMap;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, PRODUCT_BASE_CURRENCY, compact_decimal_output, format_decimal_amount,
    get_portfolio_summary, list_accounts, list_assets, list_transactions, list_transfers,
};
use crate::storage::StorageError;

#[derive(Debug, Deserialize)]
pub struct AssistantChatRequest {
    #[serde(default)]
    pub messages: Vec<AssistantChatMessageRequest>,
}

#[derive(Debug, Deserialize)]
pub struct AssistantChatMessageRequest {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantChatResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantChatErrorResponse {
    error: String,
}

pub async fn chat(
    State(state): State<AppState>,
    Json(request): Json<AssistantChatRequest>,
) -> Result<Json<AssistantChatResponse>, (StatusCode, Json<AssistantChatErrorResponse>)> {
    let prompt = latest_user_prompt(&request).unwrap_or_default();

    match build_mock_assistant_reply(&state, prompt).await {
        Ok(message) => Ok(Json(AssistantChatResponse { message })),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AssistantChatErrorResponse {
                error: format!("failed to build assistant response: {error}"),
            }),
        )),
    }
}

fn latest_user_prompt(request: &AssistantChatRequest) -> Option<&str> {
    request
        .messages
        .iter()
        .rev()
        .find(|message| message.role.eq_ignore_ascii_case("user"))
        .map(|message| message.content.trim())
        .filter(|content| !content.is_empty())
}

async fn build_mock_assistant_reply(
    state: &AppState,
    prompt: &str,
) -> Result<String, StorageError> {
    let normalized_prompt = prompt.to_ascii_lowercase();
    let pool = &state.pool;

    let accounts = list_accounts(pool).await?;
    let assets = list_assets(pool).await?;
    let transactions = list_transactions(pool).await?;
    let transfers = list_transfers(pool).await?;
    let portfolio = get_portfolio_summary(pool, PRODUCT_BASE_CURRENCY).await?;

    let total_value = portfolio.total_value_amount.map(|amount| {
        format!(
            "{} {}",
            compact_decimal_output(&format_decimal_amount(amount.as_decimal())),
            portfolio.display_currency.as_str(),
        )
    });

    let account_names = preview_list(
        accounts
            .iter()
            .map(|account| account.name.as_str())
            .collect::<Vec<_>>(),
    );
    let asset_symbols = preview_list(
        assets
            .iter()
            .map(|asset| asset.symbol.as_str())
            .collect::<Vec<_>>(),
    );

    let mut account_type_counts = BTreeMap::new();
    for account in &accounts {
        *account_type_counts.entry(account.account_type.as_str()).or_insert(0usize) += 1;
    }

    let account_type_summary = if account_type_counts.is_empty() {
        "no accounts yet".to_string()
    } else {
        account_type_counts
            .into_iter()
            .map(|(account_type, count)| format!("{count} {account_type}"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    if prompt.is_empty() {
        return Ok(format!(
            "The backend assistant is connected. Right now I can see {} account(s), {} asset(s), {} transaction(s), and {} transfer(s). Ask about your portfolio, accounts, assets, transactions, or transfers.",
            accounts.len(),
            assets.len(),
            transactions.len(),
            transfers.len(),
        ));
    }

    if normalized_prompt.contains("portfolio") {
        let total_value_sentence = match total_value {
            Some(total_value) => format!("The current portfolio total is {total_value}."),
            None => "The portfolio total is currently unavailable because some conversions are missing.".to_string(),
        };

        let holdings_preview = preview_list(
            portfolio
                .holdings
                .iter()
                .map(|holding| holding.symbol.as_str())
                .collect::<Vec<_>>(),
        );

        return Ok(format!(
            "{total_value_sentence} I can see {} account(s) and {} asset(s). Top holdings right now: {}.",
            accounts.len(),
            assets.len(),
            holdings_preview,
        ));
    }

    if normalized_prompt.contains("account") {
        return Ok(format!(
            "You currently have {} account(s): {}. Breakdown by type: {}.",
            accounts.len(),
            account_names,
            account_type_summary,
        ));
    }

    if normalized_prompt.contains("asset") {
        return Ok(format!(
            "You currently track {} asset(s). Symbols in the current set include: {}.",
            assets.len(),
            asset_symbols,
        ));
    }

    if normalized_prompt.contains("transaction") {
        let latest_trade = transactions
            .first()
            .map(|transaction| transaction.trade_date.as_str().to_string())
            .unwrap_or_else(|| "no trade date yet".to_string());

        return Ok(format!(
            "There are {} transaction(s) recorded. The most recent trade date is {}.",
            transactions.len(),
            latest_trade,
        ));
    }

    if normalized_prompt.contains("transfer") {
        let latest_transfer = transfers
            .first()
            .map(|transfer| transfer.transfer_date.as_str().to_string())
            .unwrap_or_else(|| "no transfer date yet".to_string());

        return Ok(format!(
            "There are {} transfer(s) recorded. The most recent transfer date is {}.",
            transfers.len(),
            latest_transfer,
        ));
    }

    Ok(format!(
        "I can answer from the current backend data snapshot. Right now there are {} account(s), {} asset(s), {} transaction(s), and {} transfer(s). Try asking specifically about the portfolio, accounts, assets, transactions, or transfers.",
        accounts.len(),
        assets.len(),
        transactions.len(),
        transfers.len(),
    ))
}

fn preview_list(items: Vec<&str>) -> String {
    if items.is_empty() {
        return "none yet".to_string();
    }

    let total_items = items.len();
    let preview = items
        .into_iter()
        .take(3)
        .map(str::to_string)
        .collect::<Vec<_>>();

    let listed = preview.join(", ");

    if total_items > 3 {
        format!("{listed}, and more")
    } else {
        listed
    }
}
