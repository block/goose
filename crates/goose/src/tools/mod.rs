//! Tools Module - Advanced tool management for Goose
//!
//! Provides Claude Code-style tool capabilities:
//! - Tool Search (dynamic discovery, 85% token reduction)
//! - Programmatic Tool Calling
//! - Tool Examples for accuracy
//! - Tool Registry management

mod programmatic;
mod registry;
mod search;

pub use programmatic::{ProgrammaticToolCall, ToolCallResult, ToolExample};
pub use registry::{ToolCategory, ToolDefinition, ToolRegistry};
pub use search::{ToolSearchConfig, ToolSearchResult, ToolSearchTool};

use serde::{Deserialize, Serialize};

/// Tool metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub token_cost: usize,
    pub examples: Vec<ToolExample>,
    pub schema: Option<serde_json::Value>,
}

impl ToolMetadata {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            category: ToolCategory::General,
            token_cost: 0,
            examples: Vec::new(),
            schema: None,
        }
    }

    pub fn with_category(mut self, category: ToolCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_token_cost(mut self, cost: usize) -> Self {
        self.token_cost = cost;
        self
    }

    pub fn with_examples(mut self, examples: Vec<ToolExample>) -> Self {
        self.examples = examples;
        self
    }

    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata_creation() {
        let metadata = ToolMetadata::new("test_tool", "A test tool")
            .with_category(ToolCategory::FileSystem)
            .with_token_cost(500);

        assert_eq!(metadata.name, "test_tool");
        assert_eq!(metadata.category, ToolCategory::FileSystem);
        assert_eq!(metadata.token_cost, 500);
    }
}
