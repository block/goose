use std::collections::HashMap;

use mcp_client::client::Error as ClientError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from System operation
#[derive(Error, Debug)]
pub enum SystemError {
    #[error("Failed to start the MCP server from configuration `{0}` within 60 seconds")]
    Initialization(SystemConfig),
    #[error("Failed a client call to an MCP server: {0}")]
    Client(#[from] ClientError),
    #[error("Messages exceeded context-limit and could not be truncated to fit.")]
    ContextLimit,
    #[error("Transport error: {0}")]
    Transport(#[from] mcp_client::transport::Error),
}

pub type SystemResult<T> = Result<T, SystemError>;

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

/// Represents the different types of MCP systems that can be added to the manager
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum SystemConfig {
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
    /// Built-in system that is part of the goose binary
    #[serde(rename = "builtin")]
    Builtin { name: String },
}

impl SystemConfig {
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

impl std::fmt::Display for SystemConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemConfig::Sse { uri, .. } => write!(f, "SSE({})", uri),
            SystemConfig::Stdio { cmd, args, .. } => write!(f, "Stdio({} {})", cmd, args.join(" ")),
            SystemConfig::Builtin { name } => write!(f, "Builtin({})", name),
        }
    }
}

/// Information about the system used for building prompts
#[derive(Clone, Debug, Serialize)]
pub struct SystemInfo {
    name: String,
    description: String,
    instructions: String,
}

impl SystemInfo {
    pub fn new(name: &str, description: &str, instructions: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            instructions: instructions.to_string(),
        }
    }
}
