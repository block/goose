use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::subagent_task_config::TaskConfig;
use crate::agents::subagent_tool::{
    create_subagent_tool, handle_subagent_tool, SUBAGENT_TOOL_NAME,
};
use crate::agents::tool_execution::DeferredToolCall;
use crate::agents::AgentConfig;
use crate::config::get_enabled_extensions;
use crate::config::{GooseMode, PermissionManager};
use crate::session::SessionType;
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, GetPromptResult, Implementation, InitializeResult, JsonObject,
    ListPromptsResult, ListResourcesResult, ListToolsResult, ProtocolVersion, ReadResourceResult,
    ServerCapabilities, ServerNotification, Tool,
};
use serde_json::Value;
use std::sync::Arc;
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
                capabilities: ServerCapabilities::builder().enable_tools().build(),
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

    async fn get_provider(&self) -> Option<std::sync::Arc<dyn crate::providers::base::Provider>> {
        let em = self.context.extension_manager.as_ref()?.upgrade()?;
        let provider_guard = em.get_provider().lock().await;
        provider_guard.clone()
    }

    async fn get_extensions(&self) -> Vec<crate::agents::ExtensionConfig> {
        let extensions = if let Some(em) = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|w| w.upgrade())
        {
            em.get_extension_configs().await
        } else {
            get_enabled_extensions()
        };
        extensions
            .into_iter()
            .filter(|ext| ext.name() != EXTENSION_NAME)
            .collect()
    }

    async fn get_sub_recipes(&self) -> std::collections::HashMap<String, crate::recipe::SubRecipe> {
        match &self.context.sub_recipes {
            Some(recipes) => recipes.read().await.clone(),
            None => std::collections::HashMap::new(),
        }
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
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn read_resource(
        &self,
        _session_id: &str,
        _uri: &str,
        _cancellation_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: vec![self.build_tool().await],
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let deferred = self
            .call_tool_deferred(session_id, name, arguments, cancellation_token)
            .await?;
        deferred.result.await.map_err(Error::McpError)
    }

    async fn call_tool_deferred(
        &self,
        session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        cancellation_token: CancellationToken,
    ) -> Result<DeferredToolCall, Error> {
        if name != SUBAGENT_TOOL_NAME {
            return Ok(DeferredToolCall::from(Ok(CallToolResult::error(vec![
                Content::text(format!("Unknown tool: {}", name)),
            ]))));
        }

        if self.context.goose_mode != GooseMode::Auto {
            return Ok(DeferredToolCall::from(Ok(CallToolResult::error(vec![
                Content::text("Subagents are only available in Auto mode."),
            ]))));
        }

        // Check if this is already a subagent session
        let session_manager = Arc::clone(&self.context.session_manager);
        if let Ok(session) = session_manager.get_session(session_id, false).await {
            if session.session_type == SessionType::SubAgent {
                return Ok(DeferredToolCall::from(Ok(CallToolResult::error(vec![
                    Content::text("Subagents cannot spawn subagents."),
                ]))));
            }
        }

        let Some(provider) = self.get_provider().await else {
            return Ok(DeferredToolCall::from(Ok(CallToolResult::error(vec![
                Content::text("No provider configured"),
            ]))));
        };

        if provider.get_active_model_name().starts_with("gemini") {
            return Ok(DeferredToolCall::from(Ok(CallToolResult::error(vec![
                Content::text("Subagents are not supported with Gemini models."),
            ]))));
        }

        let working_dir = session_manager
            .get_session(session_id, false)
            .await
            .map(|s| s.working_dir)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| ".".into()));

        let extensions = self.get_extensions().await;
        let sub_recipes = self.get_sub_recipes().await;
        let task_config = TaskConfig::new(provider, extensions);
        let arguments_value = arguments
            .map(Value::Object)
            .unwrap_or(Value::Object(serde_json::Map::new()));

        // Create AgentConfig for the subagent
        let agent_config = AgentConfig::new(
            session_manager,
            PermissionManager::instance(),
            None,
            GooseMode::Auto,
        );

        Ok(handle_subagent_tool(
            &agent_config,
            arguments_value,
            task_config,
            sub_recipes,
            working_dir,
            Some(cancellation_token),
        ))
    }

    async fn list_prompts(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn get_prompt(
        &self,
        _session_id: &str,
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

    async fn get_moim(&self, _session_id: &str) -> Option<String> {
        None
    }
}
