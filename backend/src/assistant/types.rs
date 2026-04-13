use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mcp::McpError;
use crate::storage::StorageError;

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AssistantChatRequest {
    #[serde(default)]
    pub messages: Vec<AssistantChatMessageRequest>,
}

#[derive(Debug, Deserialize)]
pub struct AssistantChatMessageRequest {
    pub role: String,
    /// Text content for user/system messages; null or a string for assistant messages
    /// that also carry `tool_calls`.
    pub content: Value,
    /// OpenAI-format tool_calls array (assistant role only).
    #[serde(default)]
    pub tool_calls: Option<Value>,
    /// Tool call ID this message is a result for (role: "tool").
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssistantChatResponse {
    pub message: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantChatErrorResponse {
    pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct AssistantModelSelectionRequest {
    pub model: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AssistantModelsResponse {
    pub models: Vec<String>,
    pub selected_model: String,
    pub reasoning: bool,
    pub reasoning_effort: ReasoningEffort,
    pub openai_enabled: bool,
    pub last_refreshed_at: Option<String>,
    pub refresh_error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

impl ReasoningEffort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
        }
    }
}

impl fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ReasoningEffort {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "minimal" => Ok(Self::Minimal),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "xhigh" => Ok(Self::Xhigh),
            other => Err(format!(
                "invalid reasoning effort: {other}. Valid values: none, minimal, low, medium, high, xhigh"
            )),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GenerateTitleRequest {
    #[serde(default)]
    pub messages: Vec<AssistantChatMessageRequest>,
}

#[derive(Debug, Serialize)]
pub struct GenerateTitleResponse {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct ReasoningEffortRequest {
    pub effort: String,
}

#[derive(Debug, Serialize)]
pub struct SystemPromptResponse {
    pub prompt: String,
    pub is_default: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSystemPromptRequest {
    pub prompt: String,
}

// ── Error types ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum AssistantError {
    Storage(StorageError),
    Mcp(McpError),
}

impl fmt::Display for AssistantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssistantError::Storage(e) => write!(f, "storage error: {e}"),
            AssistantError::Mcp(e) => write!(f, "mcp error: {e}"),
        }
    }
}

impl From<StorageError> for AssistantError {
    fn from(e: StorageError) -> Self {
        AssistantError::Storage(e)
    }
}

impl From<McpError> for AssistantError {
    fn from(e: McpError) -> Self {
        AssistantError::Mcp(e)
    }
}

#[derive(Debug)]
pub enum AssistantModelRefreshError {
    Config(&'static str),
    Provider(String),
}

impl fmt::Display for AssistantModelRefreshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssistantModelRefreshError::Config(message) => f.write_str(message),
            AssistantModelRefreshError::Provider(message) => f.write_str(message),
        }
    }
}

// ── OpenAI internal response shapes ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OpenAiModelsListResponse {
    pub data: Vec<OpenAiModelRecord>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAiModelRecord {
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_error_display_storage() {
        let e = AssistantError::Storage(StorageError::Internal("thing"));
        assert!(e.to_string().starts_with("storage error:"));
    }

    #[test]
    fn model_refresh_error_display_config() {
        let e = AssistantModelRefreshError::Config("bad config");
        assert_eq!(e.to_string(), "bad config");
    }

    #[test]
    fn model_refresh_error_display_provider() {
        let e = AssistantModelRefreshError::Provider("provider down".to_string());
        assert_eq!(e.to_string(), "provider down");
    }
}
