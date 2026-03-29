use std::collections::BTreeMap;
use std::fmt;

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::storage::StorageError;
use crate::{
    AppState, PRODUCT_BASE_CURRENCY, compact_decimal_output, format_decimal_amount,
    get_portfolio_summary, list_accounts, list_assets, list_transactions, list_transfers,
};

// ── Request / response types ──────────────────────────────────────────────────

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

// ── Error type ────────────────────────────────────────────────────────────────

enum AssistantError {
    Storage(StorageError),
    Api(String),
}

impl AssistantError {
    fn status_code(&self) -> StatusCode {
        match self {
            AssistantError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AssistantError::Api(_) => StatusCode::BAD_GATEWAY,
        }
    }
}

impl fmt::Display for AssistantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssistantError::Storage(e) => write!(f, "storage error: {e}"),
            AssistantError::Api(msg) => write!(f, "api error: {msg}"),
        }
    }
}

impl From<StorageError> for AssistantError {
    fn from(e: StorageError) -> Self {
        AssistantError::Storage(e)
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn chat(
    State(state): State<AppState>,
    Json(request): Json<AssistantChatRequest>,
) -> Result<Json<AssistantChatResponse>, (StatusCode, Json<AssistantChatErrorResponse>)> {
    let openai_api_key = state
        .openai_api_key
        .as_deref()
        .map(str::trim)
        .filter(|api_key| !api_key.is_empty());

    let result = match openai_api_key {
        Some(api_key) => openai_chat(&state, &request.messages, api_key).await,
        None => {
            let prompt = latest_user_prompt(&request).unwrap_or_default();
            build_mock_reply(&state, prompt)
                .await
                .map_err(AssistantError::from)
        }
    };

    result
        .map(|message| Json(AssistantChatResponse { message }))
        .map_err(|error| {
            (
                error.status_code(),
                Json(AssistantChatErrorResponse {
                    error: format!("failed to build assistant response: {error}"),
                }),
            )
        })
}

// ── OpenAI tool-calling ───────────────────────────────────────────────────────

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const OPENAI_MODEL: &str = "gpt-4o-mini";
const MAX_TOOL_ROUNDS: usize = 5;

const SYSTEM_PROMPT: &str = "\
You are a helpful portfolio assistant for the Siniscalco app. \
The app tracks investment accounts, assets, transactions, and fund transfers. \
Use the available tools to look up live data before answering. \
Be concise and precise. Format monetary amounts with their currency code.";

pub const fn openai_chat_url() -> &'static str {
    OPENAI_CHAT_URL
}

fn tool_definitions() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "get_portfolio_summary",
                "description": "Returns the portfolio total value, all holdings with their values, \
                                account totals, and cash balances by currency.",
                "parameters": { "type": "object", "properties": {}, "required": [] }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_accounts",
                "description": "Returns all accounts with their name, type, and base currency.",
                "parameters": { "type": "object", "properties": {}, "required": [] }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_assets",
                "description": "Returns all tracked assets with their symbol, name, type, \
                                current price, and total quantity held.",
                "parameters": { "type": "object", "properties": {}, "required": [] }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_transactions",
                "description": "Returns all asset transactions (buys/sells) ordered by trade date descending.",
                "parameters": { "type": "object", "properties": {}, "required": [] }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_transfers",
                "description": "Returns all fund transfers between accounts ordered by date descending.",
                "parameters": { "type": "object", "properties": {}, "required": [] }
            }
        }
    ])
}

async fn execute_tool(pool: &sqlx::SqlitePool, name: &str) -> Result<Value, StorageError> {
    match name {
        "get_portfolio_summary" => {
            let portfolio = get_portfolio_summary(pool, PRODUCT_BASE_CURRENCY).await?;
            let currency = portfolio.display_currency.as_str();

            let total_value = portfolio.total_value_amount.map(|a| {
                format!(
                    "{} {}",
                    compact_decimal_output(&format_decimal_amount(a.as_decimal())),
                    currency,
                )
            });

            let holdings: Vec<Value> = portfolio
                .holdings
                .iter()
                .map(|h| {
                    let value = format!(
                        "{} {}",
                        compact_decimal_output(&format_decimal_amount(h.value.as_decimal())),
                        currency,
                    );
                    json!({ "symbol": h.symbol, "name": h.name, "value": value })
                })
                .collect();

            let account_totals: Vec<Value> = portfolio
                .account_totals
                .iter()
                .map(|a| {
                    let total = a.total_amount.map(|amt| {
                        format!(
                            "{} {}",
                            compact_decimal_output(&format_decimal_amount(amt.as_decimal())),
                            a.total_currency.as_str(),
                        )
                    });
                    json!({
                        "name": a.name.as_str(),
                        "type": a.account_type.as_str(),
                        "total": total,
                    })
                })
                .collect();

            let cash_by_currency: Vec<Value> = portfolio
                .cash_by_currency
                .iter()
                .map(|c| {
                    json!({
                        "currency": c.currency.as_str(),
                        "amount": compact_decimal_output(&format_decimal_amount(c.amount.as_decimal())),
                    })
                })
                .collect();

            Ok(json!({
                "total_value": total_value,
                "currency": currency,
                "holdings": holdings,
                "account_totals": account_totals,
                "cash_by_currency": cash_by_currency,
            }))
        }

        "list_accounts" => {
            let accounts = list_accounts(pool).await?;
            let items: Vec<Value> = accounts
                .iter()
                .map(|a| {
                    json!({
                        "name": a.name.as_str(),
                        "type": a.account_type.as_str(),
                        "base_currency": a.base_currency.as_str(),
                    })
                })
                .collect();
            Ok(json!({ "count": items.len(), "accounts": items }))
        }

        "list_assets" => {
            let assets = list_assets(pool).await?;
            let items: Vec<Value> = assets
                .iter()
                .map(|a| {
                    let price = a.current_price.as_ref().map(|p| {
                        format!(
                            "{} {}",
                            compact_decimal_output(&format_decimal_amount(p.as_decimal())),
                            a.current_price_currency.as_ref().map_or("", |c| c.as_str()),
                        )
                    });
                    let quantity = a
                        .total_quantity
                        .as_ref()
                        .map(|q| compact_decimal_output(&format_decimal_amount(q.as_decimal())));
                    json!({
                        "symbol": a.symbol.as_str(),
                        "name": a.name.as_str(),
                        "type": a.asset_type.as_str(),
                        "current_price": price,
                        "total_quantity": quantity,
                    })
                })
                .collect();
            Ok(json!({ "count": items.len(), "assets": items }))
        }

        "list_transactions" => {
            let transactions = list_transactions(pool).await?;
            let items: Vec<Value> = transactions
                .iter()
                .map(|t| {
                    json!({
                        "trade_date": t.trade_date.as_str(),
                        "type": t.transaction_type.as_str(),
                        "quantity": compact_decimal_output(&format_decimal_amount(t.quantity.as_decimal())),
                        "unit_price": format!(
                            "{} {}",
                            compact_decimal_output(&format_decimal_amount(t.unit_price.as_decimal())),
                            t.currency_code.as_str(),
                        ),
                        "notes": t.notes,
                    })
                })
                .collect();
            Ok(json!({ "count": items.len(), "transactions": items }))
        }

        "list_transfers" => {
            let transfers = list_transfers(pool).await?;
            let items: Vec<Value> = transfers
                .iter()
                .map(|t| {
                    json!({
                        "transfer_date": t.transfer_date.as_str(),
                        "from": format!(
                            "{} {}",
                            compact_decimal_output(&format_decimal_amount(t.from_amount.as_decimal())),
                            t.from_currency.as_str(),
                        ),
                        "to": format!(
                            "{} {}",
                            compact_decimal_output(&format_decimal_amount(t.to_amount.as_decimal())),
                            t.to_currency.as_str(),
                        ),
                        "notes": t.notes,
                    })
                })
                .collect();
            Ok(json!({ "count": items.len(), "transfers": items }))
        }

        _ => Ok(json!({ "error": format!("unknown tool: {name}") })),
    }
}

async fn openai_chat(
    state: &AppState,
    incoming: &[AssistantChatMessageRequest],
    api_key: &str,
) -> Result<String, AssistantError> {
    let mut messages: Vec<Value> = vec![json!({ "role": "system", "content": SYSTEM_PROMPT })];

    for msg in incoming {
        messages.push(json!({ "role": msg.role, "content": msg.content }));
    }

    for _ in 0..MAX_TOOL_ROUNDS {
        let body = json!({
            "model": OPENAI_MODEL,
            "messages": messages,
            "tools": tool_definitions(),
            "tool_choice": "auto",
        });

        let response = state
            .http_client
            .post(&state.openai_chat_url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AssistantError::Api(e.to_string()))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AssistantError::Api(format!("OpenAI error: {text}")));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| AssistantError::Api(e.to_string()))?;

        let choice = &data["choices"][0];
        let finish_reason = choice["finish_reason"].as_str().unwrap_or("");
        let message = &choice["message"];

        if finish_reason == "stop" {
            return Ok(extract_message_text(&message["content"]));
        }

        if finish_reason == "tool_calls" {
            messages.push(normalize_assistant_tool_call_message(message)?);

            let tool_calls = message["tool_calls"]
                .as_array()
                .ok_or_else(|| AssistantError::Api("missing tool_calls array".to_string()))?;

            for call in tool_calls {
                let id = call["id"].as_str().unwrap_or("").to_string();
                let name = call["function"]["name"].as_str().unwrap_or("");

                let result = execute_tool(&state.pool, name)
                    .await
                    .map_err(AssistantError::from)?;

                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": id,
                    "content": result.to_string(),
                }));
            }

            continue;
        }

        return Err(AssistantError::Api(format!(
            "unexpected finish_reason: {finish_reason}"
        )));
    }

    Err(AssistantError::Api(
        "tool call loop exceeded max iterations".to_string(),
    ))
}

fn extract_message_text(content: &Value) -> String {
    if let Some(text) = content.as_str() {
        return text.to_string();
    }

    content
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| {
            if part["type"].as_str() != Some("text") {
                return None;
            }

            part["text"].as_str().map(str::to_string)
        })
        .collect::<Vec<_>>()
        .join("")
}

fn normalize_assistant_tool_call_message(message: &Value) -> Result<Value, AssistantError> {
    let tool_calls = message["tool_calls"]
        .as_array()
        .ok_or_else(|| AssistantError::Api("missing tool_calls array".to_string()))?;

    let normalized_tool_calls = tool_calls
        .iter()
        .map(|call| {
            let id = call["id"]
                .as_str()
                .ok_or_else(|| AssistantError::Api("missing tool call id".to_string()))?;
            let tool_type = call["type"]
                .as_str()
                .ok_or_else(|| AssistantError::Api("missing tool call type".to_string()))?;
            let function_name = call["function"]["name"]
                .as_str()
                .ok_or_else(|| AssistantError::Api("missing tool function name".to_string()))?;
            let function_arguments = call["function"]["arguments"].as_str().ok_or_else(|| {
                AssistantError::Api("missing tool function arguments".to_string())
            })?;

            Ok(json!({
                "id": id,
                "type": tool_type,
                "function": {
                    "name": function_name,
                    "arguments": function_arguments,
                },
            }))
        })
        .collect::<Result<Vec<_>, AssistantError>>()?;

    Ok(json!({
        "role": "assistant",
        "content": message["content"],
        "tool_calls": normalized_tool_calls,
    }))
}

// ── Mock fallback (no API key) ────────────────────────────────────────────────

fn latest_user_prompt(request: &AssistantChatRequest) -> Option<&str> {
    request
        .messages
        .iter()
        .rev()
        .find(|message| message.role.eq_ignore_ascii_case("user"))
        .map(|message| message.content.trim())
        .filter(|content| !content.is_empty())
}

async fn build_mock_reply(state: &AppState, prompt: &str) -> Result<String, StorageError> {
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
        *account_type_counts
            .entry(account.account_type.as_str())
            .or_insert(0usize) += 1;
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
            None => {
                "The portfolio total is currently unavailable because some conversions are missing."
                    .to_string()
            }
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
