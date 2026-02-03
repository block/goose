//! OpenTelemetry Semantic Conventions for GenAI and MCP
//!
//! This module implements the OpenTelemetry semantic conventions for generative AI
//! as specified in the OpenTelemetry specification, along with Goose-specific extensions
//! and MCP (Model Context Protocol) conventions.
//!
//! Reference: https://opentelemetry.io/docs/specs/semconv/gen-ai/

/// OpenTelemetry Semantic Conventions for Generative AI
pub mod gen_ai {
    // =========================================================================
    // System Attributes
    // =========================================================================

    /// The name of the GenAI system (e.g., "anthropic", "openai", "google", "azure")
    pub const SYSTEM: &str = "gen_ai.system";

    /// The name of the operation (e.g., "chat", "embeddings", "completion")
    pub const OPERATION_NAME: &str = "gen_ai.operation.name";

    // =========================================================================
    // Request Attributes
    // =========================================================================

    /// The model requested by the client
    pub const REQUEST_MODEL: &str = "gen_ai.request.model";

    /// Maximum number of tokens the LLM generates for a request
    pub const REQUEST_MAX_TOKENS: &str = "gen_ai.request.max_tokens";

    /// The temperature setting for generation
    pub const REQUEST_TEMPERATURE: &str = "gen_ai.request.temperature";

    /// The top_p (nucleus sampling) setting
    pub const REQUEST_TOP_P: &str = "gen_ai.request.top_p";

    /// The top_k sampling setting
    pub const REQUEST_TOP_K: &str = "gen_ai.request.top_k";

    /// Frequency penalty setting
    pub const REQUEST_FREQUENCY_PENALTY: &str = "gen_ai.request.frequency_penalty";

    /// Presence penalty setting
    pub const REQUEST_PRESENCE_PENALTY: &str = "gen_ai.request.presence_penalty";

    /// Stop sequences for generation
    pub const REQUEST_STOP_SEQUENCES: &str = "gen_ai.request.stop_sequences";

    // =========================================================================
    // Response Attributes
    // =========================================================================

    /// The unique identifier for the response
    pub const RESPONSE_ID: &str = "gen_ai.response.id";

    /// The model that generated the response (may differ from request model)
    pub const RESPONSE_MODEL: &str = "gen_ai.response.model";

    /// Reasons why the generation finished
    pub const RESPONSE_FINISH_REASONS: &str = "gen_ai.response.finish_reasons";

    // =========================================================================
    // Token Usage Attributes
    // =========================================================================

    /// Number of input tokens used
    pub const USAGE_INPUT_TOKENS: &str = "gen_ai.usage.input_tokens";

    /// Number of output tokens generated
    pub const USAGE_OUTPUT_TOKENS: &str = "gen_ai.usage.output_tokens";

    /// Total number of tokens (input + output)
    pub const USAGE_TOTAL_TOKENS: &str = "gen_ai.usage.total_tokens";

    // =========================================================================
    // Goose Extensions - Cost Tracking
    // =========================================================================

    /// Cost of the request in USD
    pub const USAGE_COST_USD: &str = "gen_ai.usage.cost_usd";

    /// Number of cached tokens used (for providers that support caching)
    pub const USAGE_CACHED_TOKENS: &str = "gen_ai.usage.cached_tokens";

    /// Cache read tokens (tokens retrieved from cache)
    pub const USAGE_CACHE_READ_TOKENS: &str = "gen_ai.usage.cache_read_tokens";

    /// Cache write tokens (tokens written to cache)
    pub const USAGE_CACHE_WRITE_TOKENS: &str = "gen_ai.usage.cache_write_tokens";

    // =========================================================================
    // Tool/Function Calling Attributes
    // =========================================================================

    /// Name of the tool being called
    pub const TOOL_NAME: &str = "gen_ai.tool.name";

    /// Unique identifier for the tool call
    pub const TOOL_CALL_ID: &str = "gen_ai.tool.call_id";

    /// JSON-encoded arguments for the tool call
    pub const TOOL_ARGUMENTS: &str = "gen_ai.tool.arguments";

    /// Result of the tool call
    pub const TOOL_RESULT: &str = "gen_ai.tool.result";

    /// Whether the tool call was successful
    pub const TOOL_SUCCESS: &str = "gen_ai.tool.success";

    // =========================================================================
    // Content Attributes
    // =========================================================================

    /// The role of the message (system, user, assistant, tool)
    pub const MESSAGE_ROLE: &str = "gen_ai.message.role";

    /// The content of the message
    pub const MESSAGE_CONTENT: &str = "gen_ai.message.content";

    // =========================================================================
    // Error Attributes
    // =========================================================================

    /// The type of error that occurred
    pub const ERROR_TYPE: &str = "gen_ai.error.type";

    /// The error message
    pub const ERROR_MESSAGE: &str = "gen_ai.error.message";

    // =========================================================================
    // Known System Values
    // =========================================================================

    /// Anthropic system
    pub const SYSTEM_ANTHROPIC: &str = "anthropic";

    /// OpenAI system
    pub const SYSTEM_OPENAI: &str = "openai";

    /// Google system
    pub const SYSTEM_GOOGLE: &str = "google";

    /// Azure OpenAI system
    pub const SYSTEM_AZURE: &str = "azure";

    /// AWS Bedrock system
    pub const SYSTEM_BEDROCK: &str = "bedrock";

    /// Ollama system
    pub const SYSTEM_OLLAMA: &str = "ollama";

    // =========================================================================
    // Known Operation Names
    // =========================================================================

    /// Chat completion operation
    pub const OPERATION_CHAT: &str = "chat";

    /// Text completion operation
    pub const OPERATION_COMPLETION: &str = "completion";

    /// Embeddings generation operation
    pub const OPERATION_EMBEDDINGS: &str = "embeddings";

    // =========================================================================
    // Known Finish Reasons
    // =========================================================================

    /// Generation stopped naturally
    pub const FINISH_REASON_STOP: &str = "stop";

    /// Generation stopped due to length limit
    pub const FINISH_REASON_LENGTH: &str = "length";

    /// Generation stopped due to tool call
    pub const FINISH_REASON_TOOL_CALLS: &str = "tool_calls";

    /// Generation stopped due to content filter
    pub const FINISH_REASON_CONTENT_FILTER: &str = "content_filter";

    /// Generation stopped for unknown reason
    pub const FINISH_REASON_OTHER: &str = "other";
}

/// MCP (Model Context Protocol) Semantic Conventions
pub mod mcp {
    // =========================================================================
    // Server Attributes
    // =========================================================================

    /// Name of the MCP server
    pub const SERVER_NAME: &str = "mcp.server.name";

    /// Version of the MCP server
    pub const SERVER_VERSION: &str = "mcp.server.version";

    /// Server identifier (unique ID)
    pub const SERVER_ID: &str = "mcp.server.id";

    /// Server status (connected, disconnected, unhealthy)
    pub const SERVER_STATUS: &str = "mcp.server.status";

    // =========================================================================
    // Transport Attributes
    // =========================================================================

    /// Type of transport (stdio, sse, websocket)
    pub const TRANSPORT_TYPE: &str = "mcp.transport.type";

    /// Transport endpoint URL (for network transports)
    pub const TRANSPORT_ENDPOINT: &str = "mcp.transport.endpoint";

    // =========================================================================
    // Capability Attributes
    // =========================================================================

    /// Number of tools provided by the server
    pub const TOOL_COUNT: &str = "mcp.tools.count";

    /// Number of resources provided by the server
    pub const RESOURCE_COUNT: &str = "mcp.resources.count";

    /// Number of prompts provided by the server
    pub const PROMPT_COUNT: &str = "mcp.prompts.count";

    /// Whether the server supports sampling
    pub const SUPPORTS_SAMPLING: &str = "mcp.capabilities.sampling";

    /// Whether the server supports logging
    pub const SUPPORTS_LOGGING: &str = "mcp.capabilities.logging";

    // =========================================================================
    // Tool Execution Attributes
    // =========================================================================

    /// Name of the tool being executed
    pub const TOOL_NAME: &str = "mcp.tool.name";

    /// Execution duration in milliseconds
    pub const TOOL_DURATION_MS: &str = "mcp.tool.duration_ms";

    /// Whether the tool execution was successful
    pub const TOOL_SUCCESS: &str = "mcp.tool.success";

    /// Error message if tool execution failed
    pub const TOOL_ERROR: &str = "mcp.tool.error";

    // =========================================================================
    // Permission Attributes
    // =========================================================================

    /// Whether permission was granted
    pub const PERMISSION_GRANTED: &str = "mcp.permission.granted";

    /// Reason for permission denial
    pub const PERMISSION_DENIAL_REASON: &str = "mcp.permission.denial_reason";

    /// User ID requesting permission
    pub const PERMISSION_USER_ID: &str = "mcp.permission.user_id";

    // =========================================================================
    // Bundle Attributes
    // =========================================================================

    /// Bundle identifier
    pub const BUNDLE_ID: &str = "mcp.bundle.id";

    /// Bundle name
    pub const BUNDLE_NAME: &str = "mcp.bundle.name";

    // =========================================================================
    // Known Transport Types
    // =========================================================================

    /// Standard I/O transport
    pub const TRANSPORT_STDIO: &str = "stdio";

    /// Server-Sent Events transport
    pub const TRANSPORT_SSE: &str = "sse";

    /// WebSocket transport
    pub const TRANSPORT_WEBSOCKET: &str = "websocket";

    // =========================================================================
    // Known Server Statuses
    // =========================================================================

    /// Server is connected and healthy
    pub const STATUS_CONNECTED: &str = "connected";

    /// Server is disconnected
    pub const STATUS_DISCONNECTED: &str = "disconnected";

    /// Server is unhealthy
    pub const STATUS_UNHEALTHY: &str = "unhealthy";

    /// Server is initializing
    pub const STATUS_INITIALIZING: &str = "initializing";
}

/// Goose-specific semantic conventions
pub mod goose {
    // =========================================================================
    // Session Attributes
    // =========================================================================

    /// Session identifier
    pub const SESSION_ID: &str = "goose.session.id";

    /// Session name
    pub const SESSION_NAME: &str = "goose.session.name";

    /// Session start time
    pub const SESSION_START_TIME: &str = "goose.session.start_time";

    // =========================================================================
    // Agent Attributes
    // =========================================================================

    /// Agent type (e.g., "default", "code", "security")
    pub const AGENT_TYPE: &str = "goose.agent.type";

    /// Agent version
    pub const AGENT_VERSION: &str = "goose.agent.version";

    // =========================================================================
    // Guardrails Attributes
    // =========================================================================

    /// Guardrail detector name
    pub const GUARDRAIL_DETECTOR: &str = "goose.guardrail.detector";

    /// Whether guardrail was triggered
    pub const GUARDRAIL_TRIGGERED: &str = "goose.guardrail.triggered";

    /// Guardrail detection confidence
    pub const GUARDRAIL_CONFIDENCE: &str = "goose.guardrail.confidence";

    /// Guardrail severity level
    pub const GUARDRAIL_SEVERITY: &str = "goose.guardrail.severity";

    // =========================================================================
    // Cost Attributes
    // =========================================================================

    /// Cumulative session cost in USD
    pub const SESSION_COST_USD: &str = "goose.session.cost_usd";

    /// Cost limit for the session
    pub const SESSION_COST_LIMIT_USD: &str = "goose.session.cost_limit_usd";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_ai_attributes() {
        assert_eq!(gen_ai::SYSTEM, "gen_ai.system");
        assert_eq!(gen_ai::REQUEST_MODEL, "gen_ai.request.model");
        assert_eq!(gen_ai::USAGE_INPUT_TOKENS, "gen_ai.usage.input_tokens");
        assert_eq!(gen_ai::USAGE_COST_USD, "gen_ai.usage.cost_usd");
    }

    #[test]
    fn test_mcp_attributes() {
        assert_eq!(mcp::SERVER_NAME, "mcp.server.name");
        assert_eq!(mcp::TOOL_COUNT, "mcp.tools.count");
        assert_eq!(mcp::TRANSPORT_TYPE, "mcp.transport.type");
    }

    #[test]
    fn test_goose_attributes() {
        assert_eq!(goose::SESSION_ID, "goose.session.id");
        assert_eq!(goose::GUARDRAIL_DETECTOR, "goose.guardrail.detector");
        assert_eq!(goose::SESSION_COST_USD, "goose.session.cost_usd");
    }

    #[test]
    fn test_known_values() {
        assert_eq!(gen_ai::SYSTEM_ANTHROPIC, "anthropic");
        assert_eq!(gen_ai::SYSTEM_OPENAI, "openai");
        assert_eq!(gen_ai::FINISH_REASON_STOP, "stop");
        assert_eq!(mcp::TRANSPORT_STDIO, "stdio");
        assert_eq!(mcp::STATUS_CONNECTED, "connected");
    }
}
