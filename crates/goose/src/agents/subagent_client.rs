use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::subagent_task_config::TaskConfig;
use crate::agents::subagent_tool::{
    create_subagent_tool, handle_subagent_tool, SUBAGENT_TOOL_NAME,
};
use crate::config::get_enabled_extensions;
use crate::session::session_manager::SessionType;
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, GetPromptResult, Implementation, InitializeResult, JsonObject,
    ListPromptsResult, ListResourcesResult, ListToolsResult, ProtocolVersion, ReadResourceResult,
    ServerCapabilities, ServerNotification, Tool, ToolsCapability,
};
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub const EXTENSION_NAME: &str = "subagent";

pub struct SubagentClient {
    context: PlatformExtensionContext,
    info: InitializeResult,
}

impl SubagentClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        Ok(Self {
            context,
            info: InitializeResult {
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
                    title: Some("Subagent".to_string()),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    icons: None,
                    website_url: None,
                },
                instructions: Some(
                    "Delegate tasks to independent subagents for parallel or focused work."
                        .to_string(),
                ),
            },
        })
    }

    fn is_subagent_session(&self) -> bool {
        matches!(self.context.session_type, Some(SessionType::SubAgent))
    }

    async fn get_provider(&self) -> Option<std::sync::Arc<dyn crate::providers::base::Provider>> {
        let em = self.context.extension_manager.as_ref()?.upgrade()?;
        em.get_provider().await
    }

    async fn get_sub_recipes(&self) -> std::collections::HashMap<String, crate::recipe::SubRecipe> {
        match &self.context.sub_recipes {
            Some(recipes) => recipes.read().await.clone(),
            None => std::collections::HashMap::new(),
        }
    }

    fn get_working_dir(&self) -> PathBuf {
        self.context
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    async fn build_tool(&self) -> Tool {
        let sub_recipes = self.get_sub_recipes().await;
        let sub_recipes_vec: Vec<_> = sub_recipes.values().cloned().collect();
        create_subagent_tool(&sub_recipes_vec)
    }
}

#[async_trait]
impl McpClientTrait for SubagentClient {
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
        if self.is_subagent_session() {
            return Ok(ListToolsResult {
                tools: Vec::new(),
                next_cursor: None,
            });
        }

        Ok(ListToolsResult {
            tools: vec![self.build_tool().await],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        if name != SUBAGENT_TOOL_NAME {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Unknown tool: {}",
                name
            ))]));
        }

        if self.is_subagent_session() {
            return Ok(CallToolResult::error(vec![Content::text(
                "Subagents cannot spawn other subagents",
            )]));
        }

        let Some(provider) = self.get_provider().await else {
            return Ok(CallToolResult::error(vec![Content::text(
                "No provider configured",
            )]));
        };

        let extensions = get_enabled_extensions();
        let working_dir = self.get_working_dir();
        let sub_recipes = self.get_sub_recipes().await;
        let task_config = TaskConfig::new_minimal(provider, extensions);

        let arguments_value = arguments
            .map(Value::Object)
            .unwrap_or(Value::Object(serde_json::Map::new()));

        let result = handle_subagent_tool(
            arguments_value,
            task_config,
            sub_recipes,
            working_dir,
            Some(cancellation_token),
        );

        match result.result.await {
            Ok(call_result) => Ok(call_result),
            Err(error_data) => Ok(CallToolResult::error(vec![Content::text(
                error_data.message.to_string(),
            )])),
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
        None
    }
}
