use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait, McpMeta};
use crate::providers::base::Provider;
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "oracle";

const DEFAULT_ORACLE_PROVIDER: &str = "openai";
const DEFAULT_ORACLE_MODEL: &str = "gpt-5.2";

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ThinkParams {
    /// The problem, question, or situation to think deeply about
    problem: String,
    /// Optional context or background information relevant to the problem
    #[serde(default)]
    context: Option<String>,
}

pub struct OracleClient {
    info: InitializeResult,
    provider: OnceCell<Arc<dyn Provider>>,
}

impl OracleClient {
    pub fn new(_context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tasks: None,
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
                title: Some("Oracle".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                indoc! {r#"
                The Oracle is a powerful reasoning tool that provides a "second opinion"
                for complex problems. It uses a different model to give
                you a fresh perspective.

                *critical* when starting a session or a new task always consult the oracle to think and come up with a plan

                Uses:
                - Architectural decisions and code review
                - Planning multi-step implementations
                - Getting a fresh perspective on stuck problems
            "#}
                .to_string(),
            ),
        };

        Ok(Self {
            info,
            provider: OnceCell::new(),
        })
    }

    async fn get_or_init_provider(&self) -> Result<&Arc<dyn Provider>, String> {
        self.provider
            .get_or_try_init(|| async {
                let config = crate::config::Config::global();

                let provider_name: String = config
                    .get_param("GOOSE_ORACLE_PROVIDER")
                    .unwrap_or_else(|_| DEFAULT_ORACLE_PROVIDER.to_string());
                let model_name: String = config
                    .get_param("GOOSE_ORACLE_MODEL")
                    .unwrap_or_else(|_| DEFAULT_ORACLE_MODEL.to_string());

                tracing::info!(
                    "Initializing Oracle with provider: {}, model: {}",
                    provider_name,
                    model_name
                );

                crate::providers::create_with_named_model(&provider_name, &model_name)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to create Oracle provider ({}:{}): {}. \
                            Make sure the provider is configured (e.g., OPENAI_API_KEY is set).",
                            provider_name,
                            model_name,
                            e
                        )
                    })
            })
            .await
            .map_err(|e| e.to_string())
    }

    async fn handle_think(&self, arguments: Option<JsonObject>) -> Result<Vec<Content>, String> {
        let args = arguments.as_ref().ok_or("Missing arguments")?;

        let problem = args
            .get("problem")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: problem")?;

        let context = args
            .get("context")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let provider = self.get_or_init_provider().await?;

        let system_prompt = indoc! {r#"
            You are the Oracle - a deep reasoning assistant that helps analyze complex problems.

            Your role is to:
            1. Carefully analyze the problem presented
            2. Consider multiple angles and potential solutions
            3. Identify edge cases, risks, and potential issues
            4. Provide clear, actionable recommendations
            5. Think step by step through complex logic

            Be thorough but concise. Focus on insights that might be missed on first glance.
            If you see potential bugs, architectural issues, or better approaches, highlight them.
        "#};

        let user_message = if let Some(ctx) = context {
            format!(
                "## Problem\n{}\n\n## Context\n{}\n\nPlease analyze this thoroughly and provide your insights.",
                problem, ctx
            )
        } else {
            format!(
                "## Problem\n{}\n\nPlease analyze this thoroughly and provide your insights.",
                problem
            )
        };

        let messages = vec![crate::conversation::message::Message::user().with_text(&user_message)];

        let (response, _usage) = provider
            .complete(system_prompt, &messages, &[])
            .await
            .map_err(|e| format!("Oracle failed to think: {}", e))?;

        let response_text = response
            .content
            .iter()
            .filter_map(|c| {
                if let crate::conversation::message::MessageContent::Text(t) = c {
                    Some(t.text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(vec![Content::text(response_text)])
    }

    fn get_tools() -> Vec<Tool> {
        let schema = schema_for!(ThinkParams);
        let schema_value =
            serde_json::to_value(schema).expect("Failed to serialize ThinkParams schema");

        vec![Tool::new(
            "think".to_string(),
            indoc! {r#"
                Ask the Oracle to think deeply about a problem.

                Use this tool when you need:
                - A second opinion on complex code or architecture
                - To plan a multi-step implementation
                - A fresh perspective when you're stuck

                The Oracle uses a powerful reasoning model to provide
                thorough analysis from a different perspective than the main agent.

                *critical* always use the the think tool first in a session or new task to think and plan
            "#}
            .to_string(),
            schema_value.as_object().unwrap().clone(),
        )
        .annotate(ToolAnnotations {
            title: Some("Think".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        })]
    }
}

#[async_trait]
impl McpClientTrait for OracleClient {
    async fn list_tools(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        _meta: McpMeta,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let content = match name {
            "think" => self.handle_think(arguments).await,
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

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}
