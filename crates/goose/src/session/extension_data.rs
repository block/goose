// Extension data management for sessions
// Provides a simple way to store extension-specific data with versioned keys

use crate::config::ExtensionConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
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

/// Metadata for a loaded directory context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectoryContext {
    /// Turn number when this directory was last accessed
    pub access_turn: u32,
    /// Unique tag for identifying this context in system prompt extras
    pub tag: String,
}

/// State tracking which directories have had their agents.md files loaded
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoadedAgentsState {
    /// Map of directory path -> context metadata (load turn, access turn, tag)
    pub loaded_directories: HashMap<String, DirectoryContext>,
}

impl ExtensionState for LoadedAgentsState {
    const EXTENSION_NAME: &'static str = "loaded_agents";
    const VERSION: &'static str = "v0";
}

impl LoadedAgentsState {
    pub fn new() -> Self {
        Self {
            loaded_directories: HashMap::new(),
        }
    }

    /// Check if a directory has already been loaded
    pub fn is_loaded(&self, directory: &Path) -> bool {
        self.loaded_directories
            .contains_key(&directory.to_string_lossy().to_string())
    }

    /// Mark a directory as loaded at a specific turn and return its tag
    pub fn mark_loaded(&mut self, directory: &Path, turn: u32) -> String {
        let path_str = directory.to_string_lossy().to_string();
        let tag = format!("agents_md:{}", path_str);

        self.loaded_directories.insert(
            path_str,
            DirectoryContext {
                access_turn: turn,
                tag: tag.clone(),
            },
        );

        tag
    }

    /// Update last access time for a directory. Returns true if updated.
    pub fn mark_accessed(&mut self, directory: &Path, turn: u32) -> bool {
        let path_str = directory.to_string_lossy().to_string();
        if let Some(context) = self.loaded_directories.get_mut(&path_str) {
            if context.access_turn != turn {
                context.access_turn = turn;
                return true;
            }
        }
        false
    }

    /// Prune directories that haven't been accessed in N turns
    /// Returns a list of (path, tag) for pruned directories
    pub fn prune_stale(&mut self, current_turn: u32, max_idle_turns: u32) -> Vec<(String, String)> {
        let mut pruned = Vec::new();

        self.loaded_directories.retain(|path, context| {
            if current_turn.saturating_sub(context.access_turn) >= max_idle_turns {
                pruned.push((path.clone(), context.tag.clone()));
                false // remove
            } else {
                true // keep
            }
        });

        pruned
    }
}

impl Default for LoadedAgentsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to get or create LoadedAgentsState from session
pub fn get_or_create_loaded_agents_state(extension_data: &ExtensionData) -> LoadedAgentsState {
    LoadedAgentsState::from_extension_data(extension_data).unwrap_or_else(LoadedAgentsState::new)
}

/// Helper function to save LoadedAgentsState to session
pub fn save_loaded_agents_state(
    extension_data: &mut ExtensionData,
    state: &LoadedAgentsState,
) -> Result<()> {
    state.to_extension_data(extension_data)
}

/// Conversation turn counter state (survives compaction)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversationTurnState {
    /// Current turn number (increments on each agent iteration)
    pub turn: u32,
}

impl ExtensionState for ConversationTurnState {
    const EXTENSION_NAME: &'static str = "conversation_turn";
    const VERSION: &'static str = "v0";
}

impl ConversationTurnState {
    pub fn new() -> Self {
        Self { turn: 0 }
    }

    pub fn increment(&mut self) -> u32 {
        self.turn += 1;
        self.turn
    }
}

impl Default for ConversationTurnState {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to get or create conversation turn state
pub fn get_or_create_conversation_turn_state(
    extension_data: &ExtensionData,
) -> ConversationTurnState {
    ConversationTurnState::from_extension_data(extension_data)
        .unwrap_or_else(ConversationTurnState::new)
}

/// Helper to save conversation turn state
pub fn save_conversation_turn_state(
    extension_data: &mut ExtensionData,
    state: &ConversationTurnState,
) -> Result<()> {
    state.to_extension_data(extension_data)
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
    fn test_loaded_agents_state_creation() {
        let state = LoadedAgentsState::new();
        assert!(state.loaded_directories.is_empty());
    }

    #[test]
    fn test_loaded_agents_state_mark_loaded() {
        let mut state = LoadedAgentsState::new();
        let path = Path::new("/repo/features/auth");

        assert!(!state.is_loaded(path));

        let tag = state.mark_loaded(path, 1);
        assert_eq!(tag, "agents_md:/repo/features/auth");
        assert!(state.is_loaded(path));

        // Verify context details
        let context = state.loaded_directories.get("/repo/features/auth").unwrap();
        assert_eq!(context.access_turn, 1);
    }

    #[test]
    fn test_loaded_agents_state_mark_accessed() {
        let mut state = LoadedAgentsState::new();
        let path = Path::new("/repo/features/auth");

        state.mark_loaded(path, 1);

        // Same turn - should return false
        assert!(!state.mark_accessed(path, 1));

        // New turn - should return true and update
        assert!(state.mark_accessed(path, 5));

        let context = state.loaded_directories.get("/repo/features/auth").unwrap();
        assert_eq!(context.access_turn, 5);

        // Same turn again - should return false
        assert!(!state.mark_accessed(path, 5));
    }

    #[test]
    fn test_prune_stale() {
        let mut state = LoadedAgentsState::new();

        // Load directories at different turns
        state.mark_loaded(Path::new("/repo/auth"), 1);
        state.mark_loaded(Path::new("/repo/payments"), 2);
        state.mark_loaded(Path::new("/repo/api"), 10);

        // Access auth at turn 8
        state.mark_accessed(Path::new("/repo/auth"), 8);

        // At turn 20, with max_idle_turns=10:
        // Clone state for independent test case
        let mut state_1 = state.clone();
        let pruned = state_1.prune_stale(20, 10);
        assert_eq!(pruned.len(), 3); // All are stale or at threshold
        assert!(!state_1.is_loaded(Path::new("/repo/auth")));

        // With max_idle_turns=11:
        let pruned = state.prune_stale(20, 11);
        assert_eq!(pruned.len(), 2); // auth is stale (idle 12), payments is stale (idle 18), api is not stale (idle 10)

        assert!(!state.is_loaded(Path::new("/repo/auth")));
        assert!(!state.is_loaded(Path::new("/repo/payments")));
        assert!(state.is_loaded(Path::new("/repo/api")));
    }

    #[test]
    fn test_loaded_agents_state_serialization() {
        let mut state = LoadedAgentsState::new();
        state.mark_loaded(Path::new("/repo/features/auth"), 1);
        state.mark_loaded(Path::new("/repo/features/payments"), 2);

        let mut extension_data = ExtensionData::default();
        state.to_extension_data(&mut extension_data).unwrap();

        let restored = LoadedAgentsState::from_extension_data(&extension_data).unwrap();
        assert_eq!(state, restored);
        assert_eq!(restored.loaded_directories.len(), 2);
    }

    #[test]
    fn test_get_or_create_loaded_agents_state() {
        let extension_data = ExtensionData::default();
        let state = get_or_create_loaded_agents_state(&extension_data);
        assert!(state.loaded_directories.is_empty());

        let mut extension_data = ExtensionData::default();
        let mut state = LoadedAgentsState::new();
        state.mark_loaded(Path::new("/test"), 1);
        save_loaded_agents_state(&mut extension_data, &state).unwrap();

        let restored = get_or_create_loaded_agents_state(&extension_data);
        assert!(restored.is_loaded(Path::new("/test")));
    }
}
