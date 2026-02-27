use goose::conversation::message::{Message, TokenState};
use goose::conversation::Conversation;
use serde::Deserialize;

/// Events emitted over the SSE stream from `POST /reply`.
///
/// The server serializes these with `#[serde(tag = "type")]`, so each JSON object
/// contains a `"type"` field that determines the variant.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum MessageEvent {
    /// An assistant message (may contain text, tool requests, tool responses, etc.)
    Message {
        message: Message,
        token_state: TokenState,
    },
    /// A non-fatal error from the agent.
    Error { error: String },
    /// Stream completed.
    Finish {
        reason: String,
        token_state: TokenState,
    },
    /// The agent switched to a different LLM model mid-stream.
    ModelChange { model: String, mode: String },
    /// An MCP server notification. The `message` field is kept as a raw JSON value
    /// to avoid depending on `rmcp` types in the public API.
    Notification {
        request_id: String,
        message: serde_json::Value,
    },
    /// The server replaced the full conversation history (e.g. after context compaction).
    UpdateConversation { conversation: Conversation },
    /// Heartbeat sent every 500ms to keep the connection alive. Safe to ignore.
    Ping,
}
