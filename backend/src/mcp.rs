use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

const MCP_TOOL_TIMEOUT: Duration = Duration::from_secs(30);

pub type SharedMcpClient = Arc<McpClient>;

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("failed to spawn MCP process: {0}")]
    Spawn(String),
    #[error("MCP I/O error: {0}")]
    Io(String),
    #[error("MCP protocol error: {0}")]
    Protocol(String),
    #[error("MCP tool error: {0}")]
    Tool(String),
}

pub struct McpClient {
    io: Mutex<McpClientIo>,
    /// Tool definitions in OpenAI function-calling format, discovered at startup.
    pub tools: Vec<Value>,
}

struct McpClientIo {
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    next_id: u64,
    // Keep the child alive for the lifetime of the client.
    _child: Child,
}

impl McpClient {
    /// Spawn `npx -y mcp-searxng` with the given SearXNG URL, perform the MCP
    /// handshake, and discover available tools.
    pub async fn spawn(searxng_url: &str) -> Result<Self, McpError> {
        info!(
            command = "npx -y mcp-searxng",
            searxng_url, "spawning MCP server"
        );

        let mut child = Command::new("npx")
            .args(["-y", "mcp-searxng"])
            .env("SEARXNG_URL", searxng_url)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    warn!("MCP server command not found: `npx` must be installed and on PATH");
                }
                McpError::Spawn(e.to_string())
            })?;

        let stdin = child.stdin.take().expect("stdin should be piped");
        let stdout = child.stdout.take().expect("stdout should be piped");
        let lines = BufReader::new(stdout).lines();

        let mut io = McpClientIo {
            stdin,
            lines,
            next_id: 1,
            _child: child,
        };

        // Step 1 – initialize
        let init_result = io
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "siniscalco", "version": "1.0.0" }
                }),
            )
            .await?;

        debug!(server_info = %init_result, "MCP server initialized");

        // Step 2 – send notifications/initialized (no response expected)
        io.notify("notifications/initialized").await?;

        // Step 3 – discover tools
        let tools_result = io.request("tools/list", json!({})).await?;
        let mcp_tools = tools_result["tools"].as_array().ok_or_else(|| {
            McpError::Protocol("missing tools array in tools/list response".into())
        })?;

        // Convert MCP tool schema to OpenAI function-calling format.
        let tools: Vec<Value> = mcp_tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t["name"],
                        "description": t["description"],
                        "parameters": t["inputSchema"],
                    }
                })
            })
            .collect();

        info!(tool_count = tools.len(), "MCP tools discovered");
        for t in &tools {
            info!(tool = %t["function"]["name"], "MCP tool available");
        }

        Ok(McpClient {
            io: Mutex::new(io),
            tools,
        })
    }

    /// Call an MCP tool by name with the given arguments (parsed from the OpenAI
    /// tool call's `arguments` JSON string). Returns the concatenated text content.
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<String, McpError> {
        let mut io = self.io.lock().await;

        let result = tokio::time::timeout(
            MCP_TOOL_TIMEOUT,
            io.request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            ),
        )
        .await
        .map_err(|_| {
            McpError::Io(format!(
                "MCP tool call timed out after {}s",
                MCP_TOOL_TIMEOUT.as_secs()
            ))
        })??;

        if result["isError"].as_bool().unwrap_or(false) {
            let msg = result["content"][0]["text"]
                .as_str()
                .unwrap_or("tool returned an error");
            return Err(McpError::Tool(msg.to_string()));
        }

        let text = result["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| {
                if c["type"] == "text" {
                    c["text"].as_str()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }

    /// Names of all tools this client exposes.
    pub fn tool_names(&self) -> impl Iterator<Item = &str> {
        self.tools
            .iter()
            .filter_map(|t| t["function"]["name"].as_str())
    }
}

impl McpClientIo {
    /// Send a JSON-RPC request and return the `result` field of the matching response.
    /// Notifications (messages without an `id`) are skipped while waiting.
    async fn request(&mut self, method: &str, params: Value) -> Result<Value, McpError> {
        let id = self.next_id;
        self.next_id += 1;

        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let line = serde_json::to_string(&msg).expect("message should serialize") + "\n";
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| McpError::Io(e.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| McpError::Io(e.to_string()))?;

        loop {
            let raw = self
                .lines
                .next_line()
                .await
                .map_err(|e| McpError::Io(e.to_string()))?
                .ok_or_else(|| McpError::Protocol("MCP server closed stdout".into()))?;

            let response: Value = serde_json::from_str(&raw)
                .map_err(|e| McpError::Protocol(format!("invalid JSON from MCP server: {e}")))?;

            debug!(mcp_response = %response, "received MCP response");

            // Skip server-sent notifications (no `id` field).
            if response.get("id").is_none() {
                debug!(method = %response["method"], "received MCP notification, skipping");
                continue;
            }

            if let Some(error) = response.get("error") {
                return Err(McpError::Protocol(format!("MCP error: {error}")));
            }

            return Ok(response["result"].clone());
        }
    }

    /// Send a JSON-RPC notification (no response expected).
    async fn notify(&mut self, method: &str) -> Result<(), McpError> {
        let msg = json!({ "jsonrpc": "2.0", "method": method });
        let line = serde_json::to_string(&msg).expect("notification should serialize") + "\n";
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| McpError::Io(e.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| McpError::Io(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inline Node.js script that acts as a minimal MCP server over stdio.
    const FAKE_MCP_SERVER: &str = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, terminal: false });
rl.on('line', line => {
  let msg;
  try { msg = JSON.parse(line); } catch { return; }
  const id = msg.id;
  if (id === undefined || id === null) return; // notification
  let resp;
  if (msg.method === 'initialize') {
    resp = { jsonrpc: '2.0', id, result: { protocolVersion: '2024-11-05', capabilities: {}, serverInfo: { name: 'test', version: '0.0.1' } } };
  } else if (msg.method === 'tools/list') {
    resp = { jsonrpc: '2.0', id, result: { tools: [
      { name: 'searxng_web_search', description: 'Search the web', inputSchema: { type: 'object', properties: { query: { type: 'string', description: 'Search query' } }, required: ['query'] } }
    ] } };
  } else if (msg.method === 'tools/call') {
    const q = (msg.params && msg.params.arguments && msg.params.arguments.query) || '';
    resp = { jsonrpc: '2.0', id, result: { content: [{ type: 'text', text: 'results for: ' + q }], isError: false } };
  } else {
    resp = { jsonrpc: '2.0', id, error: { code: -32601, message: 'Method not found' } };
  }
  process.stdout.write(JSON.stringify(resp) + '\n');
});
"#;

    async fn spawn_fake_client() -> McpClient {
        let mut child = Command::new("node")
            .arg("-e")
            .arg(FAKE_MCP_SERVER)
            .env("SEARXNG_URL", "http://127.0.0.1:1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("node should be available");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let lines = BufReader::new(stdout).lines();

        let mut io = McpClientIo {
            stdin,
            lines,
            next_id: 1,
            _child: child,
        };

        io.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0.0.1" }
            }),
        )
        .await
        .expect("initialize should succeed");

        io.notify("notifications/initialized")
            .await
            .expect("notify should succeed");

        let tools_result = io
            .request("tools/list", json!({}))
            .await
            .expect("tools/list should succeed");

        let mcp_tools = tools_result["tools"].as_array().unwrap();
        let tools: Vec<Value> = mcp_tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t["name"],
                        "description": t["description"],
                        "parameters": t["inputSchema"],
                    }
                })
            })
            .collect();

        McpClient {
            io: Mutex::new(io),
            tools,
        }
    }

    #[tokio::test]
    async fn discovers_searxng_tool() {
        let client = spawn_fake_client().await;
        assert_eq!(client.tools.len(), 1);
        assert_eq!(client.tools[0]["function"]["name"], "searxng_web_search");
        assert_eq!(
            client.tools[0]["function"]["parameters"]["properties"]["query"]["type"],
            "string"
        );
    }

    #[tokio::test]
    async fn calls_searxng_tool_and_returns_text() {
        let client = spawn_fake_client().await;
        let result = client
            .call_tool("searxng_web_search", json!({ "query": "rust programming" }))
            .await
            .expect("call_tool should succeed");
        assert_eq!(result, "results for: rust programming");
    }

    #[tokio::test]
    async fn tool_names_iterator() {
        let client = spawn_fake_client().await;
        let names: Vec<&str> = client.tool_names().collect();
        assert_eq!(names, ["searxng_web_search"]);
    }
}
