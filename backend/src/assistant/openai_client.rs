use std::convert::Infallible;

use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use axum::response::sse::Event;

use crate::AppState;

use super::tool_executor::{execute_tool, tool_definitions};
use super::types::{AssistantChatMessageRequest, AssistantError};

// ── Constants ─────────────────────────────────────────────────────────────────

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const MAX_TOOL_ROUNDS: usize = 5;
const MAX_MESSAGES_SIZE_BYTES: usize = 256 * 1024;

pub const DEFAULT_SYSTEM_PROMPT: &str = "\
You are a helpful portfolio assistant for the Siniscalco app. \
The app tracks investment accounts, assets, transactions, and fund transfers. \
Use the available tools to look up live data before answering. \
Be concise and precise. Format monetary amounts with their currency code.";

pub const fn openai_chat_url() -> &'static str {
    OPENAI_CHAT_URL
}

// ── SSE helpers ───────────────────────────────────────────────────────────────

pub async fn send_sse_event(tx: &mpsc::Sender<Result<Event, Infallible>>, data: Value) {
    let event = Event::default().data(data.to_string());
    let _ = tx.send(Ok(event)).await;
}

// ── OpenAI streaming chat loop ────────────────────────────────────────────────

pub async fn openai_chat_streaming(
    state: &AppState,
    incoming: &[AssistantChatMessageRequest],
    api_key: &str,
    model: &str,
    selected_model: &str,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
) {
    let system_prompt = crate::storage::settings::get_app_setting(
        &state.pool,
        super::model_registry::SETTING_SYSTEM_PROMPT,
    )
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());

    let mut messages: Vec<Value> = vec![json!({ "role": "system", "content": system_prompt })];

    for msg in incoming {
        messages.push(json!({ "role": msg.role, "content": msg.content }));
    }

    for _ in 0..MAX_TOOL_ROUNDS {
        let messages_size: usize = messages.iter().map(|m| m.to_string().len()).sum();
        if messages_size > MAX_MESSAGES_SIZE_BYTES {
            warn!(
                messages_size,
                max = MAX_MESSAGES_SIZE_BYTES,
                "assistant message context exceeded maximum size"
            );
            send_sse_event(
                tx,
                json!({"type": "error", "error": "message context exceeded maximum size"}),
            )
            .await;
            return;
        }

        let all_tools: Vec<Value> = {
            let mut tools = tool_definitions()
                .as_array()
                .expect("tool_definitions returns an array")
                .clone();
            if let Some(mcp) = &state.mcp_client {
                tools.extend(mcp.tools.iter().cloned());
            }
            tools
        };

        let body = json!({
            "model": model,
            "messages": messages,
            "tools": all_tools,
            "tool_choice": "auto",
        });

        let response = match state
            .http_client
            .post(&state.openai_chat_url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                error!(error = %e, "OpenAI request failed");
                send_sse_event(tx, json!({"type": "error", "error": format!("failed to build assistant response: api error: {e}")})).await;
                return;
            }
        };

        let http_status = response.status();
        if !http_status.is_success() {
            let body = response.text().await.unwrap_or_default();
            warn!(
                http_status = %http_status,
                openai_response_body = %body,
                "OpenAI returned a non-2xx response"
            );
            send_sse_event(tx, json!({"type": "error", "error": format!("failed to build assistant response: api error: OpenAI {http_status}: {body}")})).await;
            return;
        }

        let data: Value = match response.json().await {
            Ok(v) => v,
            Err(e) => {
                error!(error = %e, "failed to parse OpenAI response JSON");
                send_sse_event(tx, json!({"type": "error", "error": format!("failed to build assistant response: api error: {e}")})).await;
                return;
            }
        };

        debug!(openai_response = %data, "received OpenAI response");

        let choice = &data["choices"][0];
        let finish_reason = choice["finish_reason"].as_str().unwrap_or("");
        let message = &choice["message"];

        info!(finish_reason, "OpenAI finish_reason");

        if finish_reason == "stop" {
            let text = extract_message_text(&message["content"]);
            send_sse_event(
                tx,
                json!({"type": "text", "text": text, "model": selected_model}),
            )
            .await;
            return;
        }

        if finish_reason == "tool_calls" {
            let normalized = match normalize_tool_call_message(message) {
                Ok(v) => v,
                Err(e) => {
                    error!(error = %e, "failed to normalize tool call message");
                    send_sse_event(tx, json!({"type": "error", "error": format!("failed to build assistant response: {e}")})).await;
                    return;
                }
            };
            messages.push(normalized);

            let tool_calls = match message["tool_calls"].as_array() {
                Some(v) => v.to_vec(),
                None => {
                    send_sse_event(tx, json!({"type": "error", "error": "failed to build assistant response: api error: missing tool_calls array"})).await;
                    return;
                }
            };

            for call in &tool_calls {
                let id = call["id"].as_str().unwrap_or("").to_string();
                let name = call["function"]["name"].as_str().unwrap_or("");
                let args: Value = call["function"]["arguments"]
                    .as_str()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(json!({}));

                info!(tool = %name, "executing tool call");
                send_sse_event(
                    tx,
                    json!({"type": "tool_call", "id": id, "name": name, "args": args}),
                )
                .await;

                let result = match execute_tool(
                    &state.pool,
                    state.mcp_client.as_deref(),
                    name,
                    &args,
                )
                .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        error!(tool = %name, error = %e, "tool execution failed");
                        send_sse_event(tx, json!({"type": "error", "error": format!("failed to build assistant response: {e}")})).await;
                        return;
                    }
                };

                debug!(tool = %name, result = %result, "tool result");
                send_sse_event(
                    tx,
                    json!({"type": "tool_result", "id": id, "result": result}),
                )
                .await;

                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": id,
                    "content": result.to_string(),
                }));
            }

            continue;
        }

        warn!(finish_reason, "unexpected OpenAI finish_reason");
        send_sse_event(
            tx,
            json!({"type": "error", "error": format!("failed to build assistant response: api error: unexpected finish_reason: {finish_reason}")}),
        )
        .await;
        return;
    }

    warn!(
        max_rounds = MAX_TOOL_ROUNDS,
        "tool call loop exceeded max iterations"
    );
    send_sse_event(
        tx,
        json!({"type": "error", "error": "failed to build assistant response: api error: tool call loop exceeded max iterations"}),
    )
    .await;
}

// ── Message utilities ─────────────────────────────────────────────────────────

pub fn extract_message_text(content: &Value) -> String {
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

fn normalize_tool_call_message(message: &Value) -> Result<Value, AssistantError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_message_text_from_plain_string() {
        let content = json!("hello world");
        assert_eq!(extract_message_text(&content), "hello world");
    }

    #[test]
    fn extract_message_text_from_content_array() {
        let content = json!([
            { "type": "text", "text": "hello " },
            { "type": "image", "url": "http://example.com/img.png" },
            { "type": "text", "text": "world" }
        ]);
        assert_eq!(extract_message_text(&content), "hello world");
    }

    #[test]
    fn extract_message_text_from_null_returns_empty() {
        assert_eq!(extract_message_text(&json!(null)), "");
    }

    #[test]
    fn normalize_tool_call_message_happy_path() {
        let message = json!({
            "content": null,
            "tool_calls": [{
                "id": "call_abc",
                "type": "function",
                "function": {
                    "name": "list_accounts",
                    "arguments": "{}"
                }
            }]
        });

        let result = normalize_tool_call_message(&message).unwrap();
        assert_eq!(result["role"], "assistant");
        assert_eq!(result["tool_calls"][0]["id"], "call_abc");
        assert_eq!(result["tool_calls"][0]["function"]["name"], "list_accounts");
    }

    #[test]
    fn normalize_tool_call_message_missing_tool_calls_errors() {
        let message = json!({ "content": "no tool_calls here" });
        let result = normalize_tool_call_message(&message);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("missing tool_calls")
        );
    }

    #[test]
    fn normalize_tool_call_message_missing_id_errors() {
        let message = json!({
            "content": null,
            "tool_calls": [{
                "type": "function",
                "function": { "name": "list_accounts", "arguments": "{}" }
            }]
        });
        let result = normalize_tool_call_message(&message);
        assert!(result.is_err());
    }
}
