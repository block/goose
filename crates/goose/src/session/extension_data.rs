// Extension data management for sessions
// Provides a simple way to store extension-specific data with versioned keys

use crate::config::ExtensionConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Extension data containing all extension states
/// Keys are in format "extension_name.version" (e.g., "todo.v0")
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct ExtensionData {
    #[serde(flatten)]
    pub extension_states: HashMap<String, Value>,
}

impl ExtensionData {
    /// Create a new empty ExtensionData
    pub fn new() -> Self {
        Self {
            extension_states: HashMap::new(),
        }
    }

    /// Get extension state for a specific extension and version
    pub fn get_extension_state(&self, extension_name: &str, version: &str) -> Option<&Value> {
        let key = format!("{}.{}", extension_name, version);
        self.extension_states.get(&key)
    }

    /// Set extension state for a specific extension and version
    pub fn set_extension_state(&mut self, extension_name: &str, version: &str, state: Value) {
        let key = format!("{}.{}", extension_name, version);
        self.extension_states.insert(key, state);
    }
}

/// Helper trait for extension-specific state management
pub trait ExtensionState: Sized + Serialize + for<'de> Deserialize<'de> {
    /// The name of the extension
    const EXTENSION_NAME: &'static str;

    /// The version of the extension state format
    const VERSION: &'static str;

    /// Convert from JSON value
    fn from_value(value: &Value) -> Result<Self> {
        serde_json::from_value(value.clone()).map_err(|e| {
            anyhow::anyhow!(
                "Failed to deserialize {} state: {}",
                Self::EXTENSION_NAME,
                e
            )
        })
    }

    /// Convert to JSON value
    fn to_value(&self) -> Result<Value> {
        serde_json::to_value(self).map_err(|e| {
            anyhow::anyhow!("Failed to serialize {} state: {}", Self::EXTENSION_NAME, e)
        })
    }

    /// Get state from extension data
    fn from_extension_data(extension_data: &ExtensionData) -> Option<Self> {
        extension_data
            .get_extension_state(Self::EXTENSION_NAME, Self::VERSION)
            .and_then(|v| Self::from_value(v).ok())
    }

    /// Save state to extension data
    fn to_extension_data(&self, extension_data: &mut ExtensionData) -> Result<()> {
        let value = self.to_value()?;
        extension_data.set_extension_state(Self::EXTENSION_NAME, Self::VERSION, value);
        Ok(())
    }
}

/// TODO extension state implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoState {
    pub content: String,
}

impl ExtensionState for TodoState {
    const EXTENSION_NAME: &'static str = "todo";
    const VERSION: &'static str = "v0";
}

impl TodoState {
    /// Create a new TODO state
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

/// Enabled extensions state implementation for storing which extensions are active
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnabledExtensionsState {
    pub extensions: Vec<ExtensionConfig>,
}

impl ExtensionState for EnabledExtensionsState {
    const EXTENSION_NAME: &'static str = "enabled_extensions";
    const VERSION: &'static str = "v0";
}

impl EnabledExtensionsState {
    pub fn new(extensions: Vec<ExtensionConfig>) -> Self {
        Self { extensions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extension_data_basic_operations() {
        let mut extension_data = ExtensionData::new();

        // Test setting and getting extension state
        let todo_state = json!({"content": "- Task 1\n- Task 2"});
        extension_data.set_extension_state("todo", "v0", todo_state.clone());

        assert_eq!(
            extension_data.get_extension_state("todo", "v0"),
            Some(&todo_state)
        );
        assert_eq!(extension_data.get_extension_state("todo", "v1"), None);
    }

    #[test]
    fn test_multiple_extension_states() {
        let mut extension_data = ExtensionData::new();

        // Add multiple extension states
        extension_data.set_extension_state("todo", "v0", json!("TODO content"));
        extension_data.set_extension_state("memory", "v1", json!({"items": ["item1", "item2"]}));
        extension_data.set_extension_state("config", "v2", json!({"setting": true}));

        // Check all states exist
        assert_eq!(extension_data.extension_states.len(), 3);
        assert!(extension_data.get_extension_state("todo", "v0").is_some());
        assert!(extension_data.get_extension_state("memory", "v1").is_some());
        assert!(extension_data.get_extension_state("config", "v2").is_some());
    }

    #[test]
    fn test_todo_state_trait() {
        let mut extension_data = ExtensionData::new();

        // Create and save TODO state
        let todo = TodoState::new("- Task 1\n- Task 2".to_string());
        todo.to_extension_data(&mut extension_data).unwrap();

        // Retrieve TODO state
        let retrieved = TodoState::from_extension_data(&extension_data);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "- Task 1\n- Task 2");
    }

    #[test]
    fn test_extension_data_serialization() {
        let mut extension_data = ExtensionData::new();
        extension_data.set_extension_state("todo", "v0", json!("TODO content"));
        extension_data.set_extension_state("memory", "v1", json!({"key": "value"}));

        // Serialize to JSON
        let json = serde_json::to_value(&extension_data).unwrap();

        // Check the structure
        assert!(json.is_object());
        assert_eq!(json.get("todo.v0"), Some(&json!("TODO content")));
        assert_eq!(json.get("memory.v1"), Some(&json!({"key": "value"})));

        // Deserialize back
        let deserialized: ExtensionData = serde_json::from_value(json).unwrap();
        assert_eq!(
            deserialized.get_extension_state("todo", "v0"),
            Some(&json!("TODO content"))
        );
        assert_eq!(
            deserialized.get_extension_state("memory", "v1"),
            Some(&json!({"key": "value"}))
        );
    }

    #[test]
    fn test_enabled_extensions_state_with_full_configs() {
        use crate::agents::extension::Envs;
        use std::collections::HashMap;

        // Create multiple ExtensionConfig objects with different types
        let configs = vec![
            ExtensionConfig::Builtin {
                name: "developer".to_string(),
                display_name: Some("Developer Tools".to_string()),
                description: Some("Built-in developer extension".to_string()),
                timeout: Some(30),
                bundled: Some(true),
                available_tools: vec!["read_file".to_string(), "write_file".to_string()],
            },
            ExtensionConfig::Stdio {
                name: "custom_mcp".to_string(),
                cmd: "python".to_string(),
                args: vec!["-m".to_string(), "mcp_server".to_string()],
                envs: {
                    let mut map = HashMap::new();
                    map.insert("API_KEY".to_string(), "test123".to_string());
                    Envs::new(map)
                },
                env_keys: vec!["API_KEY".to_string()],
                timeout: Some(60),
                description: Some("Custom MCP server".to_string()),
                bundled: Some(false),
                available_tools: vec!["custom_tool".to_string()],
            },
        ];

        // Create EnabledExtensionsState
        let state = EnabledExtensionsState::new(configs.clone());

        // Verify basic properties
        assert_eq!(state.extensions.len(), 2);
        assert_eq!(state.extensions[0].name(), "developer");
        assert_eq!(state.extensions[1].name(), "custom_mcp");

        // Test round-trip serialization through ExtensionData
        let mut data = ExtensionData::default();
        state.to_extension_data(&mut data).unwrap();

        // Verify the state was saved
        assert!(data
            .get_extension_state("enabled_extensions", "v0")
            .is_some());

        // Restore from ExtensionData
        let restored = EnabledExtensionsState::from_extension_data(&data).unwrap();

        // Verify all extensions were restored
        assert_eq!(restored.extensions.len(), 2);

        // Verify first extension (Builtin) details preserved
        match &restored.extensions[0] {
            ExtensionConfig::Builtin {
                name,
                display_name,
                description,
                timeout,
                bundled,
                available_tools,
            } => {
                assert_eq!(name, "developer");
                assert_eq!(display_name, &Some("Developer Tools".to_string()));
                assert_eq!(
                    description,
                    &Some("Built-in developer extension".to_string())
                );
                assert_eq!(timeout, &Some(30));
                assert_eq!(bundled, &Some(true));
                assert_eq!(available_tools.len(), 2);
                assert_eq!(available_tools[0], "read_file");
            }
            _ => panic!("Expected Builtin variant"),
        }

        // Verify second extension (Stdio) details preserved
        match &restored.extensions[1] {
            ExtensionConfig::Stdio {
                name,
                cmd,
                args,
                envs,
                env_keys,
                timeout,
                description,
                bundled,
                available_tools,
            } => {
                assert_eq!(name, "custom_mcp");
                assert_eq!(cmd, "python");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], "-m");
                assert_eq!(envs.get_env().get("API_KEY"), Some(&"test123".to_string()));
                assert_eq!(env_keys[0], "API_KEY");
                assert_eq!(timeout, &Some(60));
                assert_eq!(description, &Some("Custom MCP server".to_string()));
                assert_eq!(bundled, &Some(false));
                assert_eq!(available_tools[0], "custom_tool");
            }
            _ => panic!("Expected Stdio variant"),
        }
    }

    #[test]
    fn test_enabled_extensions_state_missing_data() {
        // Test loading from ExtensionData without enabled_extensions
        let data = ExtensionData::default();
        let result = EnabledExtensionsState::from_extension_data(&data);

        // Should return None when the key doesn't exist
        assert!(result.is_none());
    }

    #[test]
    fn test_enabled_extensions_state_corrupt_data() {
        // Test loading from ExtensionData with corrupt data
        let mut data = ExtensionData::default();
        data.set_extension_state("enabled_extensions", "v0", json!("invalid json string"));

        let result = EnabledExtensionsState::from_extension_data(&data);

        // Should return None when deserialization fails
        assert!(result.is_none());
    }
}
