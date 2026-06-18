use std::collections::HashMap;

use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::retry::{ProviderRetry, RetryConfig};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::utils::RequestLog;
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use aws_sdk_bedrockruntime::config::ProvideCredentials;
use aws_sdk_bedrockruntime::operation::converse::ConverseError;
use aws_sdk_bedrockruntime::operation::converse_stream::ConverseStreamError;
use aws_sdk_bedrockruntime::types::error::ConverseStreamOutputError;
use aws_sdk_bedrockruntime::{types as bedrock, Client};
use base64::Engine;
use futures::future::BoxFuture;
use goose_providers::conversation::token_usage::{ProviderUsage, Usage};
use goose_providers::errors::ProviderError;
use reqwest::header::HeaderValue;
use rmcp::model::{object, CallToolRequestParams, ErrorCode, ErrorData, Tool};
use serde_json::Value;
use smithy_transport_reqwest::ReqwestHttpClient;

use super::formats::bedrock::{
    bedrock_anthropic_thinking_fields, from_bedrock_message, from_bedrock_usage,
    to_bedrock_message_with_caching, to_bedrock_tool_config,
};
use crate::session_context::SESSION_ID_HEADER;

pub(crate) const BEDROCK_PROVIDER_NAME: &str = "aws_bedrock";
pub const BEDROCK_DOC_LINK: &str =
    "https://docs.aws.amazon.com/bedrock/latest/userguide/models-supported.html";

pub const BEDROCK_DEFAULT_MODEL: &str = "us.anthropic.claude-sonnet-4-5-20250929-v1:0";
pub const BEDROCK_KNOWN_MODELS: &[&str] = &[
    "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
    "us.anthropic.claude-sonnet-4-20250514-v1:0",
    "us.anthropic.claude-3-7-sonnet-20250219-v1:0",
    "us.anthropic.claude-opus-4-20250514-v1:0",
    "us.anthropic.claude-opus-4-1-20250805-v1:0",
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
    #[serde(skip)]
    name: String,
}

/// Request inputs shared by the `Converse` and `ConverseStream` APIs.
struct ConverseRequestParts {
    system_blocks: Vec<bedrock::SystemContentBlock>,
    messages: Vec<bedrock::Message>,
    tool_config: Option<bedrock::ToolConfiguration>,
    thinking_fields: Option<aws_smithy_types::Document>,
}

impl BedrockProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        // Attempt to load config and secrets to get AWS_ prefixed keys
        // to re-export them into the environment for aws_config to use as fallback
        let set_aws_env_vars = |res: Result<HashMap<String, Value>, _>| {
            if let Ok(map) = res {
                map.into_iter()
                    .filter(|(key, _)| key.starts_with("AWS_"))
                    .filter_map(|(key, value)| value.as_str().map(|s| (key, s.to_string())))
                    .for_each(|(key, s)| std::env::set_var(key, s));
            }
        };

        let filtered_secrets = config.all_secrets().map(|map| {
            map.into_iter()
                .filter(|(key, _)| key != "AWS_BEARER_TOKEN_BEDROCK")
                .collect()
        });

        set_aws_env_vars(config.all_values());
        set_aws_env_vars(filtered_secrets);

        // Check for bearer token first to determine if region is required
        let bearer_token = match config.get_secret::<String>("AWS_BEARER_TOKEN_BEDROCK") {
            Ok(token) => {
                let token = token.trim().to_string();
                if token.is_empty() {
                    None
                } else {
                    Some(token)
                }
            }
            Err(_) => None,
        };

        // Get AWS_REGION from config if explicitly set (optional - SDK can resolve from other sources)
        let region = match config.get_param::<String>("AWS_REGION") {
            Ok(r) if !r.is_empty() => Some(r),
            Ok(_) => None,
            Err(_) => None,
        };

        // Use load_defaults() which supports AWS SSO, profiles, and environment variables
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .http_client(ReqwestHttpClient::new());

        if let Ok(profile_name) = config.get_param::<String>("AWS_PROFILE") {
            if !profile_name.is_empty() {
                loader = loader.profile_name(&profile_name);
            }
        }

        // Apply region to loader if explicitly configured
        if let Some(ref region) = region {
            loader = loader.region(aws_config::Region::new(region.clone()));
        }

        let sdk_config = loader.load().await;

        // Validate region requirement for bearer token auth after SDK config is loaded
        // This allows region to be resolved from ~/.aws/config, AWS_DEFAULT_REGION, etc.
        if bearer_token.is_some() && sdk_config.region().is_none() {
            return Err(anyhow::anyhow!(
                "AWS region is required when using AWS_BEARER_TOKEN_BEDROCK authentication. \
                Set AWS_REGION, AWS_DEFAULT_REGION, or configure region in your AWS profile."
            ));
        }

        let client = if let Some(bearer_token) = bearer_token {
            // Build from sdk_config to inherit all settings (endpoint overrides, timeouts, etc.)
            // then override authentication with bearer token
            let bedrock_config = aws_sdk_bedrockruntime::Config::new(&sdk_config)
                .to_builder()
                .bearer_token(aws_sdk_bedrockruntime::config::Token::new(
                    bearer_token,
                    None,
                ))
                .build();

            Client::from_conf(bedrock_config)
        } else {
            Self::create_client_with_credentials(&sdk_config).await?
        };

        let retry_config = Self::load_retry_config(config);

        Ok(Self {
            client,
            model,
            retry_config,
            name: BEDROCK_PROVIDER_NAME.to_string(),
        })
    }

    async fn create_client_with_credentials(sdk_config: &aws_config::SdkConfig) -> Result<Client> {
        sdk_config
            .credentials_provider()
            .ok_or_else(|| anyhow::anyhow!("No AWS credentials provider configured"))?
            .provide_credentials()
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to load AWS credentials: {}. Make sure to run 'aws sso login --profile <your-profile>' if using SSO",
                    e
                )
            })?;

        Ok(Client::new(sdk_config))
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

        RetryConfig::new(
            max_retries,
            initial_interval_ms,
            backoff_multiplier,
            max_interval_ms,
        )
    }

    fn should_enable_caching(&self) -> bool {
        let config = crate::config::Config::global();

        let enabled = config
            .get_param::<bool>("BEDROCK_ENABLE_CACHING")
            .unwrap_or(false);
        enabled && self.model.model_name.contains("anthropic.claude")
    }

    /// Build the request inputs shared by [`Self::converse`] and
    /// [`Self::converse_stream`]: system blocks (with optional cache point),
    /// converted messages (with optional trailing-message cache point), and
    /// the tool configuration.
    fn build_request_parts(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<ConverseRequestParts, ProviderError> {
        let enable_caching = self.should_enable_caching();

        let system_blocks = if enable_caching {
            vec![
                bedrock::SystemContentBlock::Text(system.to_string()),
                // Add cache point AFTER the system prompt content
                bedrock::SystemContentBlock::CachePoint(
                    bedrock::CachePointBlock::builder()
                        .r#type(bedrock::CachePointType::Default)
                        .build()
                        .map_err(|e| {
                            ProviderError::ExecutionError(format!(
                                "Failed to build cache point: {}",
                                e
                            ))
                        })?,
                ),
            ]
        } else {
            vec![bedrock::SystemContentBlock::Text(system.to_string())]
        };

        let visible_messages: Vec<&Message> =
            messages.iter().filter(|m| m.is_agent_visible()).collect();

        let last_idx = visible_messages.len().saturating_sub(1);

        let bedrock_messages = visible_messages
            .iter()
            .enumerate()
            .map(|(idx, m)| to_bedrock_message_with_caching(m, enable_caching && idx == last_idx))
            .collect::<Result<Vec<_>>>()?;

        let tool_config = if tools.is_empty() {
            None
        } else {
            Some(to_bedrock_tool_config(tools)?)
        };

        Ok(ConverseRequestParts {
            system_blocks,
            messages: bedrock_messages,
            tool_config,
            thinking_fields: bedrock_anthropic_thinking_fields(&self.model),
        })
    }

    async fn converse(
        &self,
        session_id: Option<&str>,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(bedrock::Message, Option<bedrock::TokenUsage>), ProviderError> {
        let model_name = &self.model.model_name;

        let parts = self.build_request_parts(system, messages, tools)?;

        let mut request = self
            .client
            .converse()
            .set_system(Some(parts.system_blocks))
            .model_id(model_name.to_string())
            .set_messages(Some(parts.messages));

        if let Some(fields) = parts.thinking_fields {
            request = request.additional_model_request_fields(fields);
        }

        if let Some(tool_config) = parts.tool_config {
            request = request.tool_config(tool_config);
        }

        let mut request = request.customize();

        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            let session_id = session_id.to_string();
            request = request.mutate_request(move |req| {
                if let Ok(value) = HeaderValue::from_str(&session_id) {
                    req.headers_mut().insert(SESSION_ID_HEADER, value);
                }
            });
        }

        let response = request
            .send()
            .await
            .map_err(|err| match err.into_service_error() {
                ConverseError::ThrottlingException(throttle_err) => {
                    ProviderError::RateLimitExceeded {
                        details: format!("Bedrock throttling error: {:?}", throttle_err),
                        retry_delay: None,
                    }
                }
                ConverseError::AccessDeniedException(err) => {
                    ProviderError::Authentication(format!("Failed to call Bedrock: {:?}", err))
                }
                ConverseError::ValidationException(err)
                    if {
                        let msg = err.message().unwrap_or_default();
                        msg.contains("Input is too long for requested model.")
                            || msg.contains("prompt is too long")
                    } =>
                {
                    ProviderError::ContextLengthExceeded(format!(
                        "Failed to call Bedrock: {:?}",
                        err
                    ))
                }
                ConverseError::ValidationException(err) => ProviderError::ExecutionError(format!(
                    "Bedrock validation error: {}",
                    err.message().unwrap_or("unknown validation error")
                )),
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

    /// Escape hatch: `BEDROCK_DISABLE_STREAMING=true` restores the previous
    /// blocking `Converse` behaviour in case a model or region misbehaves
    /// with `ConverseStream`.
    fn streaming_disabled(&self) -> bool {
        let config = crate::config::Config::global();
        config
            .get_param::<bool>("BEDROCK_DISABLE_STREAMING")
            .unwrap_or(false)
    }

    /// Streaming variant of [`Self::converse`]. Builds an identical request
    /// but calls the AWS `ConverseStream` API, returning the raw event
    /// receiver so [`Provider::stream`] can forward deltas incrementally.
    async fn converse_stream(
        &self,
        session_id: Option<&str>,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<
        aws_sdk_bedrockruntime::operation::converse_stream::ConverseStreamOutput,
        ProviderError,
    > {
        let model_name = &self.model.model_name;

        let parts = self.build_request_parts(system, messages, tools)?;

        let mut request = self
            .client
            .converse_stream()
            .set_system(Some(parts.system_blocks))
            .model_id(model_name.to_string())
            .set_messages(Some(parts.messages));

        if let Some(fields) = parts.thinking_fields {
            request = request.additional_model_request_fields(fields);
        }

        if let Some(tool_config) = parts.tool_config {
            request = request.tool_config(tool_config);
        }

        let mut request = request.customize();

        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            let session_id = session_id.to_string();
            request = request.mutate_request(move |req| {
                if let Ok(value) = HeaderValue::from_str(&session_id) {
                    req.headers_mut().insert(SESSION_ID_HEADER, value);
                }
            });
        }

        request
            .send()
            .await
            .map_err(|err| match err.into_service_error() {
                ConverseStreamError::ThrottlingException(throttle_err) => {
                    ProviderError::RateLimitExceeded {
                        details: format!("Bedrock throttling error: {:?}", throttle_err),
                        retry_delay: None,
                    }
                }
                ConverseStreamError::AccessDeniedException(err) => {
                    ProviderError::Authentication(format!("Failed to call Bedrock: {:?}", err))
                }
                ConverseStreamError::ValidationException(err)
                    if {
                        let msg = err.message().unwrap_or_default();
                        msg.contains("Input is too long for requested model.")
                            || msg.contains("prompt is too long")
                    } =>
                {
                    ProviderError::ContextLengthExceeded(format!(
                        "Failed to call Bedrock: {:?}",
                        err
                    ))
                }
                ConverseStreamError::ModelErrorException(err) => {
                    ProviderError::ExecutionError(format!("Failed to call Bedrock: {:?}", err))
                }
                err => ProviderError::ServerError(format!("Failed to call Bedrock: {:?}", err)),
            })
    }

    /// Pre-ConverseStream behaviour: blocking `Converse` call wrapped in a
    /// single-item stream. Kept as the `BEDROCK_DISABLE_STREAMING=true`
    /// escape hatch.
    async fn stream_via_converse(
        &self,
        session_id: Option<&str>,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
        model_name: &str,
    ) -> Result<MessageStream, ProviderError> {
        let (bedrock_message, bedrock_usage) = self
            .with_retry(|| self.converse(session_id, system, messages, tools))
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
        let mut log = RequestLog::start(&self.model, &debug_payload)?;
        log.write(
            &serde_json::to_value(&message).unwrap_or_default(),
            Some(&usage),
        )?;

        let provider_usage = ProviderUsage::new(model_name.to_string(), usage);
        Ok(super::base::stream_from_single_message(
            message,
            provider_usage,
        ))
    }
}

/// Accumulation state for in-flight content blocks while consuming a
/// `ConverseStream` response. Tool inputs and reasoning content arrive as
/// fragments that only become a complete [`Message`] at `ContentBlockStop`.
#[derive(Default)]
struct StreamBlockState {
    /// content_block_index -> (tool_use_id, tool_name, accumulated input JSON)
    tool_blocks: HashMap<i32, (String, String, String)>,
    /// content_block_index -> (accumulated reasoning text, accumulated signature)
    reasoning_blocks: HashMap<i32, (String, String)>,
    /// content_block_index -> accumulated redacted (encrypted) reasoning bytes
    redacted_blocks: HashMap<i32, Vec<u8>>,
}

/// Convert a single `ConverseStream` event into zero or more [`Message`]s
/// ready to be yielded, plus token usage when the event carries it.
///
/// Mirrors the delta-yield contract of
/// `formats::anthropic::response_to_streaming_message`: text deltas yield
/// immediately (token-level chunks); tool-use inputs and reasoning blocks
/// accumulate in `state` until their `ContentBlockStop`.
fn process_stream_event(
    event: bedrock::ConverseStreamOutput,
    state: &mut StreamBlockState,
    message_id: &str,
) -> (Vec<Message>, Option<Usage>) {
    let mut messages = Vec::new();
    let mut usage = None;

    match event {
        bedrock::ConverseStreamOutput::ContentBlockStart(ev) => {
            if let Some(bedrock::ContentBlockStart::ToolUse(tu)) = ev.start {
                state.tool_blocks.insert(
                    ev.content_block_index,
                    (tu.tool_use_id, tu.name, String::new()),
                );
            }
        }
        bedrock::ConverseStreamOutput::ContentBlockDelta(ev) => match ev.delta {
            Some(bedrock::ContentBlockDelta::Text(text)) => {
                if !text.is_empty() {
                    messages.push(Message::assistant().with_text(text).with_id(message_id));
                }
            }
            Some(bedrock::ContentBlockDelta::ToolUse(tu)) => {
                if let Some(entry) = state.tool_blocks.get_mut(&ev.content_block_index) {
                    entry.2.push_str(&tu.input);
                }
            }
            Some(bedrock::ContentBlockDelta::ReasoningContent(rc)) => match rc {
                bedrock::ReasoningContentBlockDelta::Text(t) => {
                    state
                        .reasoning_blocks
                        .entry(ev.content_block_index)
                        .or_default()
                        .0
                        .push_str(&t);
                }
                bedrock::ReasoningContentBlockDelta::Signature(s) => {
                    state
                        .reasoning_blocks
                        .entry(ev.content_block_index)
                        .or_default()
                        .1
                        .push_str(&s);
                }
                bedrock::ReasoningContentBlockDelta::RedactedContent(blob) => {
                    state
                        .redacted_blocks
                        .entry(ev.content_block_index)
                        .or_default()
                        .extend_from_slice(blob.as_ref());
                }
                _ => {}
            },
            _ => {}
        },
        bedrock::ConverseStreamOutput::ContentBlockStop(ev) => {
            let idx = ev.content_block_index;
            if let Some((text, signature)) = state.reasoning_blocks.remove(&idx) {
                if !text.is_empty() {
                    messages.push(
                        Message::assistant()
                            .with_thinking(text, signature)
                            .with_id(message_id),
                    );
                }
            }
            if let Some(bytes) = state.redacted_blocks.remove(&idx) {
                if !bytes.is_empty() {
                    // Same base64 encoding as the non-streaming path
                    // (formats::bedrock::from_bedrock_reasoning_content_block)
                    // so redacted thinking round-trips back to Bedrock intact.
                    let encoded = base64::prelude::BASE64_STANDARD.encode(&bytes);
                    messages.push(
                        Message::assistant()
                            .with_redacted_thinking(encoded)
                            .with_id(message_id),
                    );
                }
            }
            if let Some((id, name, input_json)) = state.tool_blocks.remove(&idx) {
                // Parse the accumulated tool input. On failure, yield an
                // error tool request (not a stream error) so the agent can
                // report it back to the model — same behaviour as the
                // Anthropic provider.
                let tool_call = if input_json.trim().is_empty() {
                    Ok(CallToolRequestParams::new(name)
                        .with_arguments(object(serde_json::json!({}))))
                } else {
                    match serde_json::from_str::<Value>(&input_json) {
                        Ok(parsed) => {
                            Ok(CallToolRequestParams::new(name).with_arguments(object(parsed)))
                        }
                        Err(_) => Err(ErrorData::new(
                            ErrorCode::INVALID_PARAMS,
                            format!("Could not parse tool arguments: {}", input_json),
                            None,
                        )),
                    }
                };
                messages.push(
                    Message::assistant()
                        .with_tool_request(id, tool_call)
                        .with_id(message_id),
                );
            }
        }
        bedrock::ConverseStreamOutput::Metadata(ev) => {
            if let Some(u) = ev.usage {
                usage = Some(from_bedrock_usage(&u));
            }
        }
        // MessageStart / MessageStop / unknown variants carry no content
        // that needs forwarding.
        _ => {}
    }

    (messages, usage)
}

impl ProviderDef for BedrockProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            BEDROCK_PROVIDER_NAME,
            "Amazon Bedrock",
            "Run models through Amazon Bedrock. Supports AWS SSO profiles - run 'aws sso login --profile <profile-name>' before using. Configure with AWS_PROFILE and AWS_REGION, use environment variables/credentials, or use AWS_BEARER_TOKEN_BEDROCK for bearer token authentication. Region is required for bearer token auth (can be set via AWS_REGION, AWS_DEFAULT_REGION, or AWS profile). Prompt caching can be enabled for Anthropic Claude models by setting BEDROCK_ENABLE_CACHING=true. Responses stream via the ConverseStream API; set BEDROCK_DISABLE_STREAMING=true to fall back to blocking Converse calls.",
            BEDROCK_DEFAULT_MODEL,
            BEDROCK_KNOWN_MODELS.to_vec(),
            BEDROCK_DOC_LINK,
            vec![
                ConfigKey::new("AWS_PROFILE", false, false, Some("default"), true),
                ConfigKey::new("AWS_REGION", true, false, Some("us-east-1"), true),
                ConfigKey::new("AWS_BEARER_TOKEN_BEDROCK", false, true, None, true),
                ConfigKey::new("BEDROCK_ENABLE_CACHING", false, false, Some("false"), false),
                ConfigKey::new(
                    "BEDROCK_DISABLE_STREAMING",
                    false,
                    false,
                    Some("false"),
                    false,
                ),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn retry_config(&self) -> RetryConfig {
        self.retry_config.clone()
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(BEDROCK_KNOWN_MODELS.iter().map(|s| s.to_string()).collect())
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let session_id = if session_id.is_empty() {
            None
        } else {
            Some(session_id)
        };
        let model_name = model_config.model_name.clone();

        // Escape hatch: restore the previous blocking-Converse behaviour.
        if self.streaming_disabled() {
            return self
                .stream_via_converse(session_id, system, messages, tools, &model_name)
                .await;
        }

        // Open the AWS ConverseStream event stream. Retry wraps the request
        // setup only — mid-stream errors are surfaced, not retried (matching
        // the Anthropic provider's behaviour).
        let response = self
            .with_retry(|| self.converse_stream(session_id, system, messages, tools))
            .await?;

        // Debug trace with input context; the streamed text is written once
        // the stream completes.
        let debug_payload = serde_json::json!({
            "system": system,
            "messages": messages,
            "tools": tools
        });
        let mut log = RequestLog::start(&self.model, &debug_payload)?;

        let mut event_stream = response.stream;

        Ok(Box::pin(try_stream! {
            let mut state = StreamBlockState::default();
            // One id for the whole assistant turn so consumers
            // (Conversation::push) can coalesce consecutive deltas into a
            // single message — mirrors the Anthropic provider, which stamps
            // the API-provided message id on every chunk. Bedrock's
            // MessageStart event carries no id, so generate one.
            let message_id = format!("msg_{}", uuid::Uuid::new_v4());
            let mut full_text = String::new();
            let mut final_usage: Option<ProviderUsage> = None;

            loop {
                let event = event_stream.recv().await.map_err(|err| {
                    // Map Bedrock mid-stream exceptions to specific ProviderError
                    // variants so the agent's retry / context-length / server-error
                    // handling kicks in, mirroring the non-streaming Converse error
                    // mapping. Without this, a mid-stream throttling or
                    // context-length failure would be flattened to a generic
                    // RequestFailed and lose its retryable / context semantics.
                    match err.as_service_error() {
                        Some(ConverseStreamOutputError::ThrottlingException(e)) => {
                            ProviderError::RateLimitExceeded {
                                details: format!("Bedrock streaming throttling error: {:?}", e),
                                retry_delay: None,
                            }
                        }
                        Some(ConverseStreamOutputError::ValidationException(e))
                            if {
                                let msg = e.message().unwrap_or_default();
                                msg.contains("Input is too long for requested model.")
                                    || msg.contains("prompt is too long")
                            } =>
                        {
                            ProviderError::ContextLengthExceeded(format!(
                                "Bedrock streaming validation error: {:?}",
                                e
                            ))
                        }
                        Some(ConverseStreamOutputError::ServiceUnavailableException(_))
                        | Some(ConverseStreamOutputError::InternalServerException(_)) => {
                            ProviderError::ServerError(format!(
                                "Bedrock streaming server error: {:?}",
                                err
                            ))
                        }
                        Some(ConverseStreamOutputError::ModelStreamErrorException(e)) => {
                            ProviderError::ExecutionError(format!(
                                "Bedrock model stream error: {:?}",
                                e
                            ))
                        }
                        _ => ProviderError::RequestFailed(format!(
                            "Bedrock stream receive error: {:?}",
                            err
                        )),
                    }
                })?;
                let Some(event) = event else { break };

                let (messages, usage) = process_stream_event(event, &mut state, &message_id);
                if let Some(usage) = usage {
                    final_usage = Some(ProviderUsage::new(model_name.clone(), usage));
                }
                for message in messages {
                    if let Some(text) = message.content.first().and_then(|c| c.as_text()) {
                        full_text.push_str(text);
                    }
                    yield (Some(message), None);
                }
            }

            let usage = final_usage.unwrap_or_else(|| {
                ProviderUsage::new(model_name.clone(), Usage::default())
            });
            let _ = log.write(
                &serde_json::json!({ "streamed_text": full_text }),
                Some(&usage.usage),
            );
            yield (None, Some(usage));
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn create_mock_provider(model_name: &str) -> BedrockProvider {
        let sdk_config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new("us-east-1"))
            .build();
        let client = Client::new(&sdk_config);

        BedrockProvider {
            client,
            model: ModelConfig {
                model_name: model_name.to_string(),
                context_limit: None,
                temperature: None,
                max_tokens: None,
                toolshim: false,
                toolshim_model: None,
                fast_model_config: None,
                request_params: None,
                reasoning: None,
            },
            retry_config: RetryConfig::default(),
            name: "aws_bedrock".to_string(),
        }
    }

    #[test]
    fn test_metadata_config_keys_have_expected_flags() {
        let meta = BedrockProvider::metadata();

        let aws_profile = meta
            .config_keys
            .iter()
            .find(|k| k.name == "AWS_PROFILE")
            .expect("AWS_PROFILE config key should exist");
        assert!(!aws_profile.required, "AWS_PROFILE should not be required");
        assert!(
            !aws_profile.secret,
            "AWS_PROFILE should not be marked as secret"
        );

        let aws_region = meta
            .config_keys
            .iter()
            .find(|k| k.name == "AWS_REGION")
            .expect("AWS_REGION config key should exist");
        assert!(
            aws_region.required,
            "AWS_REGION is required for Bedrock to be marked as configured"
        );
        assert!(
            !aws_region.secret,
            "AWS_REGION should not be marked as secret"
        );
        assert!(
            aws_region.default.is_some(),
            "AWS_REGION should have a default value"
        );

        let bearer_token = meta
            .config_keys
            .iter()
            .find(|k| k.name == "AWS_BEARER_TOKEN_BEDROCK")
            .expect("AWS_BEARER_TOKEN_BEDROCK config key should exist");
        assert!(
            !bearer_token.required,
            "AWS_BEARER_TOKEN_BEDROCK should not be required"
        );
        assert!(
            bearer_token.secret,
            "AWS_BEARER_TOKEN_BEDROCK should be marked as secret"
        );

        let caching = meta
            .config_keys
            .iter()
            .find(|k| k.name == "BEDROCK_ENABLE_CACHING")
            .expect("BEDROCK_ENABLE_CACHING config key should exist");
        assert!(
            !caching.required,
            "BEDROCK_ENABLE_CACHING should not be required"
        );
        assert!(
            !caching.secret,
            "BEDROCK_ENABLE_CACHING should not be marked as secret"
        );
    }

    #[test]
    #[serial]
    fn test_caching_disabled_by_default() {
        // Ensure clean environment
        std::env::remove_var("BEDROCK_ENABLE_CACHING");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        assert!(
            !provider.should_enable_caching(),
            "Caching should be disabled by default"
        );
    }

    #[test]
    fn test_caching_disabled_for_non_claude_models() {
        let provider = create_mock_provider("amazon.titan-text-express-v1");
        assert!(
            !provider.should_enable_caching(),
            "Caching should be disabled for non-Claude models"
        );
    }

    #[test]
    #[serial]
    fn test_caching_enabled_for_claude_model() {
        std::env::set_var("BEDROCK_ENABLE_CACHING", "true");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        assert!(
            provider.should_enable_caching(),
            "Caching should be enabled for Claude models when BEDROCK_ENABLE_CACHING=true"
        );

        std::env::remove_var("BEDROCK_ENABLE_CACHING");
    }

    // ── ConverseStream event processing ──────────────────────────────────

    use crate::conversation::message::MessageContent;

    /// Stand-in for the per-turn message id that `stream()` generates.
    const TEST_MESSAGE_ID: &str = "msg_test";

    fn delta_event(idx: i32, delta: bedrock::ContentBlockDelta) -> bedrock::ConverseStreamOutput {
        bedrock::ConverseStreamOutput::ContentBlockDelta(
            bedrock::ContentBlockDeltaEvent::builder()
                .delta(delta)
                .content_block_index(idx)
                .build()
                .unwrap(),
        )
    }

    fn tool_start_event(idx: i32, id: &str, name: &str) -> bedrock::ConverseStreamOutput {
        bedrock::ConverseStreamOutput::ContentBlockStart(
            bedrock::ContentBlockStartEvent::builder()
                .start(bedrock::ContentBlockStart::ToolUse(
                    bedrock::ToolUseBlockStart::builder()
                        .tool_use_id(id)
                        .name(name)
                        .build()
                        .unwrap(),
                ))
                .content_block_index(idx)
                .build()
                .unwrap(),
        )
    }

    fn tool_delta_event(idx: i32, fragment: &str) -> bedrock::ConverseStreamOutput {
        delta_event(
            idx,
            bedrock::ContentBlockDelta::ToolUse(
                bedrock::ToolUseBlockDelta::builder()
                    .input(fragment)
                    .build()
                    .unwrap(),
            ),
        )
    }

    fn stop_event(idx: i32) -> bedrock::ConverseStreamOutput {
        bedrock::ConverseStreamOutput::ContentBlockStop(
            bedrock::ContentBlockStopEvent::builder()
                .content_block_index(idx)
                .build()
                .unwrap(),
        )
    }

    #[test]
    fn test_stream_text_delta_yields_immediately() {
        let mut state = StreamBlockState::default();
        let (messages, usage) = process_stream_event(
            delta_event(0, bedrock::ContentBlockDelta::Text("Hello".to_string())),
            &mut state,
            TEST_MESSAGE_ID,
        );
        assert_eq!(messages.len(), 1, "text delta should yield one message");
        assert_eq!(messages[0].as_concat_text(), "Hello");
        assert!(usage.is_none());
    }

    #[test]
    fn test_stream_empty_text_delta_yields_nothing() {
        let mut state = StreamBlockState::default();
        let (messages, _) = process_stream_event(
            delta_event(0, bedrock::ContentBlockDelta::Text(String::new())),
            &mut state,
            TEST_MESSAGE_ID,
        );
        assert!(messages.is_empty(), "empty text delta should be skipped");
    }

    #[test]
    fn test_stream_tool_use_accumulates_until_stop() {
        let mut state = StreamBlockState::default();

        let (messages, _) = process_stream_event(
            tool_start_event(1, "tool-1", "file_write"),
            &mut state,
            TEST_MESSAGE_ID,
        );
        assert!(messages.is_empty(), "tool start should not yield");

        // Input arrives as partial-JSON fragments
        for fragment in [r#"{"path": "#, r#""a.txt"}"#] {
            let (messages, _) =
                process_stream_event(tool_delta_event(1, fragment), &mut state, TEST_MESSAGE_ID);
            assert!(messages.is_empty(), "tool input fragments should not yield");
        }

        let (messages, _) = process_stream_event(stop_event(1), &mut state, TEST_MESSAGE_ID);
        assert_eq!(
            messages.len(),
            1,
            "tool stop should yield the complete request"
        );
        match &messages[0].content[0] {
            MessageContent::ToolRequest(req) => {
                assert_eq!(req.id, "tool-1");
                let call = req
                    .tool_call
                    .as_ref()
                    .expect("accumulated JSON should parse");
                assert_eq!(call.name.to_string(), "file_write");
                let args = call.arguments.as_ref().expect("arguments should be set");
                assert_eq!(args.get("path").and_then(|v| v.as_str()), Some("a.txt"));
            }
            other => panic!("expected ToolRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_stream_tool_use_invalid_json_yields_error_request() {
        let mut state = StreamBlockState::default();

        process_stream_event(
            tool_start_event(0, "tool-2", "shell"),
            &mut state,
            TEST_MESSAGE_ID,
        );
        process_stream_event(
            tool_delta_event(0, "this is {{{ not json"),
            &mut state,
            TEST_MESSAGE_ID,
        );

        let (messages, _) = process_stream_event(stop_event(0), &mut state, TEST_MESSAGE_ID);
        assert_eq!(messages.len(), 1);
        match &messages[0].content[0] {
            MessageContent::ToolRequest(req) => {
                assert!(
                    req.tool_call.is_err(),
                    "unparseable input should yield an error tool request, not a stream failure"
                );
            }
            other => panic!("expected ToolRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_stream_tool_use_empty_input_yields_empty_args() {
        let mut state = StreamBlockState::default();

        process_stream_event(
            tool_start_event(0, "tool-3", "list_files"),
            &mut state,
            TEST_MESSAGE_ID,
        );
        // No input deltas at all — some tools take no arguments.

        let (messages, _) = process_stream_event(stop_event(0), &mut state, TEST_MESSAGE_ID);
        assert_eq!(messages.len(), 1);
        match &messages[0].content[0] {
            MessageContent::ToolRequest(req) => {
                let call = req
                    .tool_call
                    .as_ref()
                    .expect("empty input should parse as {}");
                assert_eq!(call.name.to_string(), "list_files");
            }
            other => panic!("expected ToolRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_stream_reasoning_accumulates_until_stop() {
        let mut state = StreamBlockState::default();

        for (delta, expect_empty) in [
            (
                bedrock::ReasoningContentBlockDelta::Text("Let me think".to_string()),
                true,
            ),
            (
                bedrock::ReasoningContentBlockDelta::Text(" about this.".to_string()),
                true,
            ),
            (
                bedrock::ReasoningContentBlockDelta::Signature("sig-abc".to_string()),
                true,
            ),
        ] {
            let (messages, _) = process_stream_event(
                delta_event(0, bedrock::ContentBlockDelta::ReasoningContent(delta)),
                &mut state,
                TEST_MESSAGE_ID,
            );
            assert_eq!(
                messages.is_empty(),
                expect_empty,
                "reasoning deltas accumulate"
            );
        }

        let (messages, _) = process_stream_event(stop_event(0), &mut state, TEST_MESSAGE_ID);
        assert_eq!(
            messages.len(),
            1,
            "reasoning stop should yield thinking message"
        );
        match &messages[0].content[0] {
            MessageContent::Thinking(t) => {
                assert_eq!(t.thinking, "Let me think about this.");
                assert_eq!(t.signature, "sig-abc");
            }
            other => panic!("expected Thinking, got {:?}", other),
        }
    }

    #[test]
    fn test_stream_metadata_returns_usage() {
        let mut state = StreamBlockState::default();
        let event = bedrock::ConverseStreamOutput::Metadata(
            bedrock::ConverseStreamMetadataEvent::builder()
                .usage(
                    bedrock::TokenUsage::builder()
                        .input_tokens(100)
                        .output_tokens(50)
                        .total_tokens(150)
                        .build()
                        .unwrap(),
                )
                .build(),
        );
        let (messages, usage) = process_stream_event(event, &mut state, TEST_MESSAGE_ID);
        assert!(messages.is_empty());
        let usage = usage.expect("metadata event should carry usage");
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(50));
        assert_eq!(usage.total_tokens, Some(150));
    }

    #[test]
    fn test_stream_interleaved_text_and_tool_blocks() {
        // Bedrock interleaves block indices: text at index 0, tool at index 1.
        // Text yields immediately even while a tool block is mid-accumulation.
        let mut state = StreamBlockState::default();

        process_stream_event(
            tool_start_event(1, "tool-4", "search"),
            &mut state,
            TEST_MESSAGE_ID,
        );
        process_stream_event(tool_delta_event(1, r#"{"q":"#), &mut state, TEST_MESSAGE_ID);

        let (messages, _) = process_stream_event(
            delta_event(
                0,
                bedrock::ContentBlockDelta::Text("Searching now".to_string()),
            ),
            &mut state,
            TEST_MESSAGE_ID,
        );
        assert_eq!(
            messages.len(),
            1,
            "text should stream while tool accumulates"
        );
        assert_eq!(messages[0].as_concat_text(), "Searching now");

        process_stream_event(
            tool_delta_event(1, r#""rust"}"#),
            &mut state,
            TEST_MESSAGE_ID,
        );
        let (messages, _) = process_stream_event(stop_event(1), &mut state, TEST_MESSAGE_ID);
        assert_eq!(messages.len(), 1);
        match &messages[0].content[0] {
            MessageContent::ToolRequest(req) => {
                let call = req.tool_call.as_ref().unwrap();
                let args = call.arguments.as_ref().unwrap();
                assert_eq!(args.get("q").and_then(|v| v.as_str()), Some("rust"));
            }
            other => panic!("expected ToolRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_metadata_includes_disable_streaming_key() {
        let meta = BedrockProvider::metadata();

        let key = meta
            .config_keys
            .iter()
            .find(|k| k.name == "BEDROCK_DISABLE_STREAMING")
            .expect("BEDROCK_DISABLE_STREAMING config key should exist");
        assert!(
            !key.required,
            "BEDROCK_DISABLE_STREAMING should not be required"
        );
        assert!(
            !key.secret,
            "BEDROCK_DISABLE_STREAMING should not be marked as secret"
        );
        assert_eq!(
            key.default.as_deref(),
            Some("false"),
            "BEDROCK_DISABLE_STREAMING should default to false (streaming on)"
        );
    }

    #[test]
    fn test_stream_messages_carry_turn_message_id() {
        // Every message from one turn must share the caller-provided id so
        // Conversation::push can coalesce consecutive deltas instead of
        // persisting one message per token (same contract as the Anthropic
        // provider, which stamps the API message id on every chunk).
        let mut state = StreamBlockState::default();

        let (messages, _) = process_stream_event(
            delta_event(0, bedrock::ContentBlockDelta::Text("Hello".to_string())),
            &mut state,
            TEST_MESSAGE_ID,
        );
        assert_eq!(messages[0].id.as_deref(), Some(TEST_MESSAGE_ID));

        process_stream_event(
            tool_start_event(1, "tool-9", "shell"),
            &mut state,
            TEST_MESSAGE_ID,
        );
        let (messages, _) = process_stream_event(stop_event(1), &mut state, TEST_MESSAGE_ID);
        assert_eq!(
            messages[0].id.as_deref(),
            Some(TEST_MESSAGE_ID),
            "tool requests must carry the same turn id as text deltas"
        );
    }

    #[test]
    fn test_stream_redacted_reasoning_accumulates_until_stop() {
        let mut state = StreamBlockState::default();

        let raw = b"encrypted-reasoning-bytes";
        let (messages, _) = process_stream_event(
            delta_event(
                0,
                bedrock::ContentBlockDelta::ReasoningContent(
                    bedrock::ReasoningContentBlockDelta::RedactedContent(
                        aws_smithy_types::Blob::new(raw.to_vec()),
                    ),
                ),
            ),
            &mut state,
            TEST_MESSAGE_ID,
        );
        assert!(messages.is_empty(), "redacted deltas accumulate until stop");

        let (messages, _) = process_stream_event(stop_event(0), &mut state, TEST_MESSAGE_ID);
        assert_eq!(messages.len(), 1);
        match &messages[0].content[0] {
            MessageContent::RedactedThinking(redacted) => {
                let expected = base64::prelude::BASE64_STANDARD.encode(raw);
                assert_eq!(
                    redacted.data, expected,
                    "blob must round-trip as base64, matching the non-streaming path"
                );
            }
            other => panic!("expected RedactedThinking, got {:?}", other),
        }
    }
}
