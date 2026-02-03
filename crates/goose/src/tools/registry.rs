//! Tool Registry - Central management for all available tools

use super::programmatic::ToolExample;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Category of a tool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    FileSystem,
    Search,
    Execution,
    Web,
    Database,
    Git,
    Testing,
    General,
}

/// Definition of a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub schema: Option<serde_json::Value>,
    pub examples: Vec<ToolExample>,
    pub token_cost: usize,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            category: ToolCategory::General,
            schema: None,
            examples: Vec::new(),
            token_cost: 200, // Default estimate
        }
    }

    pub fn with_category(mut self, category: ToolCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_examples(mut self, examples: Vec<ToolExample>) -> Self {
        self.examples = examples;
        self
    }

    pub fn with_token_cost(mut self, cost: usize) -> Self {
        self.token_cost = cost;
        self
    }
}

/// Central registry for all available tools
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, ToolDefinition>>,
    by_category: RwLock<HashMap<ToolCategory, Vec<String>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            by_category: RwLock::new(HashMap::new()),
        }
    }

    /// Create registry with default tools
    pub fn with_defaults() -> Self {
        let _registry = Self::new();

        // Register default tools synchronously using blocking
        let tools = Self::default_tools();
        let mut tools_map = HashMap::new();
        let mut category_map: HashMap<ToolCategory, Vec<String>> = HashMap::new();

        for tool in tools {
            category_map
                .entry(tool.category)
                .or_default()
                .push(tool.name.clone());
            tools_map.insert(tool.name.clone(), tool);
        }

        // We can't use async in new(), so we create with empty and would need to call init()
        Self {
            tools: RwLock::new(tools_map),
            by_category: RwLock::new(category_map),
        }
    }

    fn default_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::new("Read", "Read file contents from the filesystem")
                .with_category(ToolCategory::FileSystem)
                .with_token_cost(200),
            ToolDefinition::new("Write", "Write content to a file")
                .with_category(ToolCategory::FileSystem)
                .with_token_cost(250),
            ToolDefinition::new("Edit", "Edit existing file content")
                .with_category(ToolCategory::FileSystem)
                .with_token_cost(300),
            ToolDefinition::new("Glob", "Find files matching a pattern")
                .with_category(ToolCategory::Search)
                .with_token_cost(150),
            ToolDefinition::new("Grep", "Search for patterns in files")
                .with_category(ToolCategory::Search)
                .with_token_cost(200),
            ToolDefinition::new("Bash", "Execute shell commands")
                .with_category(ToolCategory::Execution)
                .with_token_cost(300),
            ToolDefinition::new("WebFetch", "Fetch content from a URL")
                .with_category(ToolCategory::Web)
                .with_token_cost(400),
            ToolDefinition::new("WebSearch", "Search the web")
                .with_category(ToolCategory::Web)
                .with_token_cost(500),
            ToolDefinition::new("Task", "Spawn a subagent to perform a task")
                .with_category(ToolCategory::Execution)
                .with_token_cost(350),
        ]
    }

    /// Register a new tool
    pub async fn register(&self, tool: ToolDefinition) {
        let mut tools = self.tools.write().await;
        let mut by_category = self.by_category.write().await;

        by_category
            .entry(tool.category)
            .or_default()
            .push(tool.name.clone());
        tools.insert(tool.name.clone(), tool);
    }

    /// Get a tool by name
    pub async fn get(&self, name: &str) -> Option<ToolDefinition> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// Get all tools
    pub async fn list(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// Get tools by category
    pub async fn list_by_category(&self, category: ToolCategory) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        let by_category = self.by_category.read().await;

        by_category
            .get(&category)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| tools.get(name).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get total token cost of all tools
    pub async fn total_token_cost(&self) -> usize {
        let tools = self.tools.read().await;
        tools.values().map(|t| t.token_cost).sum()
    }

    /// Get tool count
    pub async fn count(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }

    /// Check if a tool exists
    pub async fn contains(&self, name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(name)
    }

    /// Remove a tool
    pub async fn remove(&self, name: &str) -> Option<ToolDefinition> {
        let mut tools = self.tools.write().await;
        let mut by_category = self.by_category.write().await;

        if let Some(tool) = tools.remove(name) {
            if let Some(names) = by_category.get_mut(&tool.category) {
                names.retain(|n| n != name);
            }
            Some(tool)
        } else {
            None
        }
    }

    /// Get statistics about the registry
    pub async fn stats(&self) -> RegistryStats {
        let tools = self.tools.read().await;
        let by_category = self.by_category.read().await;

        let mut category_counts = HashMap::new();
        for (category, names) in by_category.iter() {
            category_counts.insert(*category, names.len());
        }

        RegistryStats {
            total_tools: tools.len(),
            total_token_cost: tools.values().map(|t| t.token_cost).sum(),
            by_category: category_counts,
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    pub total_tools: usize,
    pub total_token_cost: usize,
    pub by_category: HashMap<ToolCategory, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition() {
        let tool = ToolDefinition::new("TestTool", "A test tool")
            .with_category(ToolCategory::Testing)
            .with_token_cost(100);

        assert_eq!(tool.name, "TestTool");
        assert_eq!(tool.category, ToolCategory::Testing);
        assert_eq!(tool.token_cost, 100);
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let registry = ToolRegistry::new();

        let tool = ToolDefinition::new("CustomTool", "A custom tool");
        registry.register(tool).await;

        assert!(registry.contains("CustomTool").await);
        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_registry_with_defaults() {
        let registry = ToolRegistry::with_defaults();

        assert!(registry.count().await > 0);
        assert!(registry.contains("Read").await);
        assert!(registry.contains("Bash").await);
    }

    #[tokio::test]
    async fn test_registry_by_category() {
        let registry = ToolRegistry::with_defaults();

        let fs_tools = registry.list_by_category(ToolCategory::FileSystem).await;
        assert!(!fs_tools.is_empty());
        assert!(fs_tools.iter().any(|t| t.name == "Read"));
    }

    #[tokio::test]
    async fn test_registry_stats() {
        let registry = ToolRegistry::with_defaults();
        let stats = registry.stats().await;

        assert!(stats.total_tools > 0);
        assert!(stats.total_token_cost > 0);
    }
}
