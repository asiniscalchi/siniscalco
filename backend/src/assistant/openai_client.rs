use std::{convert::Infallible, time::Duration};

use futures_util::StreamExt;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use axum::response::sse::Event;

use crate::AppState;

use super::tool_executor::{execute_tool, tool_definitions};
use super::types::AssistantChatMessageRequest;

const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const MAX_TOOL_ROUNDS: usize = 5;
const MAX_INPUT_SIZE_BYTES: usize = 256 * 1024;
const STREAM_CHUNK_TIMEOUT: Duration = Duration::from_secs(30);

pub const DEFAULT_SYSTEM_PROMPT: &str = "\
You are a helpful portfolio assistant for the Siniscalco app. \
The app tracks investment accounts, assets, transactions, and fund transfers. \
Use the available tools to look up live data before answering. \
Be concise and precise. Format monetary amounts with their currency code.";

pub const fn openai_responses_url() -> &'static str {
    OPENAI_RESPONSES_URL
}

pub async fn send_sse_event(tx: &mpsc::Sender<Result<Event, Infallible>>, data: Value) {
    let event = Event::default().data(data.to_string());
    let _ = tx.send(Ok(event)).await;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ToolCall {
    id: String,
    name: String,
    arguments: String,
}

pub async fn openai_responses_streaming(
    state: &AppState,
    incoming: &[AssistantChatMessageRequest],
    api_key: &str,
    model: &str,
    reasoning_effort: &str,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
) {
    let stored_prompt = crate::storage::settings::get_app_setting(
        &state.pool,
        super::model_registry::SETTING_SYSTEM_PROMPT,
    )
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());

    let instructions = build_instructions(&stored_prompt, incoming);
    let mut input = build_response_input_items(incoming);

    let input_size = input.to_string().len() + instructions.len();
    if input_size > MAX_INPUT_SIZE_BYTES {
        warn!(
            input_size,
            max = MAX_INPUT_SIZE_BYTES,
            "assistant response input exceeded maximum size"
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
            .iter()
            .map(response_tool_definition)
            .collect::<Vec<_>>();
        if let Some(mcp) = &state.mcp_client {
            tools.extend(mcp.tools.iter().map(response_tool_definition));
        }
        tools
    };

    let mut previous_response_id: Option<String> = None;

    for _ in 0..MAX_TOOL_ROUNDS {
        let body = build_responses_request_body(
            model,
            &instructions,
            &input,
            &all_tools,
            reasoning_effort,
            previous_response_id.as_deref(),
        );

        let response = match state
            .http_client
            .post(&state.openai_responses_url)
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

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut response_id: Option<String> = None;
        let mut tool_calls: Vec<ToolCall> = Vec::new();

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

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                let data = data.trim();

                if data == "[DONE]" {
                    debug!("OpenAI Responses stream done");
                    continue;
                }

                let Ok(event) = serde_json::from_str::<Value>(data) else {
                    continue;
                };

                let event_type = event["type"].as_str().unwrap_or("");

                match event_type {
                    "response.created" | "response.completed" => {
                        if let Some(id) = event["response"]["id"].as_str() {
                            response_id = Some(id.to_string());
                        }
                    }
                    "response.output_text.delta" => {
                        if let Some(delta) = event["delta"].as_str() {
                            if !delta.is_empty() {
                                send_sse_event(tx, json!({"type": "text_delta", "delta": delta}))
                                    .await;
                            }
                        }
                    }
                    "response.reasoning_summary_text.delta" => {
                        if let Some(delta) = event["delta"].as_str() {
                            if !delta.is_empty() {
                                send_sse_event(
                                    tx,
                                    json!({"type": "reasoning_delta", "delta": delta}),
                                )
                                .await;
                            }
                        }
                    }
                    "response.function_call_arguments.done" => {
                        tool_calls.push(ToolCall {
                            id: event["call_id"].as_str().unwrap_or_default().to_string(),
                            name: event["name"].as_str().unwrap_or_default().to_string(),
                            arguments: event["arguments"].as_str().unwrap_or("{}").to_string(),
                        });
                    }
                    _ => {}
                }
            }
        }

        if tool_calls.is_empty() {
            return;
        }

        let Some(next_response_id) = response_id else {
            warn!("Responses payload with tool calls had no response id");
            send_sse_event(
                tx,
                json!({"type": "error", "error": "failed to build assistant response: api error: Responses payload missing response id"}),
            )
            .await;
            return;
        };

        let mut tool_outputs = Vec::with_capacity(tool_calls.len());
        for tool_call in &tool_calls {
            let args: Value = serde_json::from_str(&tool_call.arguments).unwrap_or(json!({}));

            info!(tool = %tool_call.name, "executing tool call");
            send_sse_event(
                tx,
                json!({"type": "tool_call", "id": tool_call.id, "name": tool_call.name, "args": args}),
            )
            .await;

            let result = match execute_tool(
                &state.pool,
                state.mcp_client.as_deref(),
                &tool_call.name,
                &args,
            )
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    error!(tool = %tool_call.name, error = %e, "tool execution failed");
                    json!({ "error": e.to_string() })
                }
            };

            debug!(tool = %tool_call.name, result = %result, "tool result");
            send_sse_event(
                tx,
                json!({"type": "tool_result", "id": tool_call.id, "result": result}),
            )
            .await;

            tool_outputs.push(json!({
                "type": "function_call_output",
                "call_id": tool_call.id,
                "output": result.to_string(),
            }));
        }

        previous_response_id = Some(next_response_id);
        input = Value::Array(tool_outputs);
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

/// Known reasoning-capable model families. OpenAI's `/v1/models` endpoint does
/// not expose a capability flag, so we match on known prefixes. Update this
/// list when new reasoning models are released.
pub fn is_reasoning_model(model: &str) -> bool {
    model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
        || model.starts_with("o5")
        || model.starts_with("gpt-5")
}

fn build_responses_request_body(
    model: &str,
    instructions: &str,
    input: &Value,
    tools: &[Value],
    reasoning_effort: &str,
    previous_response_id: Option<&str>,
) -> Value {
    let mut body = json!({
        "model": model,
        "instructions": instructions,
        "input": input,
        "tools": tools,
        "stream": true,
        "store": false,
    });

    if is_reasoning_model(model) {
        body["reasoning"] = json!({
            "effort": reasoning_effort,
            "summary": "detailed",
        });
    }

    if let Some(previous_response_id) = previous_response_id {
        body["previous_response_id"] = json!(previous_response_id);
    }

    body
}

fn build_instructions(system_prompt: &str, incoming: &[AssistantChatMessageRequest]) -> String {
    let mut instructions = vec![system_prompt.trim().to_string()];

    for msg in incoming {
        if msg.role != "system" {
            continue;
        }
        let text = extract_message_text(&msg.content);
        if !text.is_empty() {
            instructions.push(text);
        }
    }

    instructions.join("\n\n")
}

fn build_response_input_items(incoming: &[AssistantChatMessageRequest]) -> Value {
    Value::Array(
        incoming
            .iter()
            .filter(|msg| msg.role != "system")
            .flat_map(message_to_response_input_items)
            .collect(),
    )
}

fn message_to_response_input_items(msg: &AssistantChatMessageRequest) -> Vec<Value> {
    match msg.role.as_str() {
        "user" | "assistant" => {
            let mut items = Vec::new();
            let text = extract_message_text(&msg.content);
            if !text.is_empty() {
                items.push(json!({
                    "role": msg.role,
                    "content": text,
                }));
            }

            if msg.role == "assistant"
                && let Some(tool_calls) = msg.tool_calls.as_ref().and_then(Value::as_array)
            {
                items.extend(tool_calls.iter().map(|tool_call| {
                    json!({
                        "type": "function_call",
                        "call_id": tool_call["id"],
                        "name": tool_call["function"]["name"],
                        "arguments": tool_call["function"]["arguments"],
                    })
                }));
            }

            items
        }
        "tool" => msg
            .tool_call_id
            .as_ref()
            .map(|tool_call_id| {
                vec![json!({
                    "type": "function_call_output",
                    "call_id": tool_call_id,
                    "output": stringify_message_content(&msg.content),
                })]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn response_tool_definition(tool: &Value) -> Value {
    if tool["type"] == "function" && tool.get("function").is_some() {
        json!({
            "type": "function",
            "name": tool["function"]["name"],
            "description": tool["function"]["description"],
            "parameters": tool["function"]["parameters"],
        })
    } else {
        tool.clone()
    }
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

fn stringify_message_content(content: &Value) -> String {
    if let Some(text) = content.as_str() {
        text.to_string()
    } else {
        content.to_string()
    }
}

fn extract_response_output_text(payload: &Value) -> String {
    if let Some(text) = payload.get("output_text").and_then(Value::as_str) {
        return text.to_string();
    }

    payload["output"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["type"].as_str() == Some("message"))
        .flat_map(|item| item["content"].as_array().into_iter().flatten())
        .filter_map(|part| {
            if part["type"].as_str() != Some("output_text") {
                return None;
            }

            part["text"].as_str().map(str::to_string)
        })
        .collect::<Vec<_>>()
        .join("")
}

fn extract_response_tool_calls(payload: &Value) -> Vec<ToolCall> {
    payload["output"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["type"].as_str() == Some("function_call"))
        .map(|item| ToolCall {
            id: item["call_id"]
                .as_str()
                .or_else(|| item["id"].as_str())
                .unwrap_or_default()
                .to_string(),
            name: item["name"].as_str().unwrap_or_default().to_string(),
            arguments: item["arguments"].as_str().unwrap_or("{}").to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_instructions_appends_system_messages() {
        let incoming = vec![AssistantChatMessageRequest {
            role: "system".to_string(),
            content: json!("Secondary instruction"),
            tool_calls: None,
            tool_call_id: None,
        }];

        let instructions = build_instructions("Primary instruction", &incoming);
        assert_eq!(instructions, "Primary instruction\n\nSecondary instruction");
    }

    #[test]
    fn build_response_input_items_maps_tool_history() {
        let incoming = vec![
            AssistantChatMessageRequest {
                role: "user".to_string(),
                content: json!("How many accounts do I have?"),
                tool_calls: None,
                tool_call_id: None,
            },
            AssistantChatMessageRequest {
                role: "assistant".to_string(),
                content: Value::Null,
                tool_calls: Some(json!([{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "list_accounts",
                        "arguments": "{}"
                    }
                }])),
                tool_call_id: None,
            },
            AssistantChatMessageRequest {
                role: "tool".to_string(),
                content: json!("{\"count\":1}"),
                tool_calls: None,
                tool_call_id: Some("call_1".to_string()),
            },
        ];

        let items = build_response_input_items(&incoming);
        assert_eq!(
            items,
            json!([
                { "role": "user", "content": "How many accounts do I have?" },
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "list_accounts",
                    "arguments": "{}"
                },
                {
                    "type": "function_call_output",
                    "call_id": "call_1",
                    "output": "{\"count\":1}"
                }
            ])
        );
    }

    #[test]
    fn response_tool_definition_flattens_chat_shape() {
        let tool = json!({
            "type": "function",
            "function": {
                "name": "list_accounts",
                "description": "List accounts",
                "parameters": { "type": "object", "properties": {} }
            }
        });

        assert_eq!(
            response_tool_definition(&tool),
            json!({
                "type": "function",
                "name": "list_accounts",
                "description": "List accounts",
                "parameters": { "type": "object", "properties": {} }
            })
        );
    }

    #[test]
    fn is_reasoning_model_matches_known_families() {
        assert!(is_reasoning_model("o3"));
        assert!(is_reasoning_model("o3-mini"));
        assert!(is_reasoning_model("o4-mini"));
        assert!(is_reasoning_model("o1-preview"));
        assert!(is_reasoning_model("o5"));
        assert!(is_reasoning_model("gpt-5.4"));
        assert!(is_reasoning_model("gpt-5.4-mini"));
        assert!(is_reasoning_model("gpt-5.4-nano"));
    }

    #[test]
    fn is_reasoning_model_rejects_non_reasoning() {
        assert!(!is_reasoning_model("gpt-4.1"));
        assert!(!is_reasoning_model("gpt-4.1-mini"));
        assert!(!is_reasoning_model("gpt-4o"));
        assert!(!is_reasoning_model("gpt-4o-mini"));
    }

    #[test]
    fn build_body_includes_reasoning_for_reasoning_model() {
        let body = build_responses_request_body(
            "o4-mini",
            "You are helpful.",
            &json!([]),
            &[],
            "high",
            None,
        );
        assert_eq!(body["reasoning"]["effort"], "high");
        assert_eq!(body["reasoning"]["summary"], "detailed");
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], true);
        assert_eq!(body["instructions"], "You are helpful.");
    }

    #[test]
    fn build_body_omits_reasoning_for_non_reasoning_model() {
        let body = build_responses_request_body(
            "gpt-4.1-mini",
            "You are helpful.",
            &json!([]),
            &[],
            "medium",
            None,
        );
        assert!(body.get("reasoning").is_none());
    }

    #[test]
    fn build_body_includes_previous_response_id_when_provided() {
        let body = build_responses_request_body(
            "gpt-4.1",
            "instructions",
            &json!([]),
            &[],
            "medium",
            Some("resp_abc"),
        );
        assert_eq!(body["previous_response_id"], "resp_abc");
        assert_eq!(body["instructions"], "instructions");
    }

    #[test]
    fn extract_response_output_text_reads_message_content() {
        let payload = json!({
            "output": [{
                "type": "message",
                "content": [
                    { "type": "output_text", "text": "Hello " },
                    { "type": "output_text", "text": "world" }
                ]
            }]
        });

        assert_eq!(extract_response_output_text(&payload), "Hello world");
    }

    #[test]
    fn extract_response_tool_calls_reads_function_calls() {
        let payload = json!({
            "output": [{
                "type": "function_call",
                "call_id": "call_1",
                "name": "list_accounts",
                "arguments": "{}"
            }]
        });

        assert_eq!(
            extract_response_tool_calls(&payload),
            vec![ToolCall {
                id: "call_1".to_string(),
                name: "list_accounts".to_string(),
                arguments: "{}".to_string(),
            }]
        );
    }
}
