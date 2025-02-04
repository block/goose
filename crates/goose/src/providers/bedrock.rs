use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_bedrockruntime::operation::converse::ConverseError;
use aws_sdk_bedrockruntime::{types as bedrock, Client};
use mcp_core::Tool;

use super::base::{Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::utils::emit_debug_trace;

// Import the migrated helper functions from providers/formats/bedrock.rs
use super::formats::bedrock::{
    from_bedrock_message, from_bedrock_usage, to_bedrock_message, to_bedrock_tool_config,
};

pub const BEDROCK_DOC_LINK: &str =
    "https://docs.aws.amazon.com/bedrock/latest/userguide/models-supported.html";

pub const BEDROCK_DEFAULT_MODEL: &str = "anthropic.claude-3-5-sonnet-20240620-v1:0";
pub const BEDROCK_KNOWN_MODELS: &[&str] = &[
    "anthropic.claude-3-5-sonnet-20240620-v1:0",
    "anthropic.claude-3-5-sonnet-20241022-v2:0",
];

#[derive(Debug, serde::Serialize)]
pub struct BedrockProvider {
    #[serde(skip)]
    client: Client,
    model: ModelConfig,
}

impl BedrockProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let sdk_config = futures::executor::block_on(aws_config::load_from_env());
        let client = Client::new(&sdk_config);

        Ok(Self { client, model })
    }
}

impl Default for BedrockProvider {
    fn default() -> Self {
        let model = ModelConfig::new(BedrockProvider::metadata().default_model);
        BedrockProvider::from_env(model).expect("Failed to initialize Bedrock provider")
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "bedrock",
            "Amazon Bedrock",
            "Run models through Amazon Bedrock. You may have to set AWS_ACCESS_KEY_ID, AWS_ACCESS_KEY, and AWS_REGION as env vars before configuring.",
            BEDROCK_DEFAULT_MODEL,
            BEDROCK_KNOWN_MODELS.iter().map(|s| s.to_string()).collect(),
            BEDROCK_DOC_LINK,
            vec![],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let model_name = &self.model.model_name;

        let mut request = self
            .client
            .converse()
            .system(bedrock::SystemContentBlock::Text(system.to_string()))
            .model_id(model_name.to_string())
            .set_messages(Some(
                messages
                    .iter()
                    .map(to_bedrock_message)
                    .collect::<Result<_>>()?,
            ));

        if !tools.is_empty() {
            request = request.tool_config(to_bedrock_tool_config(tools)?);
        }

        let response = request.send().await;

        let response = match response {
            Ok(response) => response,
            Err(err) => {
                return Err(match err.into_service_error() {
                    ConverseError::AccessDeniedException(err) => {
                        ProviderError::Authentication(format!("Failed to call Bedrock: {:?}", err))
                    }
                    ConverseError::ThrottlingException(err) => ProviderError::RateLimitExceeded(
                        format!("Failed to call Bedrock: {:?}", err),
                    ),
                    ConverseError::ValidationException(err)
                        if err
                            .message()
                            .unwrap_or_default()
                            .contains("Input is too long for requested model.") =>
                    {
                        ProviderError::ContextLengthExceeded(format!(
                            "Failed to call Bedrock: {:?}",
                            err
                        ))
                    }
                    ConverseError::ModelErrorException(err) => {
                        ProviderError::ExecutionError(format!("Failed to call Bedrock: {:?}", err))
                    }
                    err => {
                        ProviderError::ServerError(format!("Failed to call Bedrock: {:?}", err,))
                    }
                });
            }
        };

        let message = match response.output {
            Some(bedrock::ConverseOutput::Message(message)) => message,
            _ => {
                return Err(ProviderError::RequestFailed(
                    "No output from Bedrock".to_string(),
                ))
            }
        };

        let usage = response
            .usage
            .as_ref()
            .map(from_bedrock_usage)
            .unwrap_or_default();

        let message = from_bedrock_message(&message)?;

        // Add debug trace with input context
        let debug_payload = serde_json::json!({
            "system": system,
            "messages": messages,
            "tools": tools
        });
        emit_debug_trace(
            &self.model,
            &debug_payload,
            &serde_json::to_value(&message).unwrap_or_default(),
            &usage,
        );

        let provider_usage = ProviderUsage::new(model_name.to_string(), usage);
        Ok((message, provider_usage))
    }
}
