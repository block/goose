//! Unified execution management for Goose agents
//!
//! This module provides centralized agent lifecycle management with session isolation,
//! enabling multiple concurrent sessions with independent agents, extensions, and providers.

pub mod manager;

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionMode {
    Interactive,
    Background,
    SubTask { parent_session: String },
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
