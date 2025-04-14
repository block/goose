
/// Represents the different types of MCP extensions that can be added to the manager
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(tag = "type")]
pub enum ExtensionConfig {
    /// Server-sent events client with a URI endpoint
    #[serde(rename = "sse")]
    Sse {
        /// The name used to identify this extension
        name: String,
        uri: String,
        #[serde(default)]
        envs: Envs,
        description: Option<String>,
        // NOTE: set timeout to be optional for compatibility.
        // However, new configurations should include this field.
        timeout: Option<u64>,
        /// Whether this extension is bundled with Goose
        #[serde(default)]
        bundled: Option<bool>,
    },
    /// Standard I/O client with command and arguments
    #[serde(rename = "stdio")]
    Stdio {
        /// The name used to identify this extension
        name: String,
        cmd: String,
        args: Vec<String>,
        #[serde(default)]
        envs: Envs,
        timeout: Option<u64>,
        description: Option<String>,
        /// Whether this extension is bundled with Goose
        #[serde(default)]
        bundled: Option<bool>,
    },
    /// Built-in extension that is part of the goose binary
    #[serde(rename = "builtin")]
    Builtin {
        /// The name used to identify this extension
        name: String,
        display_name: Option<String>, // needed for the UI
        timeout: Option<u64>,
        /// Whether this extension is bundled with Goose
        #[serde(default)]
        bundled: Option<bool>,
    },
    /// Frontend-provided tools that will be called through the frontend
    #[serde(rename = "frontend")]
    Frontend {
        /// The name used to identify this extension
        name: String,
        /// The tools provided by the frontend
        tools: Vec<Tool>,
        /// Instructions for how to use these tools
        instructions: Option<String>,
        /// Whether this extension is bundled with Goose
        #[serde(default)]
        bundled: Option<bool>,
    },
}