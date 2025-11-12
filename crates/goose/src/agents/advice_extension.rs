use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::subagent_handler::run_complete_subagent_task;
use crate::agents::subagent_task_config::TaskConfig;
use crate::prompt_template;
use crate::recipe::Recipe;
use crate::session::SessionManager;
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, ErrorCode, ErrorData, GetPromptResult, Implementation,
    InitializeResult, JsonObject, ListPromptsResult, ListResourcesResult, ListToolsResult,
    ProtocolVersion, ReadResourceResult, ServerCapabilities, ServerNotification, Tool,
    ToolAnnotations, ToolsCapability,
};
use rmcp::object;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "advice";

pub struct AdviceClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
}

impl AdviceClient {
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
                title: Some("Advice".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                indoc! {r#"
                Expert Advice System
                
                ALWAYS use advice() when facing: uncertainty, multiple paths, complex decisions, or before implementing solutions.
                The advisor has deep expertise you lack. Consulting it prevents mistakes and reveals better approaches.
                Use iteratively: get advice, implement, get more advice. Your work should be punctuated by multiple advice calls.
            "#}
                .to_string(),
            ),
        };

        Ok(Self { info, context })
    }

    async fn provide_advice(&self, query: &str) -> Result<String> {
        let parent_session_id = self
            .context
            .session_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No session ID"))?;

        let parent_session = SessionManager::get_session(parent_session_id, false).await?;

        let subagent_session = SessionManager::create_session(
            parent_session.working_dir.clone(),
            "Advice Subagent".to_string(),
            crate::session::session_manager::SessionType::SubAgent,
        )
        .await?;

        let prompt_context = serde_json::json!({"query": query});
        let advice_prompt = prompt_template::render_global_file("advice.md", &prompt_context)?;

        let recipe = Recipe::builder()
            .version("1.0.0")
            .title("Advice")
            .description("Expert advice")
            .prompt(advice_prompt)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build recipe: {}", e))?;

        let provider = self
            .context
            .provider
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No provider"))?
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Provider not initialized"))?
            .clone();

        let task_config = TaskConfig::new(
            provider,
            parent_session_id,
            &parent_session.working_dir,
            vec![],
        );

        run_complete_subagent_task(recipe, task_config, true, subagent_session.id).await
    }
}

#[async_trait]
impl McpClientTrait for AdviceClient {
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
            tools: vec![Tool::new(
                "advice".to_string(),
                "Expert advisor - ALWAYS consult when uncertain, choosing paths, or before complex work. Prevents mistakes.".to_string(),
                object!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Your question, dilemma, or planned approach to validate, including all context that an expert would need to provide solid advice on the subject."
                        }
                    },
                    "required": ["query"]
                }),
            )
            .annotate(ToolAnnotations {
                title: Some("Expert Advice".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: Some(true),
            })],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        if name != "advice" {
            return Err(Error::McpError(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                "Unknown tool".to_string(),
                None,
            )));
        }

        let query = arguments
            .as_ref()
            .ok_or_else(|| {
                Error::McpError(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    "Missing arguments".to_string(),
                    None,
                ))
            })?
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                Error::McpError(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    "Missing query parameter".to_string(),
                    None,
                ))
            })?;

        match self.provide_advice(query).await {
            Ok(advice) => Ok(CallToolResult::success(vec![
                Content::text(advice).with_priority(1.0)
            ])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get advice: {}",
                e
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
