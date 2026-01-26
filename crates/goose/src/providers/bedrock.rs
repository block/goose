use std::collections::HashMap;

use crate::conversation::message::Message;
use crate::model::ModelConfig;
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

use crate::providers::base::MessageStream;
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

    pub fn model_info(&self, model: &str) -> crate::providers::base::ModelInfo {
        use crate::providers::base::ModelInfo;

        let (input_cost, output_cost) = match model {
            // Anthropic Claude models on Bedrock
            m if m.contains("claude-3-5-sonnet") || m.contains("claude-sonnet-4-5") => (3.0, 15.0),
            m if m.contains("claude-3-5-haiku") => (1.0, 5.0),
            m if m.contains("claude-3-haiku") => (0.25, 1.25),
            m if m.contains("claude-3-sonnet") || m.contains("claude-sonnet-3-7") => (3.0, 15.0),
            m if m.contains("claude-3-opus") || m.contains("claude-opus-4") => (15.0, 75.0),

            // Amazon Titan models
            m if m.contains("amazon.titan-text-premier") => (0.5, 1.5),
            m if m.contains("amazon.titan-text-express") => (0.2, 0.6),
            m if m.contains("amazon.titan-text-lite") => (0.15, 0.2),

            // Amazon Nova models
            m if m.contains("amazon.nova-pro") => (0.8, 3.2),
            m if m.contains("amazon.nova-lite") => (0.06, 0.24),
            m if m.contains("amazon.nova-micro") => (0.035, 0.14),

            // Meta Llama models
            m if m.contains("meta.llama3-1-405b") => (5.32, 16.0),
            m if m.contains("meta.llama3-1-70b") => (0.99, 2.97),
            m if m.contains("meta.llama3-1-8b") => (0.22, 0.66),
            m if m.contains("meta.llama3-2-90b") => (1.0, 3.0),
            m if m.contains("meta.llama3-2-11b") => (0.35, 1.05),
            m if m.contains("meta.llama3-2-3b") => (0.15, 0.45),
            m if m.contains("meta.llama3-2-1b") => (0.1, 0.3),

            // Cohere Command models
            m if m.contains("cohere.command-r-plus") => (3.0, 15.0),
            m if m.contains("cohere.command-r-v") => (0.5, 1.5),
            m if m.contains("cohere.command-light") => (0.3, 0.6),

            // AI21 Jurassic models
            m if m.contains("ai21.j2-ultra") || m.contains("ai21.jamba-1-5-large") => (18.8, 18.8),
            m if m.contains("ai21.j2-mid") || m.contains("ai21.jamba-1-5-mini") => (12.5, 12.5),

            // Mistral AI models
            m if m.contains("mistral.mistral-large-2") => (3.0, 9.0),
            m if m.contains("mistral.mistral-large") => (4.0, 12.0),
            m if m.contains("mistral.mistral-small") => (1.0, 3.0),
            m if m.contains("mistral.mixtral-8x7b") => (0.45, 0.7),

            // Default fallback
            _ => (0.0, 0.0),
        };

        ModelInfo {
            name: model.to_string(),
            context_limit: 0,
            input_token_cost: Some(input_cost),
            output_token_cost: Some(output_cost),
            currency: Some("$".to_string()),
            supports_cache_control: Some(false),
        }
    }

    pub fn estimate_cost(&self, usage: &crate::providers::base::Usage) -> Option<f64> {
        let model_info = self.model_info(&self.model.model_name);
        let input_cost = (usage.input_tokens.unwrap_or(0) as f64 / 1_000_000.0)
            * model_info.input_token_cost.unwrap_or(0.0);
        let output_cost = (usage.output_tokens.unwrap_or(0) as f64 / 1_000_000.0)
            * model_info.output_token_cost.unwrap_or(0.0);
        Some(input_cost + output_cost)
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        _session_id: &str,
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
        _session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        // Set up the channel for streaming responses
        let (tx, rx) =
            mpsc::channel::<Result<(Option<Message>, Option<ProviderUsage>), ProviderError>>(100);
        let stream_receiver = ReceiverStream::new(rx);

        // Create the streaming task
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
        true // Indicate that this Bedrock provider supports streaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::base::{Provider, Usage};
    use crate::providers::retry::RetryConfig;
    use std::sync::Arc;

    fn create_test_provider(model_name: &str) -> BedrockProvider {
        let config = aws_sdk_bedrockruntime::Config::builder()
            .behavior_version(aws_sdk_bedrockruntime::config::BehaviorVersion::latest())
            .region(aws_sdk_bedrockruntime::config::Region::new("us-east-1"))
            .credentials_provider(aws_sdk_bedrockruntime::config::Credentials::new(
                "test_access_key",
                "test_secret_key",
                None,
                None,
                "test",
            ))
            .build();
        BedrockProvider {
            client: aws_sdk_bedrockruntime::Client::from_conf(config),
            model: ModelConfig {
                model_name: model_name.to_string(),
                context_limit: Some(200_000),
                temperature: None,
                max_tokens: None,
                toolshim: false,
                toolshim_model: None,
                fast_model: None,
                request_params: None,
            },
            retry_config: RetryConfig::default(),
            name: "bedrock".to_string(),
        }
    }

    #[test]
    fn test_model_info_claude_3_5_sonnet() {
        let provider = create_test_provider("us.anthropic.claude-3-5-sonnet-20241022-v2:0");
        let info = provider.model_info(&provider.model.model_name);

        assert_eq!(info.name, "us.anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert_eq!(info.input_token_cost, Some(3.0));
        assert_eq!(info.output_token_cost, Some(15.0));
        assert_eq!(info.currency, Some("$".to_string()));
        assert_eq!(info.supports_cache_control, Some(false));
    }

    #[test]
    fn test_model_info_claude_3_opus() {
        let provider = create_test_provider("anthropic.claude-3-opus-20240229-v1:0");
        let info = provider.model_info(&provider.model.model_name);

        assert_eq!(info.input_token_cost, Some(15.0));
        assert_eq!(info.output_token_cost, Some(75.0));
    }

    #[test]
    fn test_model_info_claude_3_haiku() {
        let provider = create_test_provider("anthropic.claude-3-haiku-20240307-v1:0");
        let info = provider.model_info(&provider.model.model_name);

        assert_eq!(info.input_token_cost, Some(0.25));
        assert_eq!(info.output_token_cost, Some(1.25));
    }

    #[test]
    fn test_model_info_titan_text_premier() {
        let provider = create_test_provider("amazon.titan-text-premier-v1:0");
        let info = provider.model_info(&provider.model.model_name);

        assert_eq!(info.input_token_cost, Some(0.5));
        assert_eq!(info.output_token_cost, Some(1.5));
    }

    #[test]
    fn test_model_info_nova_pro() {
        let provider = create_test_provider("us.amazon.nova-pro-v1:0");
        let info = provider.model_info(&provider.model.model_name);

        assert_eq!(info.input_token_cost, Some(0.8));
        assert_eq!(info.output_token_cost, Some(3.2));
    }

    #[test]
    fn test_model_info_llama_3_1_405b() {
        let provider = create_test_provider("meta.llama3-1-405b-instruct-v1:0");
        let info = provider.model_info(&provider.model.model_name);

        assert_eq!(info.input_token_cost, Some(5.32));
        assert_eq!(info.output_token_cost, Some(16.0));
    }

    #[test]
    fn test_model_info_cohere_command_r_plus() {
        let provider = create_test_provider("cohere.command-r-plus-v1:0");
        let info = provider.model_info(&provider.model.model_name);

        assert_eq!(info.input_token_cost, Some(3.0));
        assert_eq!(info.output_token_cost, Some(15.0));
    }

    #[test]
    fn test_model_info_mistral_large() {
        let provider = create_test_provider("mistral.mistral-large-2407-v1:0");
        let info = provider.model_info(&provider.model.model_name);

        assert_eq!(info.input_token_cost, Some(3.0));
        assert_eq!(info.output_token_cost, Some(9.0));
    }

    #[test]
    fn test_model_info_unknown_model() {
        let provider = create_test_provider("unknown-model-xyz");
        let info = provider.model_info(&provider.model.model_name);

        assert_eq!(info.input_token_cost, Some(0.0));
        assert_eq!(info.output_token_cost, Some(0.0));
        assert_eq!(info.name, "unknown-model-xyz");
    }

    #[test]
    fn test_estimate_cost_claude_sonnet() {
        let provider = create_test_provider("anthropic.claude-3-5-sonnet-20241022-v2:0");

        let usage = Usage {
            input_tokens: Some(1_000_000),
            output_tokens: Some(1_000_000),
            total_tokens: Some(2_000_000),
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        let cost_value = cost.unwrap();
        assert_eq!(cost_value, 18.0);
    }

    #[test]
    fn test_estimate_cost_small_usage() {
        let provider = create_test_provider("anthropic.claude-3-5-sonnet-20241022-v2:0");

        let usage = Usage {
            input_tokens: Some(1000),
            output_tokens: Some(500),
            total_tokens: Some(1500),
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        let cost_value = cost.unwrap();
        assert!((cost_value - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_estimate_cost_zero_tokens() {
        let provider = create_test_provider("anthropic.claude-3-5-sonnet-20241022-v2:0");

        let usage = Usage {
            input_tokens: Some(0),
            output_tokens: Some(0),
            total_tokens: Some(0),
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        assert_eq!(cost.unwrap(), 0.0);
    }

    #[test]
    fn test_estimate_cost_none_tokens() {
        let provider = create_test_provider("anthropic.claude-3-5-sonnet-20241022-v2:0");

        let usage = Usage {
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        assert_eq!(cost.unwrap(), 0.0);
    }

    #[test]
    fn test_estimate_cost_only_input_tokens() {
        let provider = create_test_provider("anthropic.claude-3-5-sonnet-20241022-v2:0");

        let usage = Usage {
            input_tokens: Some(1_000_000),
            output_tokens: None,
            total_tokens: Some(1_000_000),
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        assert_eq!(cost.unwrap(), 3.0);
    }

    #[test]
    fn test_estimate_cost_only_output_tokens() {
        let provider = create_test_provider("anthropic.claude-3-5-sonnet-20241022-v2:0");

        let usage = Usage {
            input_tokens: None,
            output_tokens: Some(1_000_000),
            total_tokens: Some(1_000_000),
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        assert_eq!(cost.unwrap(), 15.0);
    }

    #[test]
    fn test_estimate_cost_unknown_model() {
        let provider = create_test_provider("unknown-model");

        let usage = Usage {
            input_tokens: Some(1_000_000),
            output_tokens: Some(1_000_000),
            total_tokens: Some(2_000_000),
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        assert_eq!(cost.unwrap(), 0.0);
    }

    #[test]
    fn test_estimate_cost_nova_lite() {
        let provider = create_test_provider("us.amazon.nova-lite-v1:0");

        let usage = Usage {
            input_tokens: Some(1_000_000),
            output_tokens: Some(1_000_000),
            total_tokens: Some(2_000_000),
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        let cost_value = cost.unwrap();
        assert_eq!(cost_value, 0.3);
    }

    #[test]
    fn test_estimate_cost_llama_8b() {
        let provider = create_test_provider("meta.llama3-1-8b-instruct-v1:0");

        let usage = Usage {
            input_tokens: Some(1_000_000),
            output_tokens: Some(1_000_000),
            total_tokens: Some(2_000_000),
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        let cost_value = cost.unwrap();
        assert_eq!(cost_value, 0.88);
    }

    #[test]
    fn test_estimate_cost_realistic_conversation() {
        let provider = create_test_provider("anthropic.claude-3-5-sonnet-20241022-v2:0");

        let usage = Usage {
            input_tokens: Some(2500),
            output_tokens: Some(1200),
            total_tokens: Some(3700),
        };

        let cost = provider.estimate_cost(&usage);

        assert!(cost.is_some());
        let cost_value = cost.unwrap();
        let expected = (2500.0 / 1_000_000.0 * 3.0) + (1200.0 / 1_000_000.0 * 15.0);
        assert!((cost_value - expected).abs() < 0.000001);
    }

    #[test]
    fn test_model_info_fields_complete() {
        let provider = create_test_provider("anthropic.claude-3-5-sonnet-20241022-v2:0");
        let info = provider.model_info(&provider.model.model_name);

        assert!(!info.name.is_empty());
        assert!(info.input_token_cost.is_some());
        assert!(info.output_token_cost.is_some());
        assert!(info.currency.is_some());
        assert!(info.supports_cache_control.is_some());
        assert_eq!(info.currency.unwrap(), "$");
        assert!(!info.supports_cache_control.unwrap());
    }

    #[test]
    fn test_as_any_downcast() {
        let provider = create_test_provider("anthropic.claude-3-5-sonnet-20241022-v2:0");
        let provider_arc: Arc<dyn Provider> = Arc::new(provider);

        let bedrock_provider = provider_arc.as_any().downcast_ref::<BedrockProvider>();

        assert!(bedrock_provider.is_some());

        let bedrock = bedrock_provider.unwrap();
        let info = bedrock.model_info("anthropic.claude-3-5-sonnet-20241022-v2:0");

        assert_eq!(info.input_token_cost, Some(3.0));
        assert_eq!(info.output_token_cost, Some(15.0));
    }
}
