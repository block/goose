use std::collections::HashMap;

use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::retry::{ProviderRetry, RetryConfig};
use crate::conversation::message::Message;
use crate::impl_provider_default;
use crate::model::ModelConfig;
use crate::providers::utils::emit_debug_trace;
use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_bedrockruntime::config::ProvideCredentials;
use aws_sdk_bedrockruntime::operation::converse::ConverseError;
use aws_sdk_bedrockruntime::operation::converse_stream::ConverseStreamError;
use aws_sdk_bedrockruntime::{types as bedrock, Client};
use rmcp::model::Tool;
use serde_json::Value;

// Import the migrated helper functions from providers/formats/bedrock.rs
use super::formats::bedrock::{
    from_bedrock_message, from_bedrock_usage, sanitize_messages_for_bedrock, to_bedrock_message,
    to_bedrock_tool_config,
};
use async_stream::try_stream;
use mcp_core::ToolCall;
use rmcp::model::{ErrorCode, ErrorData};

pub const BEDROCK_DOC_LINK: &str =
    "https://docs.aws.amazon.com/bedrock/latest/userguide/models-supported.html";

pub const BEDROCK_DEFAULT_MODEL: &str = "anthropic.claude-sonnet-4-20250514-v1:0";
pub const BEDROCK_KNOWN_MODELS: &[&str] = &[
    "anthropic.claude-3-5-sonnet-20240620-v1:0",
    "anthropic.claude-3-5-sonnet-20241022-v2:0",
    "anthropic.claude-3-7-sonnet-20250219-v1:0",
    "anthropic.claude-sonnet-4-20250514-v1:0",
    "anthropic.claude-opus-4-20250514-v1:0",
    "anthropic.claude-opus-4-1-20250805-v1:0",
];

pub const BEDROCK_DEFAULT_MAX_RETRIES: usize = 6;
pub const BEDROCK_DEFAULT_INITIAL_RETRY_INTERVAL_MS: u64 = 2000;
pub const BEDROCK_DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;
pub const BEDROCK_DEFAULT_MAX_RETRY_INTERVAL_MS: u64 = 120_000;

#[derive(Debug, serde::Serialize)]
pub struct BedrockProvider {
    #[serde(skip)]
    client: Client,
    model: ModelConfig,
    #[serde(skip)]
    retry_config: RetryConfig,
}

impl BedrockProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        // Attempt to load config and secrets to get AWS_ prefixed keys
        // to re-export them into the environment for aws_config::load_from_env()
        let set_aws_env_vars = |res: Result<HashMap<String, Value>, _>| {
            if let Ok(map) = res {
                map.into_iter()
                    .filter(|(key, _)| key.starts_with("AWS_"))
                    .filter_map(|(key, value)| value.as_str().map(|s| (key, s.to_string())))
                    .for_each(|(key, s)| std::env::set_var(key, s));
            }
        };

        set_aws_env_vars(config.load_values());
        set_aws_env_vars(config.load_secrets());

        let sdk_config = futures::executor::block_on(aws_config::load_from_env());

        // validate credentials or return error back up
        futures::executor::block_on(
            sdk_config
                .credentials_provider()
                .unwrap()
                .provide_credentials(),
        )?;
        let client = Client::new(&sdk_config);

        let retry_config = Self::load_retry_config(config);

        Ok(Self {
            client,
            model,
            retry_config,
        })
    }

    fn load_retry_config(config: &crate::config::Config) -> RetryConfig {
        let max_retries = config
            .get_param::<usize>("BEDROCK_MAX_RETRIES")
            .unwrap_or(BEDROCK_DEFAULT_MAX_RETRIES);

        let initial_interval_ms = config
            .get_param::<u64>("BEDROCK_INITIAL_RETRY_INTERVAL_MS")
            .unwrap_or(BEDROCK_DEFAULT_INITIAL_RETRY_INTERVAL_MS);

        let backoff_multiplier = config
            .get_param::<f64>("BEDROCK_BACKOFF_MULTIPLIER")
            .unwrap_or(BEDROCK_DEFAULT_BACKOFF_MULTIPLIER);

        let max_interval_ms = config
            .get_param::<u64>("BEDROCK_MAX_RETRY_INTERVAL_MS")
            .unwrap_or(BEDROCK_DEFAULT_MAX_RETRY_INTERVAL_MS);

        RetryConfig {
            max_retries,
            initial_interval_ms,
            backoff_multiplier,
            max_interval_ms,
        }
    }

    async fn converse(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(bedrock::Message, Option<bedrock::TokenUsage>), ProviderError> {
        let model_name = &self.model.model_name;

        // Sanitize message history to avoid orphaned tool requests
        let sanitized = sanitize_messages_for_bedrock(messages);

        let mut request = self
            .client
            .converse()
            .system(bedrock::SystemContentBlock::Text(system.to_string()))
            .model_id(model_name.to_string())
            .set_messages(Some({
                let mut out: Vec<bedrock::Message> = Vec::new();
                for m in sanitized.iter() {
                    if let Some(bm) = to_bedrock_message(m)? {
                        out.push(bm);
                    }
                }
                out
            }));

        if !tools.is_empty() {
            request = request.tool_config(to_bedrock_tool_config(tools)?);
        }

        let response = request
            .send()
            .await
            .map_err(|err| match err.into_service_error() {
                ConverseError::ThrottlingException(throttle_err) => {
                    ProviderError::RateLimitExceeded(format!(
                        "Bedrock throttling error: {:?}",
                        throttle_err
                    ))
                }
                ConverseError::AccessDeniedException(err) => {
                    ProviderError::Authentication(format!("Failed to call Bedrock: {:?}", err))
                }
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
                err => ProviderError::ServerError(format!("Failed to call Bedrock: {:?}", err)),
            })?;

        match response.output {
            Some(bedrock::ConverseOutput::Message(message)) => Ok((message, response.usage)),
            _ => Err(ProviderError::RequestFailed(
                "No output from Bedrock".to_string(),
            )),
        }
    }
}

impl_provider_default!(BedrockProvider);

#[async_trait]
impl Provider for BedrockProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "aws_bedrock",
            "Amazon Bedrock",
            "Run models through Amazon Bedrock. You may have to set 'AWS_' environment variables to configure authentication.",
            BEDROCK_DEFAULT_MODEL,
            BEDROCK_KNOWN_MODELS.to_vec(),
            BEDROCK_DOC_LINK,
            vec![ConfigKey::new("AWS_PROFILE", true, false, Some("default"))],
        )
    }

    fn retry_config(&self) -> RetryConfig {
        self.retry_config.clone()
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let model_name = model_config.model_name.clone();

        let (bedrock_message, bedrock_usage) = self
            .with_retry(|| self.converse(system, messages, tools))
            .await?;

        let usage = bedrock_usage
            .as_ref()
            .map(from_bedrock_usage)
            .unwrap_or_default();

        let message = from_bedrock_message(&bedrock_message)?;

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

    fn supports_streaming(&self) -> bool {
        true
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let model_name = &self.model.model_name;

        // Build request similar to converse(), but using the streaming API
        // Sanitize message history to avoid orphaned tool requests
        let sanitized = sanitize_messages_for_bedrock(messages);

        let mut request = self
            .client
            .converse_stream()
            .system(bedrock::SystemContentBlock::Text(system.to_string()))
            .model_id(model_name.to_string())
            .set_messages(Some({
                let mut out: Vec<bedrock::Message> = Vec::new();
                for m in sanitized.iter() {
                    if let Some(bm) = to_bedrock_message(m)? {
                        out.push(bm);
                    }
                }
                out
            }));

        if !tools.is_empty() {
            request = request.tool_config(to_bedrock_tool_config(tools)?);
        }

        let response = request
            .send()
            .await
            .map_err(|err| match err.into_service_error() {
                ConverseStreamError::ThrottlingException(throttle_err) => {
                    ProviderError::RateLimitExceeded(format!(
                        "Bedrock throttling error: {:?}",
                        throttle_err
                    ))
                }
                ConverseStreamError::AccessDeniedException(err) => ProviderError::Authentication(
                    format!("Failed to call Bedrock ConverseStream: {:?}", err),
                ),
                ConverseStreamError::ValidationException(err)
                    if err
                        .message()
                        .unwrap_or_default()
                        .contains("Input is too long for requested model.") =>
                {
                    ProviderError::ContextLengthExceeded(format!(
                        "Failed to call Bedrock ConverseStream: {:?}",
                        err
                    ))
                }
                ConverseStreamError::ModelErrorException(err) => ProviderError::ExecutionError(
                    format!("Failed to call Bedrock ConverseStream: {:?}", err),
                ),
                err => ProviderError::ServerError(format!(
                    "Failed to call Bedrock ConverseStream: {:?}",
                    err
                )),
            })?;

        // We'll consume the EventReceiver in an async stream and map events to our MessageStream
        let mut receiver = response.stream;

        // We need to track in-progress tool use blocks by content_block_index
        use std::collections::HashMap;
        #[derive(Default)]
        struct ToolUseAgg {
            id: String,
            name: String,
            input: String,
        }
        let debug_payload = serde_json::json!({
            "system": system,
            "messages": messages,
            "tools": tools
        });
        let model_config = self.model.clone();
        let model_name_for_usage = model_name.to_string();

        Ok(Box::pin(try_stream! {
            let mut tool_blocks: HashMap<i32, ToolUseAgg> = HashMap::new();
            let empty_usage: super::base::Usage = Default::default();
            let stream_id = format!("bedrock-{}", uuid::Uuid::new_v4());

            loop {
                let next = receiver.recv().await;
                match next {
                    Ok(Some(event)) => {
                        use aws_sdk_bedrockruntime::types::ConverseStreamOutput as Ev;
                        match event {
                            Ev::ContentBlockDelta(delta_event) => {
                                let idx = delta_event.content_block_index();
                                if let Some(delta) = delta_event.delta() {
                                    use aws_sdk_bedrockruntime::types::ContentBlockDelta as CBD;
                                    match delta {
                                        CBD::Text(text_chunk) => {
                                            // Stream partial text as a message chunk
                                            let msg = Message::assistant()
                                                .with_text(text_chunk.clone())
                                                .with_id(stream_id.clone());
                                            emit_debug_trace(&model_config, &debug_payload, &msg, &empty_usage);
                                            yield (Some(msg), None);
                                        }
                                        CBD::ToolUse(tool_delta) => {
                                            // Accumulate tool input JSON string for this block
                                            let entry = tool_blocks.entry(idx).or_default();
                                            entry.input.push_str(tool_delta.input());
                                        }
                                        CBD::ReasoningContent(_reasoning) => {
                                            // We don't stream thinking content to the client; ignore
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Ev::ContentBlockStart(start_event) => {
                                if let Some(start) = start_event.start() {
                                    use aws_sdk_bedrockruntime::types::ContentBlockStart as CBS;
                                    if let CBS::ToolUse(start_block) = start {
                                        let idx = start_event.content_block_index();
                                        tool_blocks.insert(idx, ToolUseAgg {
                                            id: start_block.tool_use_id().to_string(),
                                            name: start_block.name().to_string(),
                                            input: String::new(),
                                        });
                                    }
                                }
                            }
                            Ev::ContentBlockStop(stop_event) => {
                                // Finalize tool call for this content block index if present
                                let idx = stop_event.content_block_index();
                                if let Some(agg) = tool_blocks.remove(&idx) {
                                    // Try to parse JSON arguments
                                    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&agg.input);
                                    let content = match parsed {
                                        Ok(args) => {
                                            let tool_call = ToolCall::new(agg.name.clone(), args);
                                            Message::assistant()
                                                .with_tool_request(agg.id.clone(), Ok(tool_call))
                                                .with_id(stream_id.clone())
                                        }
                                        Err(e) => {
                                            let err = ErrorData {
                                                code: ErrorCode::INTERNAL_ERROR,
                                                message: std::borrow::Cow::from(format!("Invalid tool input JSON: {}", e)),
                                                data: None,
                                            };
                                            Message::assistant()
                                                .with_tool_request(agg.id.clone(), Err(err))
                                                .with_id(stream_id.clone())
                                        }
                                    };
                                    emit_debug_trace(&model_config, &debug_payload, &content, &empty_usage);
                                    yield (Some(content), None);
                                }
                            }
                            Ev::MessageStart(_ms) => {
                                // No-op for now
                            }
                            Ev::MessageStop(_ms) => {
                                // No-op; usage comes via Metadata events below
                            }
                            Ev::Metadata(meta) => {
                                if let Some(usage) = meta.usage() {
                                    let usage = from_bedrock_usage(usage);
                                    let provider_usage = ProviderUsage::new(model_name_for_usage.clone(), usage);
                                    emit_debug_trace(&model_config, &debug_payload, &serde_json::json!({"metadata": "usage"}), &provider_usage.usage);
                                    yield (None, Some(provider_usage));
                                }
                            }
                            _ => { /* ignore other variants */ }
                        }
                    }
                    Ok(None) => break, // end of stream
                    Err(e) => {
                        // Transport or modeled event-stream error
                        Err(ProviderError::RequestFailed(format!("Bedrock ConverseStream error: {:?}", e)))?;
                    }
                }
            }
        }))
    }
}
