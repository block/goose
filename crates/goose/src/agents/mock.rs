use crate::agents::capabilities::Capabilities;
use crate::message::{Message, MessageContent, ToolRequest};
use crate::model::ModelConfig;
use crate::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
use crate::providers::errors::ProviderError;
use chrono::Utc;
use mcp_core::{tool::Tool, Role, ToolCall, ToolResult};
use serde_json::json;

#[derive(Clone)]
pub(crate) struct MockProvider {
    pub(crate) model_config: ModelConfig,
}

#[async_trait::async_trait]
impl Provider for MockProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::empty()
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model_config.clone()
    }

    async fn complete(
        &self,
        _system: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
        Ok((
            Message {
                role: Role::Assistant,
                created: Utc::now().timestamp(),
                content: vec![MessageContent::ToolRequest(ToolRequest {
                    id: "mock_tool_request".to_string(),
                    tool_call: ToolResult::Ok(ToolCall {
                        name: "platform__tool_by_tool_permission".to_string(),
                        arguments: json!({
                            "read_only_tools": ["file_reader", "data_fetcher"]
                        }),
                    }),
                })],
            },
            ProviderUsage::new("mock".to_string(), Usage::default()),
        ))
    }
}

pub(crate) fn create_mock_capabilities() -> Capabilities {
    let mock_model_config =
        ModelConfig::new("test-model".to_string()).with_context_limit(200_000.into());
    Capabilities::new(Box::new(MockProvider {
        model_config: mock_model_config,
    }))
}
