//! Request/response data structures for OpenAI and custom LLM formats.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// OpenAI Request Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiChatRequest {
    pub model: Option<String>,
    pub messages: Vec<OpenAiMessage>,
    #[serde(default)]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(default)]
    pub response_format: Option<serde_json::Value>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<i64>,
    #[serde(default)]
    pub max_completion_tokens: Option<i64>,
    #[serde(default)]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenAiMessage {
    pub role: String,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Custom LLM Request Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CustomLlmRequest {
    pub contents: Vec<String>,
    #[serde(rename = "llmId")]
    pub llm_id: String,
    #[serde(rename = "isStream")]
    pub is_stream: bool,
    #[serde(rename = "llmConfig")]
    pub llm_config: LlmConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmConfig {
    pub temperature: f64,
    pub top_p: f64,
    pub top_k: i32,
    pub repitition_penalty: f64, // preserve API typo
    pub max_new_token: i64,
}

// ---------------------------------------------------------------------------
// Custom LLM Response Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CustomLlmResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default, rename = "responseCode")]
    pub response_code: Option<String>,
    #[serde(default, rename = "promptToken")]
    pub prompt_token: Option<i64>,
    #[serde(default, rename = "completionToken")]
    pub completion_token: Option<i64>,
}

// ---------------------------------------------------------------------------
// Custom LLM SSE Chunk
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CustomSseChunk {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub event_status: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub response_code: Option<String>,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub prompt_token: Option<i64>,
    #[serde(default)]
    pub completion_token: Option<i64>,
}

// ---------------------------------------------------------------------------
// Proxy Configuration
// ---------------------------------------------------------------------------

/// Proxy operation mode
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProxyMode {
    /// Fabrix custom format: converts OpenAI → Custom format (llmId, contents)
    #[default]
    Fabrix,
    /// OpenAI proxy mode: keeps OpenAI format (model, messages), only injects tools
    OpenAi,
}

impl ProxyMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" | "openai_proxy" => ProxyMode::OpenAi,
            _ => ProxyMode::Fabrix,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub llm_url: String,
    pub api_key: String,
    pub llm_id: String,
    pub temperature: f64,
    pub top_p: f64,
    pub top_k: i32,
    pub repetition_penalty: f64,
    pub max_tokens: i64,
    pub timeout_secs: u64,
    /// Proxy operation mode (fabrix or openai)
    pub mode: ProxyMode,
    /// Force non-streaming mode even if client requests streaming.
    /// Useful when the upstream LLM doesn't support streaming well.
    pub force_non_stream: bool,
    /// Enable debug logging for the proxy.
    pub debug: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            llm_url: String::new(),
            api_key: String::new(),
            llm_id: "gpt-oss".to_string(),
            temperature: 0.7,
            top_p: 0.9,
            top_k: 50,
            repetition_penalty: 1.0,
            max_tokens: 1024,
            timeout_secs: 120,
            mode: ProxyMode::default(),
            force_non_stream: false,
            debug: false,
        }
    }
}

impl ProxyConfig {
    /// Create a ProxyConfig from environment variables.
    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = std::env::var("CONNECTOR_API_KEY")
            .map_err(|_| anyhow::anyhow!("CONNECTOR_API_KEY environment variable not set"))?;
        let llm_url = std::env::var("CONNECTOR_LLM_URL")
            .map_err(|_| anyhow::anyhow!("CONNECTOR_LLM_URL environment variable not set"))?;

        // CONNECTOR_MODE: "fabrix" (default) or "openai"
        let mode = std::env::var("CONNECTOR_MODE")
            .map(|s| ProxyMode::from_str(&s))
            .unwrap_or_default();

        // CONNECTOR_FORCE_NON_STREAM: "true" or "1" to force non-streaming
        let force_non_stream = std::env::var("CONNECTOR_FORCE_NON_STREAM")
            .map(|s| s == "true" || s == "1")
            .unwrap_or(false);

        // CONNECTOR_DEBUG: "true" or "1" to enable debug logging
        let debug = std::env::var("CONNECTOR_DEBUG")
            .map(|s| s == "true" || s == "1")
            .unwrap_or(false);

        Ok(Self {
            api_key,
            llm_url,
            llm_id: std::env::var("CONNECTOR_LLM_ID").unwrap_or_else(|_| "gpt-oss".to_string()),
            temperature: parse_env_f64("CONNECTOR_TEMPERATURE", 0.7),
            top_p: parse_env_f64("CONNECTOR_TOP_P", 0.9),
            top_k: parse_env_i32("CONNECTOR_TOP_K", 50),
            repetition_penalty: parse_env_f64("CONNECTOR_REPETITION_PENALTY", 1.0),
            max_tokens: parse_env_i64("CONNECTOR_MAX_TOKENS", 1024),
            timeout_secs: parse_env_u64("CONNECTOR_TIMEOUT", 120),
            mode,
            force_non_stream,
            debug,
        })
    }
}

fn parse_env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_env_i32(key: &str, default: i32) -> i32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
