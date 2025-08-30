use std::future::Future;
use std::pin::Pin;

use mcp_core::handler::{PromptError, ResourceError};
use mcp_core::protocol::ServerCapabilities;
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;
use rmcp::model::{Content, ErrorCode, ErrorData, JsonRpcMessage, Prompt, Resource, Tool};
use serde_json::Value;
use tokio::sync::mpsc;

// Import the TODO tools
use goose::agents::todo_tools::{todo_read_tool, todo_write_tool};

/// A lightweight router that provides TODO task management capabilities.
///
/// This extension acts as a metadata provider - it exposes the TODO tools
/// and provides instructions, but the actual execution remains in the agent
/// for session storage access.
#[derive(Clone)]
pub struct TodoRouter {
    tools: Vec<Tool>,
    instructions: String,
}

impl Default for TodoRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoRouter {
    pub fn new() -> Self {
        let instructions = r#"The todo extension provides persistent task management throughout your session.

These tools help you track multi-step work, maintain context between interactions, and ensure systematic task completion.

## Task Management Guidelines

**Required Usage:**
- Use `todo__read` and `todo__write` for any task with 2+ steps, multiple files/components, or uncertain scope
- Skipping these tools when needed is considered an error

**Workflow:**
1. Start: Always `todo__read` first, then `todo__write` a brief checklist using Markdown checkboxes
2. During: After each major action, reread via `todo__read`, then update via `todo__write` - mark completed items, add new discoveries, note blockers
3. Finish: Ensure every item is checked, or clearly list what remains

**Critical:** `todo__write` replaces the entire list. Always read before writing - not doing so is an error.

**Best Practices:**
- Keep items short, specific, and action-oriented
- Use nested checkboxes for subtasks
- Include context about blockers or dependencies

Example format:
```markdown
- [x] Analyze request fully
- [ ] Create implementation plan
  - [x] General guidelines
  - [ ] Sample work
- [ ] Begin on implementation plan
```"#;

        Self {
            tools: vec![todo_read_tool(), todo_write_tool()],
            instructions: instructions.to_string(),
        }
    }
}

impl Router for TodoRouter {
    fn name(&self) -> String {
        "todo".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_prompts(false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        _tool_name: &str,
        _arguments: Value,
        _notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ErrorData>> + Send + 'static>> {
        // The agent handles TODO tool execution directly for session access.
        // This router only provides metadata and tool definitions.
        Box::pin(async move {
            Err(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                "TODO tools are executed directly by the agent".to_string(),
                None,
            ))
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async move { Ok(String::new()) })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        Vec::new()
    }

    fn get_prompt(
        &self,
        _prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        Box::pin(async move {
            Err(PromptError::NotFound(
                "TODO extension has no prompts".to_string(),
            ))
        })
    }
}
