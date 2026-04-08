use std::collections::BTreeMap;

use crate::storage::StorageError;
use crate::{
    AppState, PRODUCT_BASE_CURRENCY, compact_decimal_output, format_decimal_amount,
    get_portfolio_summary, list_accounts, list_assets, list_transactions, list_transfers,
};

use super::types::AssistantChatMessageRequest;

pub fn latest_user_prompt_from(messages: &[AssistantChatMessageRequest]) -> Option<&str> {
    messages
        .iter()
        .rev()
        .find(|message| message.role.eq_ignore_ascii_case("user"))
        .and_then(|message| message.content.as_str())
        .map(str::trim)
        .filter(|content| !content.is_empty())
}

pub async fn build_mock_reply(state: &AppState, prompt: &str) -> Result<String, StorageError> {
    let normalized_prompt = prompt.to_ascii_lowercase();
    let pool = &state.pool;

    if prompt.is_empty() {
        let (accounts, assets, transactions, transfers) = tokio::try_join!(
            list_accounts(pool),
            list_assets(pool),
            list_transactions(pool),
            list_transfers(pool),
        )?;
        return Ok(format!(
            "The backend assistant is connected. Right now I can see {} account(s), {} asset(s), {} transaction(s), and {} transfer(s). Ask about your portfolio, accounts, assets, transactions, or transfers.",
            accounts.len(),
            assets.len(),
            transactions.len(),
            transfers.len(),
        ));
    }

    if normalized_prompt.contains("portfolio") {
        let (portfolio, accounts, assets) = tokio::try_join!(
            get_portfolio_summary(pool, PRODUCT_BASE_CURRENCY),
            list_accounts(pool),
            list_assets(pool),
        )?;
        let total_value_sentence = match portfolio.total_value_amount.map(|amount| {
            format!(
                "{} {}",
                compact_decimal_output(&format_decimal_amount(amount.as_decimal())),
                portfolio.display_currency.as_str(),
            )
        }) {
            Some(v) => format!("The current portfolio total is {v}."),
            None => {
                "The portfolio total is currently unavailable because some conversions are missing."
                    .to_string()
            }
        };
        let holdings_preview = preview_list(
            portfolio
                .holdings
                .iter()
                .map(|h| h.symbol.as_str())
                .collect(),
        );
        return Ok(format!(
            "{total_value_sentence} I can see {} account(s) and {} asset(s). Top holdings right now: {}.",
            accounts.len(),
            assets.len(),
            holdings_preview,
        ));
    }

    if normalized_prompt.contains("account") {
        let accounts = list_accounts(pool).await?;
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
                .map(|(t, c)| format!("{c} {t}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let account_names = preview_list(accounts.iter().map(|a| a.name.as_str()).collect());
        return Ok(format!(
            "You currently have {} account(s): {}. Breakdown by type: {}.",
            accounts.len(),
            account_names,
            account_type_summary,
        ));
    }

    if normalized_prompt.contains("asset") {
        let assets = list_assets(pool).await?;
        let asset_symbols = preview_list(assets.iter().map(|a| a.symbol.as_str()).collect());
        return Ok(format!(
            "You currently track {} asset(s). Symbols in the current set include: {}.",
            assets.len(),
            asset_symbols,
        ));
    }

    if normalized_prompt.contains("transaction") {
        let transactions = list_transactions(pool).await?;
        let latest_trade = transactions
            .first()
            .map(|t| t.trade_date.as_str().to_string())
            .unwrap_or_else(|| "no trade date yet".to_string());
        return Ok(format!(
            "There are {} transaction(s) recorded. The most recent trade date is {}.",
            transactions.len(),
            latest_trade,
        ));
    }

    if normalized_prompt.contains("transfer") {
        let transfers = list_transfers(pool).await?;
        let latest_transfer = transfers
            .first()
            .map(|t| t.transfer_date.as_str().to_string())
            .unwrap_or_else(|| "no transfer date yet".to_string());
        return Ok(format!(
            "There are {} transfer(s) recorded. The most recent transfer date is {}.",
            transfers.len(),
            latest_transfer,
        ));
    }

    let (accounts, assets, transactions, transfers) = tokio::try_join!(
        list_accounts(pool),
        list_assets(pool),
        list_transactions(pool),
        list_transfers(pool),
    )?;
    Ok(format!(
        "I can answer from the current backend data snapshot. Right now there are {} account(s), {} asset(s), {} transaction(s), and {} transfer(s). Try asking specifically about the portfolio, accounts, assets, transactions, or transfers.",
        accounts.len(),
        assets.len(),
        transactions.len(),
        transfers.len(),
    ))
}

pub fn preview_list(items: Vec<&str>) -> String {
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn preview_list_empty() {
        assert_eq!(preview_list(vec![]), "none yet");
    }

    #[test]
    fn preview_list_one() {
        assert_eq!(preview_list(vec!["A"]), "A");
    }

    #[test]
    fn preview_list_three() {
        assert_eq!(preview_list(vec!["A", "B", "C"]), "A, B, C");
    }

    #[test]
    fn preview_list_more_than_three() {
        assert_eq!(preview_list(vec!["A", "B", "C", "D"]), "A, B, C, and more");
    }

    #[test]
    fn latest_user_prompt_finds_last_user_message() {
        let messages = vec![
            AssistantChatMessageRequest {
                role: "user".to_string(),
                content: json!("first"),
                tool_calls: None,
                tool_call_id: None,
            },
            AssistantChatMessageRequest {
                role: "assistant".to_string(),
                content: json!("response"),
                tool_calls: None,
                tool_call_id: None,
            },
            AssistantChatMessageRequest {
                role: "user".to_string(),
                content: json!("  second  "),
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        assert_eq!(latest_user_prompt_from(&messages), Some("second"));
    }

    #[test]
    fn latest_user_prompt_returns_none_when_last_message_is_blank() {
        // The function finds the last user message and returns None if it is blank.
        // It does not fall back to earlier user messages.
        let messages = vec![
            AssistantChatMessageRequest {
                role: "user".to_string(),
                content: json!("first"),
                tool_calls: None,
                tool_call_id: None,
            },
            AssistantChatMessageRequest {
                role: "user".to_string(),
                content: json!("   "),
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        assert_eq!(latest_user_prompt_from(&messages), None);
    }

    #[test]
    fn latest_user_prompt_case_insensitive_role() {
        let messages = vec![AssistantChatMessageRequest {
            role: "USER".to_string(),
            content: json!("hello"),
            tool_calls: None,
            tool_call_id: None,
        }];
        assert_eq!(latest_user_prompt_from(&messages), Some("hello"));
    }

    #[test]
    fn latest_user_prompt_no_user_messages_returns_none() {
        let messages = vec![AssistantChatMessageRequest {
            role: "assistant".to_string(),
            content: json!("hello"),
            tool_calls: None,
            tool_call_id: None,
        }];
        assert_eq!(latest_user_prompt_from(&messages), None);
    }
}
