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
    ServerCapabilities, ServerNotification, Tool, ToolAnnotations, ToolsCapability,
};
use rmcp::object;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "todo";

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

                Use todo_read and todo_write for tasks with 2+ steps, multiple files/components, or uncertain scope.

                Workflow:
                - Start: read → write checklist
                - During: read → update progress
                - End: verify all complete

                Warning: todo_write overwrites entirely; always todo_read first (skipping is an error)

                Keep items short, specific, action-oriented. Not using the todo tools for complex tasks is an error.

                Template:
                - [ ] Implement feature X
                  - [ ] Update API
                  - [ ] Write tests
                  - [ ] Run tests (subagent in parallel)
                  - [ ] Run lint (subagent in parallel)
                - [ ] Blocked: waiting on credentials
            "#}.to_string()),
        };

        Ok(Self {
            info,
            context,
            fallback_content: tokio::sync::RwLock::new(String::new()),
        })
    }

    async fn handle_read_todo(&self) -> Result<Vec<Content>, String> {
        if let Some(session_id) = &self.context.session_id {
            match SessionManager::get_session(session_id, false).await {
                Ok(metadata) => {
                    let content =
                        extension_data::TodoState::from_extension_data(&metadata.extension_data)
                            .map(|state| state.content)
                            .unwrap_or_default();
                    Ok(vec![Content::text(content)])
                }
                Err(_) => Ok(vec![Content::text(String::new())]),
            }
        } else {
            let content = self.fallback_content.read().await;
            Ok(vec![Content::text(content.clone())])
        }
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
        vec![
            Tool::new(
                "todo_read".to_string(),
                indoc! {r#"
                        Read the entire TODO file content.

                        This tool reads the complete TODO file and returns its content as a string.
                        Use this to view current tasks, notes, and any other information stored in the TODO file.

                        The tool will return an error if the TODO file doesn't exist or cannot be read.
                    "#}.to_string(),
                object!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    }),
            ).annotate(ToolAnnotations {
                title: Some("Read TODO file".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: Some(false),
            }),
            Tool::new(
                "todo_write".to_string(),
                indoc! {r#"
                    Write or overwrite the entire TODO file content.

                    This tool replaces the complete TODO file content with the provided string.
                    Use this to update tasks, add new items, or reorganize the TODO file.

                    WARNING: This operation completely replaces the file content. Make sure to include
                    all content you want to keep, not just the changes.

                    The tool will create the TODO file if it doesn't exist, or overwrite it if it does.
                    Returns an error if the file cannot be written due to permissions or other I/O issues.
                "#}.to_string(),
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
            ).annotate(ToolAnnotations {
                title: Some("Write TODO file".to_string()),
                read_only_hint: Some(false),
                destructive_hint: Some(true),
                idempotent_hint: Some(false),
                open_world_hint: Some(false),
            })
        ]
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
            "todo_read" => self.handle_read_todo().await,
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
}
