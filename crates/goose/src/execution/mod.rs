//! Unified execution management for Goose agents
//!
//! This module provides centralized agent lifecycle management with session isolation,
//! enabling multiple concurrent sessions with independent agents, extensions, and providers.

pub mod adapters;
pub mod manager;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Execution context that defines how an agent should be configured and run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Interactive chat sessions with user
    /// Future: streaming, confirmations, UI interactions
    Interactive,

    /// Background execution (scheduler, automated tasks)
    /// Future: retry config, scheduling metadata
    Background,

    /// Sub-task execution with parent context
    /// Future: approval bubbling, extension inheritance
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

/// Unique session identifier with room for future metadata
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    /// Generate a new unique session ID
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from an existing ID string
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Get the inner string value
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_helpers() {
        assert_eq!(ExecutionMode::chat(), ExecutionMode::Interactive);
        assert_eq!(ExecutionMode::scheduled(), ExecutionMode::Background);

        let parent = "parent-123".to_string();
        assert_eq!(
            ExecutionMode::task(parent.clone()),
            ExecutionMode::SubTask {
                parent_session: parent
            }
        );
    }

    #[test]
    fn test_session_id_generation() {
        let id1 = SessionId::generate();
        let id2 = SessionId::generate();

        // Should be unique
        assert_ne!(id1, id2);

        // Should be valid UUIDs
        assert_eq!(id1.0.len(), 36); // UUID string length
        assert_eq!(id2.0.len(), 36);
    }

    #[test]
    fn test_session_id_from_string() {
        let id_str = "test-session-123";
        let session_id = SessionId::from_string(id_str.to_string());
        assert_eq!(session_id.as_str(), id_str);

        // Test From trait
        let session_id2: SessionId = id_str.into();
        assert_eq!(session_id, session_id2);
    }

    #[test]
    fn test_display_traits() {
        let session = SessionId::from_string("display-test".to_string());
        assert_eq!(format!("{}", session), "display-test");

        let mode = ExecutionMode::Interactive;
        assert_eq!(format!("{}", mode), "interactive");

        let mode2 = ExecutionMode::task("parent-456".to_string());
        assert_eq!(format!("{}", mode2), "subtask(parent: parent-456)");
    }
}
