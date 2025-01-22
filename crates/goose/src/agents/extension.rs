use std::collections::HashMap;

use mcp_client::client::Error as ClientError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from Extension operation
#[derive(Error, Debug)]
pub enum ExtensionError {
    #[error("Failed to start the MCP server from configuration `{0}` within 60 seconds")]
    Initialization(ExtensionConfig),
    #[error("Failed a client call to an MCP server: {0}")]
    Client(#[from] ClientError),
    #[error("User Message exceeded context-limit. History could not be truncated to accomodate.")]
    ContextLimit,
    #[error("Transport error: {0}")]
    Transport(#[from] mcp_client::transport::Error),
}

pub type ExtensionResult<T> = Result<T, ExtensionError>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Envs {
    /// A map of environment variables to set, e.g. API_KEY -> some_secret, HOST -> host
    #[serde(default)]
    #[serde(flatten)]
    map: HashMap<String, String>,
}

impl Envs {
    pub fn new(map: HashMap<String, String>) -> Self {
        Self { map }
    }

    pub fn get_env(&self) -> HashMap<String, String> {
        self.map
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
}

/// Represents the different types of MCP extensions that can be added to the manager
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ExtensionConfig {
    /// Server-sent events client with a URI endpoint
    #[serde(rename = "sse")]
    Sse {
        uri: String,
        #[serde(default)]
        envs: Envs,
    },
    /// Standard I/O client with command and arguments
    #[serde(rename = "stdio")]
    Stdio {
        cmd: String,
        args: Vec<String>,
        #[serde(default)]
        envs: Envs,
    },
    /// Built-in extension that is part of the goose binary
    #[serde(rename = "builtin")]
    Builtin { name: String },
}

impl Default for ExtensionConfig {
    fn default() -> Self {
        Self::Builtin {
            name: String::from("default"),
        }
    }
}

impl ExtensionConfig {
    pub fn sse<S: Into<String>>(uri: S) -> Self {
        Self::Sse {
            uri: uri.into(),
            envs: Envs::default(),
        }
    }

    pub fn stdio<S: Into<String>>(cmd: S) -> Self {
        Self::Stdio {
            cmd: cmd.into(),
            args: vec![],
            envs: Envs::default(),
        }
    }

    pub fn with_args<I, S>(self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        match self {
            Self::Stdio { cmd, envs, .. } => Self::Stdio {
                cmd,
                envs,
                args: args.into_iter().map(Into::into).collect(),
            },
            other => other,
        }
    }
}

impl std::fmt::Display for ExtensionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtensionConfig::Sse { uri, .. } => write!(f, "SSE({})", uri),
            ExtensionConfig::Stdio { cmd, args, .. } => {
                write!(f, "Stdio({} {})", cmd, args.join(" "))
            }
            ExtensionConfig::Builtin { name } => write!(f, "Builtin({})", name),
        }
    }
}

/// Information about the extension used for building prompts
#[derive(Clone, Debug, Serialize)]
pub struct ExtensionInfo {
    name: String,
    instructions: String,
    has_resources: bool,
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
