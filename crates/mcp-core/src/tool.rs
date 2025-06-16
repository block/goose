/// Tools represent a routine that a server can execute
/// Tool calls represent requests from the client to execute one
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Additional properties describing a tool to clients.
///
/// NOTE: all properties in ToolAnnotations are **hints**.
/// They are not guaranteed to provide a faithful description of
/// tool behavior (including descriptive properties like `title`).
///
/// Clients should never make tool use decisions based on ToolAnnotations
/// received from untrusted servers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    /// A human-readable title for the tool.
    pub title: Option<String>,

    /// If true, the tool does not modify its environment.
    ///
    /// Default: false
    #[serde(default)]
    pub read_only_hint: bool,

    /// If true, the tool may perform destructive updates to its environment.
    /// If false, the tool performs only additive updates.
    ///
    /// (This property is meaningful only when `read_only_hint == false`)
    ///
    /// Default: true
    #[serde(default = "default_true")]
    pub destructive_hint: bool,

    /// If true, calling the tool repeatedly with the same arguments
    /// will have no additional effect on its environment.
    ///
    /// (This property is meaningful only when `read_only_hint == false`)
    ///
    /// Default: false
    #[serde(default)]
    pub idempotent_hint: bool,

    /// If true, this tool may interact with an "open world" of external
    /// entities. If false, the tool's domain of interaction is closed.
    /// For example, the world of a web search tool is open, whereas that
    /// of a memory tool is not.
    ///
    /// Default: true
    #[serde(default = "default_true")]
    pub open_world_hint: bool,
}

impl Default for ToolAnnotations {
    fn default() -> Self {
        ToolAnnotations {
            title: None,
            read_only_hint: false,
            destructive_hint: true,
            idempotent_hint: false,
            open_world_hint: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Implement builder methods for `ToolAnnotations`
impl ToolAnnotations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only_hint = read_only;
        self
    }

    pub fn with_destructive(mut self, destructive: bool) -> Self {
        self.destructive_hint = destructive;
        self
    }

    pub fn with_idempotent(mut self, idempotent: bool) -> Self {
        self.idempotent_hint = idempotent;
        self
    }

    pub fn with_open_world(mut self, open_world: bool) -> Self {
        self.open_world_hint = open_world;
        self
    }
}

/// A tool that can be used by a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The name of the tool
    pub name: String,
    /// A description of what the tool does
    pub description: String,
    /// A JSON Schema object defining the expected parameters for the tool
    pub input_schema: Value,
    /// Optional additional tool information.
    pub annotations: Option<ToolAnnotations>,
}

impl Tool {
    /// Create a new tool with the given name and description
    pub fn new<N, D>(
        name: N,
        description: D,
        input_schema: Value,
        annotations: Option<ToolAnnotations>,
    ) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        Tool {
            name: name.into(),
            description: description.into(),
            input_schema,
            annotations,
        }
    }
}

/// A tool call request that an extension can execute
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    /// The name of the tool to execute
    pub name: String,
    /// The parameters for the execution
    pub arguments: Value,
}

impl ToolCall {
    /// Create a new ToolUse with the given name and parameters
    pub fn new<S: Into<String>>(name: S, arguments: Value) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}
