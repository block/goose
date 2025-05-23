use mcp_core::{Content, ToolError};

use async_trait::async_trait;
use serde_json::Value;

pub enum RouterToolSelectionStrategy {
    Vector,
}

#[async_trait]
pub trait RouterToolSelector: Send + Sync {
    async fn select_tools(&self, params: Value) -> Result<Vec<Content>, ToolError>;
}

pub struct VectorToolSelector;

#[async_trait]
impl RouterToolSelector for VectorToolSelector {
    async fn select_tools(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let query = params.get("query").and_then(|v| v.as_str());
        println!("query: {:?}", query);
        let selected_tools = Vec::new();
        // TODO: placeholder for vector tool selection
        Ok(selected_tools)
    }
}

// Helper function to create a boxed tool selector
pub fn create_tool_selector(
    strategy: Option<RouterToolSelectionStrategy>,
) -> Box<dyn RouterToolSelector> {
    match strategy {
        Some(RouterToolSelectionStrategy::Vector) => Box::new(VectorToolSelector),
        _ => Box::new(VectorToolSelector), // Default to VectorToolSelector
    }
}
