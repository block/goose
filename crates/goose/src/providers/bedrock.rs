use std::collections::HashMap;

use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::MessageStream;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use crate::providers::errors::ProviderError;
use crate::providers::retry::{ProviderRetry, RetryConfig};
use crate::providers::utils::RequestLog;
use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_bedrockruntime::config::ProvideCredentials;
use aws_sdk_bedrockruntime::operation::converse::ConverseError;
use aws_sdk_bedrockruntime::operation::converse_stream::ConverseStreamError;
use aws_sdk_bedrockruntime::{types as bedrock, Client};
use rmcp::model::Tool;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

// Import the migrated helper functions from providers/formats/bedrock.rs
use crate::providers::formats::bedrock::{
    from_bedrock_message, from_bedrock_usage, to_bedrock_message, to_bedrock_tool_config,
    BedrockStreamAccumulator,
};

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

impl BedrockProvider {
    #[allow(clippy::type_complexity)]
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

        set_aws_env_vars(config.all_values());
        set_aws_env_vars(config.all_secrets());

        // Use load_defaults() which supports AWS SSO, profiles, and environment variables
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

        if let Ok(profile_name) = config.get_param::<String>("AWS_PROFILE") {
            if !profile_name.is_empty() {
                loader = loader.profile_name(&profile_name);
            }
        }

        // Check for AWS_REGION configuration
        if let Ok(region) = config.get_param::<String>("AWS_REGION") {
            if !region.is_empty() {
                loader = loader.region(aws_config::Region::new(region));
            }
        }

        let sdk_config = loader.load().await;

        // Validate credentials or return error back up
        sdk_config
            .credentials_provider()
            .ok_or_else(|| anyhow::anyhow!("No AWS credentials provider configured"))?
            .provide_credentials()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load AWS credentials: {}. Make sure to run 'aws sso login --profile <your-profile>' if using SSO", e))?;

        let client = Client::new(&sdk_config);

        let retry_config = Self::load_retry_config(config);

        Ok(Self {
            client,
            model,
            retry_config,
            name: Self::metadata().name,
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

        let mut request = self
            .client
            .converse()
            .system(bedrock::SystemContentBlock::Text(system.to_string()))
            .model_id(model_name.to_string())
            .set_messages(Some(
                messages
                    .iter()
                    .filter(|m| m.is_agent_visible())
                    .map(to_bedrock_message)
                    .collect::<Result<_>>()?,
            ));

        if !tools.is_empty() {
            request = request.tool_config(to_bedrock_tool_config(tools)?);
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

    #[allow(clippy::type_complexity)]
    async fn converse_stream_internal(
        client: &Client,
        model_name: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
        tx: mpsc::Sender<Result<(Option<Message>, Option<ProviderUsage>), ProviderError>>,
    ) -> Result<(), ProviderError> {
        let mut request = client.converse_stream().model_id(model_name.to_string());

        if !system.is_empty() {
            request = request.system(bedrock::SystemContentBlock::Text(system.to_string()));
        }

        let bedrock_messages: Vec<bedrock::Message> = messages
            .iter()
            .filter(|m| m.is_agent_visible())
            .map(to_bedrock_message)
            .collect::<Result<_>>()?;
        request = request.set_messages(Some(bedrock_messages));

        if !tools.is_empty() {
            request = request.tool_config(to_bedrock_tool_config(tools)?);
        }

        let response = request
            .send()
            .await
            .map_err(Self::map_converse_stream_error)?;
        let mut stream = response.stream;
        let mut accumulator = BedrockStreamAccumulator::new();

        loop {
            match stream.recv().await {
                Ok(Some(event)) => {
                    let maybe_message = match event {
                        bedrock::ConverseStreamOutput::MessageStart(msg_start) => {
                            accumulator.handle_message_start(&msg_start.role)?;
                            None
                        }
                        bedrock::ConverseStreamOutput::ContentBlockStart(block_start) => {
                            if let Some(start) = block_start.start {
                                accumulator.handle_content_block_start(
                                    block_start.content_block_index,
                                    &start,
                                )?;
                                None
                            } else {
                                None
                            }
                        }
                        bedrock::ConverseStreamOutput::ContentBlockDelta(delta_event) => {
                            if let Some(ref delta) = delta_event.delta {
                                let msg = accumulator.handle_content_block_delta(
                                    delta_event.content_block_index,
                                    delta,
                                )?;
                                tracing::debug!(
                                    "ContentBlockDelta produced message: {}",
                                    msg.is_some()
                                );
                                msg
                            } else {
                                None
                            }
                        }
                        bedrock::ConverseStreamOutput::ContentBlockStop(_) => None,
                        bedrock::ConverseStreamOutput::MessageStop(msg_stop) => {
                            let msg = accumulator.handle_message_stop(msg_stop.stop_reason)?;
                            tracing::debug!("MessageStop produced message: {}", msg.is_some());
                            msg
                        }
                        bedrock::ConverseStreamOutput::Metadata(metadata) => {
                            accumulator.handle_metadata(metadata.usage);
                            tracing::debug!("Received metadata");
                            None
                        }
                        _ => None,
                    };

                    if let Some(incremental_msg) = maybe_message {
                        tracing::debug!("Sending message through channel");
                        tx.send(Ok((Some(incremental_msg), None)))
                            .await
                            .map_err(|_| ProviderError::RequestFailed("Channel closed".into()))?;
                    }
                }
                Ok(None) => {
                    tracing::debug!("Stream ended");
                    break;
                }
                Err(e) => {
                    let error_msg = format!("Stream error: {:?}", e);
                    tracing::error!("{}", error_msg);
                    let provider_error = ProviderError::ServerError(error_msg);
                    let _ = tx.send(Err(provider_error)).await;
                    return Ok(());
                }
            }
        }

        if let Some(usage) = accumulator.get_usage() {
            let provider_usage = ProviderUsage::new(model_name.to_string(), usage);
            tracing::debug!("Sending final usage");
            tx.send(Ok((None, Some(provider_usage))))
                .await
                .map_err(|_| ProviderError::RequestFailed("Channel closed".into()))?;
        }

        tracing::debug!("Sending end marker");
        tx.send(Ok((None, None)))
            .await
            .map_err(|_| ProviderError::RequestFailed("Channel closed".into()))?;

        Ok(())
    }

    fn map_converse_stream_error(
        err: aws_sdk_bedrockruntime::error::SdkError<ConverseStreamError>,
    ) -> ProviderError {
        match err.into_service_error() {
            ConverseStreamError::ThrottlingException(throttle_err) => {
                ProviderError::RateLimitExceeded {
                    details: format!("Bedrock streaming throttling: {:?}", throttle_err),
                    retry_delay: None,
                }
            }
            ConverseStreamError::AccessDeniedException(err) => {
                ProviderError::Authentication(format!("Bedrock streaming access denied: {:?}", err))
            }
            ConverseStreamError::ValidationException(err)
                if err.message().unwrap_or_default().contains("too long") =>
            {
                ProviderError::ContextLengthExceeded(format!(
                    "Bedrock streaming context exceeded: {:?}",
                    err
                ))
            }
            ConverseStreamError::ModelStreamErrorException(err) => {
                ProviderError::ExecutionError(format!("Bedrock model streaming error: {:?}", err))
            }
            err => ProviderError::ServerError(format!("Bedrock streaming error: {:?}", err)),
        }
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "aws_bedrock",
            "Amazon Bedrock",
            "Run models through Amazon Bedrock. Supports AWS SSO profiles - run 'aws sso login --profile <profile-name>' before using. Configure with AWS_PROFILE and AWS_REGION, or use environment variables/credentials.",
            BEDROCK_DEFAULT_MODEL,
            BEDROCK_KNOWN_MODELS.to_vec(),
            BEDROCK_DOC_LINK,
            vec![
                ConfigKey::new("AWS_PROFILE", true, false, Some("default")),
                ConfigKey::new("AWS_REGION", true, false, None),
            ],
        )
    }

    fn get_name(&self) -> &str {
        &self.name
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
        let mut log = RequestLog::start(&self.model, &debug_payload)?;
        log.write(
            &serde_json::to_value(&message).unwrap_or_default(),
            Some(&usage),
        )?;

        let provider_usage = ProviderUsage::new(model_name.to_string(), usage);
        Ok((message, provider_usage))
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let (tx, rx) =
            mpsc::channel::<Result<(Option<Message>, Option<ProviderUsage>), ProviderError>>(100);
        let stream_receiver = ReceiverStream::new(rx);

        let client = self.client.clone();
        let model_name = self.model.model_name.clone();
        let system_prompt = system.to_string();
        let messages_clone = messages.to_vec();
        let tools_clone = tools.to_vec();

        tokio::spawn(async move {
            let result = Self::converse_stream_internal(
                &client,
                &model_name,
                &system_prompt,
                &messages_clone,
                &tools_clone,
                tx.clone(),
            )
            .await;

            if let Err(e) = result {
                let _ = tx.send(Err(e)).await;
            }
        });

        Ok(Box::pin(stream_receiver))
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
