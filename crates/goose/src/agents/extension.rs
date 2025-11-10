use crate::agents::chatrecall_extension;
use crate::agents::extension_manager_extension;
use crate::agents::todo_extension;
use std::collections::HashMap;

use crate::agents::mcp_client::McpClientTrait;
use crate::config;
use crate::config::extensions::name_to_key;
use crate::config::permission::PermissionLevel;
use once_cell::sync::Lazy;
use rmcp::model::Tool;
use rmcp::service::ClientInitializeError;
use rmcp::ServiceError as ClientError;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;
use utoipa::ToSchema;

#[derive(Error, Debug)]
#[error("process quit before initialization: stderr = {stderr}")]
pub struct ProcessExit {
    stderr: String,
    #[source]
    source: ClientInitializeError,
}

impl ProcessExit {
    pub fn new<T>(stderr: T, source: ClientInitializeError) -> Self
    where
        T: Into<String>,
    {
        ProcessExit {
            stderr: stderr.into(),
            source,
        }
    }
}

pub static PLATFORM_EXTENSIONS: Lazy<HashMap<&'static str, PlatformExtensionDef>> = Lazy::new(
    || {
        let mut map = HashMap::new();

        map.insert(
            todo_extension::EXTENSION_NAME,
            PlatformExtensionDef {
                name: todo_extension::EXTENSION_NAME,
                description:
                    "Enable a todo list for Goose so it can keep track of what it is doing",
                default_enabled: true,
                client_factory: |ctx| Box::new(todo_extension::TodoClient::new(ctx).unwrap()),
            },
        );

        map.insert(
            chatrecall_extension::EXTENSION_NAME,
            PlatformExtensionDef {
                name: chatrecall_extension::EXTENSION_NAME,
                description:
                    "Search past conversations and load session summaries for contextual memory",
                default_enabled: false,
                client_factory: |ctx| {
                    Box::new(chatrecall_extension::ChatRecallClient::new(ctx).unwrap())
                },
            },
        );

        map.insert(
            "extensionmanager",
            PlatformExtensionDef {
                name: extension_manager_extension::EXTENSION_NAME,
                description:
                    "Enable extension management tools for discovering, enabling, and disabling extensions",
                default_enabled: true,
                client_factory: |ctx| Box::new(extension_manager_extension::ExtensionManagerClient::new(ctx).unwrap()),
            },
        );

        map
    },
);

#[derive(Clone)]
pub struct PlatformExtensionContext {
    pub session_id: Option<String>,
    pub extension_manager:
        Option<std::sync::Weak<crate::agents::extension_manager::ExtensionManager>>,
    pub tool_route_manager:
        Option<std::sync::Weak<crate::agents::tool_route_manager::ToolRouteManager>>,
}

#[derive(Debug, Clone)]
pub struct PlatformExtensionDef {
    pub name: &'static str,
    pub description: &'static str,
    pub default_enabled: bool,
    pub client_factory: fn(PlatformExtensionContext) -> Box<dyn McpClientTrait>,
}

/// Errors from Extension operation
#[derive(Error, Debug)]
pub enum ExtensionError {
    #[error("failed a client call to an MCP server: {0}")]
    Client(#[from] ClientError),
    #[error("invalid config: {0}")]
    ConfigError(String),
    #[error("error during extension setup: {0}")]
    SetupError(String),
    #[error("join error occurred during task execution: {0}")]
    TaskJoinError(#[from] tokio::task::JoinError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("failed to initialize MCP client: {0}")]
    InitializeError(#[from] ClientInitializeError),
    #[error("{0}")]
    ProcessExit(#[from] ProcessExit),
}

pub type ExtensionResult<T> = Result<T, ExtensionError>;

#[derive(Debug, Clone, Deserialize, Serialize, Default, ToSchema)]
pub struct Envs {
    /// A map of environment variables to set, e.g. API_KEY -> some_secret, HOST -> host
    #[serde(default)]
    #[serde(flatten)]
    map: HashMap<String, String>,
}

impl Envs {
    /// List of sensitive env vars that should not be overridden
    const DISALLOWED_KEYS: [&'static str; 31] = [
        // 🔧 Binary path manipulation
        "PATH",       // Controls executable lookup paths — critical for command hijacking
        "PATHEXT",    // Windows: Determines recognized executable extensions (e.g., .exe, .bat)
        "SystemRoot", // Windows: Can affect system DLL resolution (e.g., `kernel32.dll`)
        "windir",     // Windows: Alternative to SystemRoot (used in legacy apps)
        // 🧬 Dynamic linker hijacking (Linux/macOS)
        "LD_LIBRARY_PATH",  // Alters shared library resolution
        "LD_PRELOAD",       // Forces preloading of shared libraries — common attack vector
        "LD_AUDIT",         // Loads a monitoring library that can intercept execution
        "LD_DEBUG",         // Enables verbose linker logging (information disclosure risk)
        "LD_BIND_NOW",      // Forces immediate symbol resolution, affecting ASLR
        "LD_ASSUME_KERNEL", // Tricks linker into thinking it's running on an older kernel
        // 🍎 macOS dynamic linker variables
        "DYLD_LIBRARY_PATH",     // Same as LD_LIBRARY_PATH but for macOS
        "DYLD_INSERT_LIBRARIES", // macOS equivalent of LD_PRELOAD
        "DYLD_FRAMEWORK_PATH",   // Overrides framework lookup paths
        // 🐍 Python / Node / Ruby / Java / Golang hijacking
        "PYTHONPATH",   // Overrides Python module resolution
        "PYTHONHOME",   // Overrides Python root directory
        "NODE_OPTIONS", // Injects options/scripts into every Node.js process
        "RUBYOPT",      // Injects Ruby execution flags
        "GEM_PATH",     // Alters where RubyGems looks for installed packages
        "GEM_HOME",     // Changes RubyGems default install location
        "CLASSPATH",    // Java: Controls where classes are loaded from — critical for RCE attacks
        "GO111MODULE",  // Go: Forces use of module proxy or disables it
        "GOROOT", // Go: Changes root installation directory (could lead to execution hijacking)
        // 🖥️ Windows-specific process & DLL hijacking
        "APPINIT_DLLS", // Forces Windows to load a DLL into every process
        "SESSIONNAME",  // Affects Windows session configuration
        "ComSpec",      // Determines default command interpreter (can replace `cmd.exe`)
        "TEMP",
        "TMP",          // Redirects temporary file storage (useful for injection attacks)
        "LOCALAPPDATA", // Controls application data paths (can be abused for persistence)
        "USERPROFILE",  // Windows user directory (can affect profile-based execution paths)
        "HOMEDRIVE",
        "HOMEPATH", // Changes where the user's home directory is located
    ];

    /// Constructs a new Envs, skipping disallowed env vars with a warning
    pub fn new(map: HashMap<String, String>) -> Self {
        let mut validated = HashMap::new();

        for (key, value) in map {
            if Self::is_disallowed(&key) {
                warn!("Skipping disallowed env var: {}", key);
                continue;
            }
            validated.insert(key, value);
        }

        Self { map: validated }
    }

    /// Returns a copy of the validated env vars
    pub fn get_env(&self) -> HashMap<String, String> {
        self.map.clone()
    }

    /// Returns an error if any disallowed env var is present
    pub fn validate(&self) -> Result<(), Box<ExtensionError>> {
        for key in self.map.keys() {
            if Self::is_disallowed(key) {
                return Err(Box::new(ExtensionError::ConfigError(format!(
                    "environment variable {} not allowed to be overwritten",
                    key
                ))));
            }
        }
        Ok(())
    }

    fn is_disallowed(key: &str) -> bool {
        Self::DISALLOWED_KEYS
            .iter()
            .any(|disallowed| disallowed.eq_ignore_ascii_case(key))
    }
}

/// Represents the different types of MCP extensions that can be added to the manager
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(tag = "type")]
pub enum ExtensionConfig {
    /// Server-sent events client with a URI endpoint
    #[serde(rename = "sse")]
    Sse {
        /// The name used to identify this extension
        name: String,
        #[serde(default)]
        #[serde(deserialize_with = "deserialize_null_with_default")]
        #[schema(required)]
        description: String,
        uri: String,
        #[serde(default)]
        envs: Envs,
        #[serde(default)]
        env_keys: Vec<String>,
        // NOTE: set timeout to be optional for compatibility.
        // However, new configurations should include this field.
        timeout: Option<u64>,
        #[serde(default)]
        bundled: Option<bool>,
        #[serde(default)]
        available_tools: Vec<String>,
    },
    /// Standard I/O client with command and arguments
    #[serde(rename = "stdio")]
    Stdio {
        /// The name used to identify this extension
        name: String,
        #[serde(default)]
        #[serde(deserialize_with = "deserialize_null_with_default")]
        #[schema(required)]
        description: String,
        cmd: String,
        args: Vec<String>,
        #[serde(default)]
        envs: Envs,
        #[serde(default)]
        env_keys: Vec<String>,
        timeout: Option<u64>,
        #[serde(default)]
        bundled: Option<bool>,
        #[serde(default)]
        available_tools: Vec<String>,
    },
    /// Built-in extension that is part of the bundled goose MCP server
    #[serde(rename = "builtin")]
    Builtin {
        /// The name used to identify this extension
        name: String,
        #[serde(default)]
        #[serde(deserialize_with = "deserialize_null_with_default")]
        #[schema(required)]
        description: String,
        display_name: Option<String>, // needed for the UI
        timeout: Option<u64>,
        #[serde(default)]
        bundled: Option<bool>,
        #[serde(default)]
        available_tools: Vec<String>,
    },
    /// Platform extensions that have direct access to the agent etc and run in the agent process
    #[serde(rename = "platform")]
    Platform {
        /// The name used to identify this extension
        name: String,
        #[serde(deserialize_with = "deserialize_null_with_default")]
        #[schema(required)]
        description: String,
        #[serde(default)]
        bundled: Option<bool>,
        #[serde(default)]
        available_tools: Vec<String>,
    },
    /// Streamable HTTP client with a URI endpoint using MCP Streamable HTTP specification
    #[serde(rename = "streamable_http")]
    StreamableHttp {
        /// The name used to identify this extension
        name: String,
        #[serde(deserialize_with = "deserialize_null_with_default")]
        #[schema(required)]
        description: String,
        uri: String,
        #[serde(default)]
        envs: Envs,
        #[serde(default)]
        env_keys: Vec<String>,
        #[serde(default)]
        headers: HashMap<String, String>,
        // NOTE: set timeout to be optional for compatibility.
        // However, new configurations should include this field.
        timeout: Option<u64>,
        #[serde(default)]
        bundled: Option<bool>,
        #[serde(default)]
        available_tools: Vec<String>,
    },
    /// Frontend-provided tools that will be called through the frontend
    #[serde(rename = "frontend")]
    Frontend {
        /// The name used to identify this extension
        name: String,
        #[serde(deserialize_with = "deserialize_null_with_default")]
        #[schema(required)]
        description: String,
        /// The tools provided by the frontend
        tools: Vec<Tool>,
        /// Instructions for how to use these tools
        instructions: Option<String>,
        #[serde(default)]
        bundled: Option<bool>,
        #[serde(default)]
        available_tools: Vec<String>,
    },
    /// Inline Python code that will be executed using uvx
    #[serde(rename = "inline_python")]
    InlinePython {
        /// The name used to identify this extension
        name: String,
        #[serde(deserialize_with = "deserialize_null_with_default")]
        #[schema(required)]
        description: String,
        /// The Python code to execute
        code: String,
        /// Timeout in seconds
        timeout: Option<u64>,
        /// Python package dependencies required by this extension
        #[serde(default)]
        dependencies: Option<Vec<String>>,
        #[serde(default)]
        available_tools: Vec<String>,
    },
}

impl Default for ExtensionConfig {
    fn default() -> Self {
        Self::Builtin {
            name: config::DEFAULT_EXTENSION.to_string(),
            display_name: Some(config::DEFAULT_DISPLAY_NAME.to_string()),
            description: "default".to_string(),
            timeout: Some(config::DEFAULT_EXTENSION_TIMEOUT),
            bundled: Some(true),
            available_tools: Vec::new(),
        }
    }
}

impl ExtensionConfig {
    pub fn sse<S: Into<String>, T: Into<u64>>(name: S, uri: S, description: S, timeout: T) -> Self {
        Self::Sse {
            name: name.into(),
            uri: uri.into(),
            envs: Envs::default(),
            env_keys: Vec::new(),
            description: description.into(),
            timeout: Some(timeout.into()),
            bundled: None,
            available_tools: Vec::new(),
        }
    }

    pub fn streamable_http<S: Into<String>, T: Into<u64>>(
        name: S,
        uri: S,
        description: S,
        timeout: T,
    ) -> Self {
        Self::StreamableHttp {
            name: name.into(),
            uri: uri.into(),
            envs: Envs::default(),
            env_keys: Vec::new(),
            headers: HashMap::new(),
            description: description.into(),
            timeout: Some(timeout.into()),
            bundled: None,
            available_tools: Vec::new(),
        }
    }

    pub fn stdio<S: Into<String>, T: Into<u64>>(
        name: S,
        cmd: S,
        description: S,
        timeout: T,
    ) -> Self {
        Self::Stdio {
            name: name.into(),
            cmd: cmd.into(),
            args: vec![],
            envs: Envs::default(),
            env_keys: Vec::new(),
            description: description.into(),
            timeout: Some(timeout.into()),
            bundled: None,
            available_tools: Vec::new(),
        }
    }

    pub fn inline_python<S: Into<String>, T: Into<u64>>(
        name: S,
        code: S,
        description: S,
        timeout: T,
    ) -> Self {
        Self::InlinePython {
            name: name.into(),
            code: code.into(),
            description: description.into(),
            timeout: Some(timeout.into()),
            dependencies: None,
            available_tools: Vec::new(),
        }
    }

    pub fn with_args<I, S>(self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        match self {
            Self::Stdio {
                name,
                cmd,
                envs,
                env_keys,
                timeout,
                description,
                bundled,
                available_tools,
                ..
            } => Self::Stdio {
                name,
                cmd,
                envs,
                env_keys,
                args: args.into_iter().map(Into::into).collect(),
                description,
                timeout,
                bundled,
                available_tools,
            },
            other => other,
        }
    }

    pub fn key(&self) -> String {
        let name = self.name();
        name_to_key(&name)
    }

    /// Get the extension name regardless of variant
    pub fn name(&self) -> String {
        match self {
            Self::Sse { name, .. } => name,
            Self::StreamableHttp { name, .. } => name,
            Self::Stdio { name, .. } => name,
            Self::Builtin { name, .. } => name,
            Self::Platform { name, .. } => name,
            Self::Frontend { name, .. } => name,
            Self::InlinePython { name, .. } => name,
        }
        .to_string()
    }

    /// Check if a tool should be available to the LLM
    pub fn is_tool_available(&self, tool_name: &str) -> bool {
        let available_tools = match self {
            Self::Sse {
                available_tools, ..
            }
            | Self::StreamableHttp {
                available_tools, ..
            }
            | Self::Stdio {
                available_tools, ..
            }
            | Self::Builtin {
                available_tools, ..
            }
            | Self::Platform {
                available_tools, ..
            }
            | Self::InlinePython {
                available_tools, ..
            }
            | Self::Frontend {
                available_tools, ..
            } => available_tools,
        };

        // If no tools are specified, all tools are available
        // If tools are specified, only those tools are available
        available_tools.is_empty() || available_tools.contains(&tool_name.to_string())
    }
}

impl std::fmt::Display for ExtensionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtensionConfig::Sse { name, uri, .. } => write!(f, "SSE({}: {})", name, uri),
            ExtensionConfig::StreamableHttp { name, uri, .. } => {
                write!(f, "StreamableHttp({}: {})", name, uri)
            }
            ExtensionConfig::Stdio {
                name, cmd, args, ..
            } => {
                write!(f, "Stdio({}: {} {})", name, cmd, args.join(" "))
            }
            ExtensionConfig::Builtin { name, .. } => write!(f, "Builtin({})", name),
            ExtensionConfig::Platform { name, .. } => write!(f, "Platform({})", name),
            ExtensionConfig::Frontend { name, tools, .. } => {
                write!(f, "Frontend({}: {} tools)", name, tools.len())
            }
            ExtensionConfig::InlinePython { name, code, .. } => {
                write!(f, "InlinePython({}: {} chars)", name, code.len())
            }
        }
    }
}

/// Information about the extension used for building prompts
#[derive(Clone, Debug, Serialize)]
pub struct ExtensionInfo {
    pub name: String,
    pub instructions: String,
    pub has_resources: bool,
}

impl ExtensionInfo {
    pub fn new(name: &str, instructions: &str, has_resources: bool) -> Self {
        Self {
            name: name.to_string(),
            instructions: instructions.to_string(),
            has_resources,
        }
    }
}

fn deserialize_null_with_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Information about the tool used for building prompts
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
    pub permission: Option<PermissionLevel>,
}

impl ToolInfo {
    pub fn new(
        name: &str,
        description: &str,
        parameters: Vec<String>,
        permission: Option<PermissionLevel>,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
            permission,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::agents::*;

    #[test]
    fn test_deserialize_missing_description() {
        let config: ExtensionConfig = serde_yaml::from_str(
            "enabled: true
type: builtin
name: developer
display_name: Developer
timeout: 300
bundled: true
available_tools: []",
        )
        .unwrap();
        if let ExtensionConfig::Builtin { description, .. } = config {
            assert_eq!(description, "")
        } else {
            panic!("unexpected result of deserialization: {}", config)
        }
    }

    #[test]
    fn test_deserialize_null_description() {
        let config: ExtensionConfig = serde_yaml::from_str(
            "enabled: true
type: builtin
name: developer
display_name: Developer
description: null
timeout: 300
bundled: true
available_tools: []
",
        )
        .unwrap();
        if let ExtensionConfig::Builtin { description, .. } = config {
            assert_eq!(description, "")
        } else {
            panic!("unexpected result of deserialization: {}", config)
        }
    }

    #[test]
    fn test_deserialize_normal_description() {
        let config: ExtensionConfig = serde_yaml::from_str(
            "enabled: true
type: builtin
name: developer
display_name: Developer
description: description goes here
timeout: 300
bundled: true
available_tools: []
    ",
        )
        .unwrap();
        if let ExtensionConfig::Builtin { description, .. } = config {
            assert_eq!(description, "description goes here")
        } else {
            panic!("unexpected result of deserialization: {}", config)
        }
    }
}
