// Tool state management for sessions
// Provides a simple way to store tool-specific data with versioned keys

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Session data containing all tool states
/// Keys are in format "tool_name.version" (e.g., "todo.v0")
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct SessionData {
    #[serde(flatten)]
    pub tool_states: HashMap<String, Value>,
}

impl SessionData {
    /// Create a new empty SessionData
    pub fn new() -> Self {
        Self {
            tool_states: HashMap::new(),
        }
    }

    /// Get tool state for a specific tool and version
    pub fn get_tool_state(&self, tool_name: &str, version: &str) -> Option<&Value> {
        let key = format!("{}.{}", tool_name, version);
        self.tool_states.get(&key)
    }

    /// Set tool state for a specific tool and version
    pub fn set_tool_state(&mut self, tool_name: &str, version: &str, state: Value) {
        let key = format!("{}.{}", tool_name, version);
        self.tool_states.insert(key, state);
    }
}

/// Helper trait for tool-specific state management
pub trait ToolState: Sized + Serialize + for<'de> Deserialize<'de> {
    /// The name of the tool
    const TOOL_NAME: &'static str;

    /// The version of the tool state format
    const VERSION: &'static str;

    /// Convert from JSON value
    fn from_value(value: &Value) -> Result<Self> {
        serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize {} state: {}", Self::TOOL_NAME, e))
    }

    /// Convert to JSON value
    fn to_value(&self) -> Result<Value> {
        serde_json::to_value(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize {} state: {}", Self::TOOL_NAME, e))
    }

    /// Get state from session data
    fn from_session_data(session_data: &SessionData) -> Option<Self> {
        session_data
            .get_tool_state(Self::TOOL_NAME, Self::VERSION)
            .and_then(|v| Self::from_value(v).ok())
    }

    /// Save state to session data
    fn to_session_data(&self, session_data: &mut SessionData) -> Result<()> {
        let value = self.to_value()?;
        session_data.set_tool_state(Self::TOOL_NAME, Self::VERSION, value);
        Ok(())
    }
}

/// TODO tool state implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoState {
    pub content: String,
}

impl ToolState for TodoState {
    const TOOL_NAME: &'static str = "todo";
    const VERSION: &'static str = "v0";
}

impl TodoState {
    /// Create a new TODO state
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_session_data_basic_operations() {
        let mut session_data = SessionData::new();

        // Test setting and getting tool state
        let todo_state = json!({"content": "- Task 1\n- Task 2"});
        session_data.set_tool_state("todo", "v0", todo_state.clone());

        assert_eq!(session_data.get_tool_state("todo", "v0"), Some(&todo_state));
        assert_eq!(session_data.get_tool_state("todo", "v1"), None);
    }

    #[test]
    fn test_multiple_tool_states() {
        let mut session_data = SessionData::new();

        // Add multiple tool states
        session_data.set_tool_state("todo", "v0", json!("TODO content"));
        session_data.set_tool_state("memory", "v1", json!({"items": ["item1", "item2"]}));
        session_data.set_tool_state("config", "v2", json!({"setting": true}));

        // Check all states exist
        assert_eq!(session_data.tool_states.len(), 3);
        assert!(session_data.get_tool_state("todo", "v0").is_some());
        assert!(session_data.get_tool_state("memory", "v1").is_some());
        assert!(session_data.get_tool_state("config", "v2").is_some());
    }

    #[test]
    fn test_todo_state_trait() {
        let mut session_data = SessionData::new();

        // Create and save TODO state
        let todo = TodoState::new("- Task 1\n- Task 2".to_string());
        todo.to_session_data(&mut session_data).unwrap();

        // Retrieve TODO state
        let retrieved = TodoState::from_session_data(&session_data);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "- Task 1\n- Task 2");
    }

    #[test]
    fn test_session_data_serialization() {
        let mut session_data = SessionData::new();
        session_data.set_tool_state("todo", "v0", json!("TODO content"));
        session_data.set_tool_state("memory", "v1", json!({"key": "value"}));

        // Serialize to JSON
        let json = serde_json::to_value(&session_data).unwrap();

        // Check the structure
        assert!(json.is_object());
        assert_eq!(json.get("todo.v0"), Some(&json!("TODO content")));
        assert_eq!(json.get("memory.v1"), Some(&json!({"key": "value"})));

        // Deserialize back
        let deserialized: SessionData = serde_json::from_value(json).unwrap();
        assert_eq!(
            deserialized.get_tool_state("todo", "v0"),
            Some(&json!("TODO content"))
        );
        assert_eq!(
            deserialized.get_tool_state("memory", "v1"),
            Some(&json!({"key": "value"}))
        );
    }
}
