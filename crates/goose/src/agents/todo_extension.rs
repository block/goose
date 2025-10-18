use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::session::extension_data::ExtensionState;
use crate::session::{extension_data, SessionManager};
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, GetPromptResult, Implementation, InitializeResult, JsonObject,
    ListPromptsResult, ListResourcesResult, ListToolsResult, ProtocolVersion, ReadResourceResult,
    Role, ServerCapabilities, ServerNotification, Tool, ToolAnnotations, ToolsCapability,
};
use rmcp::object;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "todo";

// Template definitions
const TODO_TEMPLATE_INDEPENDENT: &str = include_str!("../prompts/todo_independent.md");
const TODO_TEMPLATE_WITH_USER: &str = include_str!("../prompts/todo_with_user.md");

// Template selection function
fn select_template_for_prompt(prompt: &str) -> Option<&'static str> {
    let prompt_lower = prompt.to_lowercase();

    // Check for autonomous/independent keywords
    if prompt_lower.contains("act autonomously") || prompt_lower.contains("work independently") {
        return Some(TODO_TEMPLATE_INDEPENDENT);
    }

    // Default to with_user template for all other cases
    Some(TODO_TEMPLATE_WITH_USER)
}

pub struct TodoClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
    fallback_content: tokio::sync::RwLock<String>,
}

impl TodoClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                title: Some("Todo".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(indoc! {r#"
                Task Management

                Use todo_write for tasks with 2+ steps, multiple files/components, or uncertain scope.
                Your TODO content is automatically available in your context.

                Workflow:
                - Start: write initial checklist
                - During: update progress
                - End: verify all complete

                Warning: todo_write overwrites entirely; always include ALL content you want to keep

                Keep items short, specific, action-oriented. Not using the todo tool for complex tasks is an error.

                Template:
                - [ ] Implement feature X
                  - [ ] Update API
                  - [ ] Write tests
                  - [ ] Run tests
                  - [ ] Run lint
                - [ ] Blocked: waiting on credentials
            "#}.to_string()),
        };

        Ok(Self {
            info,
            context,
            fallback_content: tokio::sync::RwLock::new(String::new()),
        })
    }

    async fn handle_write_todo(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let content = arguments
            .as_ref()
            .ok_or("Missing arguments")?
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: content")?
            .to_string();

        let char_count = content.chars().count();
        let max_chars = std::env::var("GOOSE_TODO_MAX_CHARS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50_000);

        if max_chars > 0 && char_count > max_chars {
            return Err(format!(
                "Todo list too large: {} chars (max: {})",
                char_count, max_chars
            ));
        }

        if let Some(session_id) = &self.context.session_id {
            match SessionManager::get_session(session_id, false).await {
                Ok(mut session) => {
                    let todo_state = extension_data::TodoState::new(content);
                    if todo_state
                        .to_extension_data(&mut session.extension_data)
                        .is_ok()
                    {
                        match SessionManager::update_session(session_id)
                            .extension_data(session.extension_data)
                            .apply()
                            .await
                        {
                            Ok(_) => Ok(vec![Content::text(format!(
                                "Updated ({} chars)",
                                char_count
                            ))]),
                            Err(_) => Err("Failed to update session metadata".to_string()),
                        }
                    } else {
                        Err("Failed to serialize TODO state".to_string())
                    }
                }
                Err(_) => Err("Failed to read session metadata".to_string()),
            }
        } else {
            let mut fallback = self.fallback_content.write().await;
            *fallback = content;
            Ok(vec![Content::text(format!(
                "Updated ({} chars)",
                char_count
            ))])
        }
    }

    fn get_tools() -> Vec<Tool> {
        vec![Tool::new(
            "todo_write".to_string(),
            indoc! {r#"
                    Overwrite the entire TODO content.

                    The content persists across conversation turns and compaction. Use this for:
                    - Task tracking and progress updates
                    - Important notes and reminders

                    WARNING: This operation completely replaces the existing content. Always include
                    all content you want to keep, not just the changes.
                "#}
            .to_string(),
            object!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The TODO list content to save"
                    }
                },
                "required": ["content"]
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Write TODO".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        })]
    }
}

#[async_trait]
impl McpClientTrait for TodoClient {
    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn read_resource(
        &self,
        _uri: &str,
        _cancellation_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn list_tools(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let content = match name {
            "todo_write" => self.handle_write_todo(arguments).await,
            _ => Err(format!("Unknown tool: {}", name)),
        };

        match content {
            Ok(content) => Ok(CallToolResult::success(content)),
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                error
            ))])),
        }
    }

    async fn list_prompts(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: Value,
        _cancellation_token: CancellationToken,
    ) -> Result<GetPromptResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        mpsc::channel(1).1
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    async fn get_moim(&self) -> Option<String> {
        // Retrieve TODO content from session or fallback storage
        let content = if let Some(session_id) = &self.context.session_id {
            // Session-aware: get from session metadata
            match SessionManager::get_session(session_id, true).await {
                Ok(mut session) => {
                    // Check if TODO is empty
                    let todo_state =
                        extension_data::TodoState::from_extension_data(&session.extension_data);
                    let is_empty = todo_state
                        .as_ref()
                        .is_none_or(|s| s.content.trim().is_empty());

                    if is_empty {
                        // Check for template selection from either recipe instructions or first user message
                        let mut selected_template = None;

                        // First, check if there's a recipe with instructions
                        if let Some(recipe) = &session.recipe {
                            if let Some(instructions) = &recipe.instructions {
                                selected_template = select_template_for_prompt(instructions);
                                if selected_template.is_some() {
                                    tracing::debug!(
                                        "Selected template based on recipe instructions"
                                    );
                                }
                            }
                        }

                        // If no template from recipe, try first user message
                        if selected_template.is_none() {
                            if let Some(conversation) = &session.conversation {
                                if let Some(first_user_msg) = conversation
                                    .messages()
                                    .iter()
                                    .find(|m| matches!(m.role, Role::User))
                                    .and_then(|m| m.content.first())
                                    .and_then(|c| c.as_text())
                                {
                                    selected_template = select_template_for_prompt(first_user_msg);
                                    if selected_template.is_some() {
                                        tracing::debug!(
                                            "Selected template based on first user message"
                                        );
                                    }
                                }
                            }
                        }

                        // Apply the selected template if we have one
                        if let Some(template) = selected_template {
                            let todo_state = extension_data::TodoState::new(template.to_string());
                            if todo_state
                                .to_extension_data(&mut session.extension_data)
                                .is_ok()
                            {
                                if let Err(e) = SessionManager::update_session(session_id)
                                    .extension_data(session.extension_data.clone())
                                    .apply()
                                    .await
                                {
                                    tracing::warn!("Failed to save default TODO template: {}", e);
                                } else {
                                    tracing::debug!("Populated TODO with default template");
                                    return Some(format!(
                                        "Current tasks and notes:\n{}\n",
                                        template
                                    ));
                                }
                            }
                        }
                    }

                    // Return existing content if any
                    extension_data::TodoState::from_extension_data(&session.extension_data)
                        .map(|state| state.content)
                        .filter(|c| !c.trim().is_empty())
                }
                Err(e) => {
                    tracing::debug!("Could not read session for MOIM: {}", e);
                    None
                }
            }
        } else {
            // No session: use fallback storage
            let fallback = self.fallback_content.read().await;
            if !fallback.trim().is_empty() {
                Some(fallback.clone())
            } else {
                None
            }
        };

        // Format content for MOIM injection
        content.map(|c| format!("Current tasks and notes:\n{}\n", c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_selection() {
        // Test autonomous keywords
        assert_eq!(
            select_template_for_prompt("Please act autonomously on this task"),
            Some(TODO_TEMPLATE_INDEPENDENT)
        );

        assert_eq!(
            select_template_for_prompt("Work independently to solve this"),
            Some(TODO_TEMPLATE_INDEPENDENT)
        );

        assert_eq!(
            select_template_for_prompt("ACT AUTONOMOUSLY"),
            Some(TODO_TEMPLATE_INDEPENDENT)
        );

        // Test default with_user template
        assert_eq!(
            select_template_for_prompt("Help me with this task"),
            Some(TODO_TEMPLATE_WITH_USER)
        );

        assert_eq!(
            select_template_for_prompt("Can you assist with debugging?"),
            Some(TODO_TEMPLATE_WITH_USER)
        );

        assert_eq!(
            select_template_for_prompt(""),
            Some(TODO_TEMPLATE_WITH_USER)
        );
    }

    #[test]
    fn test_template_selection_with_recipe_instructions() {
        // Test that recipe instructions with keywords select independent template
        let recipe_instructions = "You should act autonomously to complete this weekly report. Do not ask for user input.";
        assert_eq!(
            select_template_for_prompt(recipe_instructions),
            Some(TODO_TEMPLATE_INDEPENDENT)
        );

        let recipe_instructions2 =
            "Work independently on analyzing the codebase and generating documentation.";
        assert_eq!(
            select_template_for_prompt(recipe_instructions2),
            Some(TODO_TEMPLATE_INDEPENDENT)
        );

        // Test that recipe instructions without keywords select with_user template
        let recipe_instructions3 = "Generate a summary of the project status.";
        assert_eq!(
            select_template_for_prompt(recipe_instructions3),
            Some(TODO_TEMPLATE_WITH_USER)
        );
    }
}
