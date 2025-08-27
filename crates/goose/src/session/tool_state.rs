// Tool state management for sessions
// Provides a versioned, extensible way to store tool-specific data in sessions

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Version identifier for tool state formats
pub type ToolVersion = String;

/// Tool state key format: "tool_name.version"
/// Example: "todo.v0", "memory.v1"
pub type ToolStateKey = String;

/// Session data containing all tool states
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionData {
    /// Map of tool state keys to their data
    /// Keys are in format "tool_name.version" (e.g., "todo.v0")
    #[serde(flatten)]
    pub tool_states: HashMap<ToolStateKey, Value>,
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

    /// Check if a tool state exists
    pub fn has_tool_state(&self, tool_name: &str, version: &str) -> bool {
        let key = format!("{}.{}", tool_name, version);
        self.tool_states.contains_key(&key)
    }

    /// Get all tool state keys
    pub fn tool_state_keys(&self) -> Vec<ToolStateKey> {
        self.tool_states.keys().cloned().collect()
    }

    /// Clear all tool states
    pub fn clear(&mut self) {
        self.tool_states.clear();
    }
}

/// Tool state registry for managing tool versions and parsers
pub struct ToolStateRegistry {
    /// Map of tool names to their current version and parser functions
    parsers: HashMap<String, (ToolVersion, ToolParserFn)>,
}

/// Type alias for tool parser functions
type ToolParserFn = Box<dyn Fn(&Value) -> Result<Value> + Send + Sync>;

impl Default for ToolStateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolStateRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: HashMap::new(),
        };

        // Register default tools
        registry.register_default_tools();
        registry
    }

    /// Register default tool parsers
    fn register_default_tools(&mut self) {
        // Register TODO tool v0 parser
        self.register_tool(
            "todo".to_string(),
            "v0".to_string(),
            Box::new(|value| {
                // For todo v0, we expect a simple string
                if let Some(s) = value.as_str() {
                    Ok(Value::String(s.to_string()))
                } else {
                    // Try to convert to string if it's another type
                    Ok(Value::String(value.to_string()))
                }
            }),
        );
    }

    /// Register a tool with its version and parser
    pub fn register_tool(&mut self, tool_name: String, version: ToolVersion, parser: ToolParserFn) {
        self.parsers.insert(tool_name, (version, parser));
    }

    /// Get the current version for a tool
    pub fn get_tool_version(&self, tool_name: &str) -> Option<&ToolVersion> {
        self.parsers.get(tool_name).map(|(v, _)| v)
    }

    /// Parse tool state using the registered parser
    pub fn parse_tool_state(&self, tool_name: &str, value: &Value) -> Result<Value> {
        if let Some((_, parser)) = self.parsers.get(tool_name) {
            parser(value)
        } else {
            // If no parser registered, return as-is
            Ok(value.clone())
        }
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

    /// Create an empty TODO state
    pub fn empty() -> Self {
        Self {
            content: String::new(),
        }
    }
}

// Global registry instance
lazy_static::lazy_static! {
    static ref TOOL_STATE_REGISTRY: ToolStateRegistry = ToolStateRegistry::new();
}

/// Get the global tool state registry
pub fn registry() -> &'static ToolStateRegistry {
    &TOOL_STATE_REGISTRY
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

        assert!(session_data.has_tool_state("todo", "v0"));
        assert_eq!(session_data.get_tool_state("todo", "v0"), Some(&todo_state));
    }

    #[test]
    fn test_multiple_tool_states() {
        let mut session_data = SessionData::new();

        // Add multiple tool states
        session_data.set_tool_state("todo", "v0", json!("TODO content"));
        session_data.set_tool_state("memory", "v1", json!({"items": ["item1", "item2"]}));
        session_data.set_tool_state("config", "v2", json!({"setting": true}));

        // Check all states exist
        assert_eq!(session_data.tool_state_keys().len(), 3);
        assert!(session_data.has_tool_state("todo", "v0"));
        assert!(session_data.has_tool_state("memory", "v1"));
        assert!(session_data.has_tool_state("config", "v2"));

        // Clear all states
        session_data.clear();
        assert_eq!(session_data.tool_state_keys().len(), 0);
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
    fn test_tool_state_registry() {
        let registry = ToolStateRegistry::new();

        // Check TODO tool is registered
        assert_eq!(registry.get_tool_version("todo"), Some(&"v0".to_string()));

        // Test parsing TODO state
        let value = json!("My TODO content");
        let parsed = registry.parse_tool_state("todo", &value).unwrap();
        assert_eq!(parsed, json!("My TODO content"));
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
