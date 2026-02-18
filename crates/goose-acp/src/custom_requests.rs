use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Schema metadata for custom methods
// ---------------------------------------------------------------------------

/// Schema descriptor for a single custom method, produced by the
/// `#[custom_methods]` macro's generated `custom_method_schemas()` function.
///
/// `params_schema` / `response_schema` hold `$ref` pointers or inline schemas
/// produced by `SchemaGenerator::subschema_for`. All referenced types are
/// collected in the generator's `$defs` map.
///
/// `params_type_name` / `response_type_name` carry the Rust struct name so the
/// binary can key `$defs` entries and annotate them with `x-method` / `x-side`.
#[derive(Debug, Serialize)]
pub struct CustomMethodSchema {
    pub method: String,
    pub params_schema: Option<schemars::Schema>,
    pub params_type_name: Option<String>,
    pub response_schema: Option<schemars::Schema>,
    pub response_type_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Agent: extensions
// ---------------------------------------------------------------------------

/// Add an extension to an active session.
/// Method: `_agent/extensions/add`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddExtensionRequest {
    pub session_id: String,
    /// Extension configuration (see ExtensionConfig variants: Stdio, StreamableHttp, Builtin, Platform).
    pub config: serde_json::Value,
}

/// Remove an extension from an active session.
/// Method: `_agent/extensions/remove`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveExtensionRequest {
    pub session_id: String,
    pub name: String,
}

// ---------------------------------------------------------------------------
// Agent: tools
// ---------------------------------------------------------------------------

/// List all tools available in a session.
/// Method: `_agent/tools`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetToolsRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetToolsResponse {
    /// Array of tool info objects with `name`, `description`, `parameters`, and optional `permission`.
    pub tools: Vec<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Agent: resource
// ---------------------------------------------------------------------------

/// Read a resource from an extension.
/// Method: `_agent/resource/read`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadResourceRequest {
    pub session_id: String,
    pub uri: String,
    pub extension_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ReadResourceResponse {
    /// The resource result from the extension (MCP ReadResourceResult).
    pub result: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Agent: working directory
// ---------------------------------------------------------------------------

/// Update the working directory for a session.
/// Method: `_agent/working_dir/update`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateWorkingDirRequest {
    pub session_id: String,
    pub working_dir: String,
}

// ---------------------------------------------------------------------------
// Session management
// ---------------------------------------------------------------------------

/// Get a session by ID.
/// Method: `_session/get`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSessionRequest {
    pub session_id: String,
    #[serde(default)]
    pub include_messages: bool,
}

/// Get a session response.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GetSessionResponse {
    /// The session object with id, name, working_dir, timestamps, tokens, etc.
    pub session: serde_json::Value,
}

/// List all sessions.
/// Method: `_session/list`
#[derive(Debug, Serialize, JsonSchema)]
pub struct ListSessionsResponse {
    pub sessions: Vec<serde_json::Value>,
}

/// Delete a session.
/// Method: `_session/delete`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteSessionRequest {
    pub session_id: String,
}

/// Export a session as a JSON string.
/// Method: `_session/export`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ExportSessionResponse {
    pub data: String,
}

/// Import a session from a JSON string.
/// Method: `_session/import`
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportSessionRequest {
    pub data: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ImportSessionResponse {
    /// The imported session object.
    pub session: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// List configured extensions and any warnings.
/// Method: `_config/extensions`
#[derive(Debug, Serialize, JsonSchema)]
pub struct GetExtensionsResponse {
    /// Array of ExtensionEntry objects with `enabled` flag and config details.
    pub extensions: Vec<serde_json::Value>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Shared empty response
// ---------------------------------------------------------------------------

/// Empty success response for operations that return no data.
#[derive(Debug, Serialize, JsonSchema)]
pub struct EmptyResponse {}
