use std::{convert::Infallible, time::Duration};

use futures_util::StreamExt;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use axum::response::sse::Event;

use crate::AppState;

use super::tool_executor::{execute_tool, tool_definitions};
use super::types::AssistantChatMessageRequest;

// ── Constants ─────────────────────────────────────────────────────────────────

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const MAX_TOOL_ROUNDS: usize = 5;
const MAX_MESSAGES_SIZE_BYTES: usize = 256 * 1024;
const STREAM_CHUNK_TIMEOUT: Duration = Duration::from_secs(30);

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

// ── Streaming accumulator ─────────────────────────────────────────────────────

#[derive(Default)]
struct AccumulatedToolCall {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Debug)]
enum StreamChunk {
    Done,
    Delta {
        content: Option<String>,
        tool_calls: Option<Vec<Value>>,
        finish_reason: Option<String>,
    },
}

fn parse_openai_sse_line(line: &str) -> Option<StreamChunk> {
    let data = line.strip_prefix("data: ")?;
    let data = data.trim();

    if data == "[DONE]" {
        return Some(StreamChunk::Done);
    }

    let parsed: Value = serde_json::from_str(data).ok()?;
    let choice = parsed.get("choices")?.get(0)?;
    let delta = choice.get("delta")?;

    Some(StreamChunk::Delta {
        content: delta
            .get("content")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        tool_calls: delta.get("tool_calls").and_then(|v| v.as_array()).cloned(),
        finish_reason: choice
            .get("finish_reason")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

fn accumulate_tool_call_deltas(accumulated: &mut Vec<AccumulatedToolCall>, deltas: &[Value]) {
    for delta in deltas {
        let index = delta["index"].as_u64().unwrap_or(0) as usize;

        while accumulated.len() <= index {
            accumulated.push(AccumulatedToolCall::default());
        }

        if let Some(id) = delta["id"].as_str() {
            accumulated[index].id = id.to_string();
        }
        if let Some(name) = delta["function"]["name"].as_str() {
            accumulated[index].name.push_str(name);
        }
        if let Some(args) = delta["function"]["arguments"].as_str() {
            accumulated[index].arguments.push_str(args);
        }
    }
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
            "stream": true,
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

        // Stream the response
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();
        let mut tool_calls: Vec<AccumulatedToolCall> = Vec::new();
        let mut final_finish_reason: Option<String> = None;

        loop {
            let next = tokio::time::timeout(STREAM_CHUNK_TIMEOUT, stream.next()).await;
            let chunk_result = match next {
                Ok(Some(r)) => r,
                Ok(None) => break,
                Err(_) => {
                    warn!(
                        "OpenAI stream chunk timeout after {}s",
                        STREAM_CHUNK_TIMEOUT.as_secs()
                    );
                    send_sse_event(tx, json!({"type": "error", "error": "failed to build assistant response: api error: stream timed out"})).await;
                    return;
                }
            };
            let chunk = match chunk_result {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!(error = %e, "error reading OpenAI stream chunk");
                    send_sse_event(tx, json!({"type": "error", "error": format!("failed to build assistant response: api error: {e}")})).await;
                    return;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                match parse_openai_sse_line(&line) {
                    Some(StreamChunk::Done) => {
                        debug!("OpenAI stream done");
                    }
                    Some(StreamChunk::Delta {
                        content,
                        tool_calls: tc_deltas,
                        finish_reason,
                    }) => {
                        if let Some(text) = content.filter(|t| !t.is_empty()) {
                            full_text.push_str(&text);
                            send_sse_event(tx, json!({"type": "text_delta", "delta": text})).await;
                        }

                        if let Some(deltas) = tc_deltas {
                            accumulate_tool_call_deltas(&mut tool_calls, &deltas);
                        }

                        if let Some(reason) = finish_reason {
                            final_finish_reason = Some(reason);
                        }
                    }
                    None => {}
                }
            }
        }

        let finish_reason = final_finish_reason.as_deref().unwrap_or("");
        info!(finish_reason, "OpenAI finish_reason");

        if finish_reason == "stop" {
            send_sse_event(
                tx,
                json!({"type": "text", "text": full_text, "model": selected_model}),
            )
            .await;
            return;
        }

        if finish_reason == "tool_calls" {
            // Build the assistant message for conversation history
            let tool_calls_json: Vec<Value> = tool_calls
                .iter()
                .map(|tc| {
                    json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        },
                    })
                })
                .collect();

            messages.push(json!({
                "role": "assistant",
                "content": if full_text.is_empty() { Value::Null } else { Value::String(full_text.clone()) },
                "tool_calls": tool_calls_json,
            }));

            for tc in &tool_calls {
                let args: Value = serde_json::from_str(&tc.arguments).unwrap_or(json!({}));

                info!(tool = %tc.name, "executing tool call");
                send_sse_event(
                    tx,
                    json!({"type": "tool_call", "id": tc.id, "name": tc.name, "args": args}),
                )
                .await;

                let result =
                    match execute_tool(&state.pool, state.mcp_client.as_deref(), &tc.name, &args)
                        .await
                    {
                        Ok(v) => v,
                        Err(e) => {
                            error!(tool = %tc.name, error = %e, "tool execution failed");
                            json!({ "error": e.to_string() })
                        }
                    };

                debug!(tool = %tc.name, result = %result, "tool result");
                send_sse_event(
                    tx,
                    json!({"type": "tool_result", "id": tc.id, "result": result}),
                )
                .await;

                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tc.id,
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

#[cfg(test)]
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
    fn parse_sse_line_done() {
        let chunk = parse_openai_sse_line("data: [DONE]");
        assert!(matches!(chunk, Some(StreamChunk::Done)));
    }

    #[test]
    fn parse_sse_line_text_delta() {
        let line = r#"data: {"choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        match parse_openai_sse_line(line) {
            Some(StreamChunk::Delta {
                content,
                tool_calls,
                finish_reason,
            }) => {
                assert_eq!(content.as_deref(), Some("Hello"));
                assert!(tool_calls.is_none());
                assert!(finish_reason.is_none());
            }
            other => panic!("expected Delta, got {other:?}"),
        }
    }

    #[test]
    fn parse_sse_line_finish_stop() {
        let line = r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#;
        match parse_openai_sse_line(line) {
            Some(StreamChunk::Delta {
                content,
                finish_reason,
                ..
            }) => {
                assert!(content.is_none());
                assert_eq!(finish_reason.as_deref(), Some("stop"));
            }
            other => panic!("expected Delta, got {other:?}"),
        }
    }

    #[test]
    fn parse_sse_line_tool_call_delta() {
        let line = r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_abc","function":{"name":"list_accounts","arguments":""}}]},"finish_reason":null}]}"#;
        match parse_openai_sse_line(line) {
            Some(StreamChunk::Delta { tool_calls, .. }) => {
                let tc = tool_calls.expect("should have tool_calls");
                assert_eq!(tc.len(), 1);
                assert_eq!(tc[0]["id"], "call_abc");
                assert_eq!(tc[0]["function"]["name"], "list_accounts");
            }
            other => panic!("expected Delta, got {other:?}"),
        }
    }

    #[test]
    fn parse_sse_line_non_data_returns_none() {
        assert!(parse_openai_sse_line("event: ping").is_none());
        assert!(parse_openai_sse_line("").is_none());
    }

    #[test]
    fn accumulate_tool_calls_across_chunks() {
        let mut acc = Vec::new();

        // First chunk: id + function name
        accumulate_tool_call_deltas(
            &mut acc,
            &[
                json!({"index": 0, "id": "call_1", "function": {"name": "list_accounts", "arguments": ""}}),
            ],
        );
        assert_eq!(acc.len(), 1);
        assert_eq!(acc[0].id, "call_1");
        assert_eq!(acc[0].name, "list_accounts");
        assert_eq!(acc[0].arguments, "");

        // Second chunk: arguments fragment
        accumulate_tool_call_deltas(
            &mut acc,
            &[json!({"index": 0, "function": {"arguments": "{\"acc"}})],
        );
        assert_eq!(acc[0].arguments, "{\"acc");

        // Third chunk: more arguments
        accumulate_tool_call_deltas(
            &mut acc,
            &[json!({"index": 0, "function": {"arguments": "ount_type\": \"bank\"}"}})],
        );
        assert_eq!(acc[0].arguments, r#"{"account_type": "bank"}"#);
    }

    #[test]
    fn accumulate_multiple_tool_calls() {
        let mut acc = Vec::new();

        accumulate_tool_call_deltas(
            &mut acc,
            &[
                json!({"index": 0, "id": "call_1", "function": {"name": "list_accounts", "arguments": "{}"}}),
            ],
        );
        accumulate_tool_call_deltas(
            &mut acc,
            &[
                json!({"index": 1, "id": "call_2", "function": {"name": "list_assets", "arguments": "{}"}}),
            ],
        );

        assert_eq!(acc.len(), 2);
        assert_eq!(acc[0].name, "list_accounts");
        assert_eq!(acc[1].name, "list_assets");
    }
}
