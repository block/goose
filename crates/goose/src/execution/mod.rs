//! Unified execution management for Goose agents
//!
//! This module provides centralized agent lifecycle management with session isolation,
//! enabling multiple concurrent sessions with independent agents, extensions, and providers.

pub mod manager;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Execution context that defines how an agent should be configured and run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionMode {
    Interactive,
    Background,
    SubTask {
        /// Parent session that spawned this task
        parent_session: String,
    },
}

impl ExecutionMode {
    /// Create an interactive chat mode
    pub fn chat() -> Self {
        Self::Interactive
    }

    /// Create a background/scheduled mode
    pub fn scheduled() -> Self {
        Self::Background
    }

    /// Create a sub-task mode with parent reference
    pub fn task(parent: String) -> Self {
        Self::SubTask {
            parent_session: parent,
        }
    }
}

impl fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interactive => write!(f, "interactive"),
            Self::Background => write!(f, "background"),
            Self::SubTask { parent_session } => write!(f, "subtask(parent: {})", parent_session),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
