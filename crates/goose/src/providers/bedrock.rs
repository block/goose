use std::collections::HashMap;

use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use crate::providers::errors::ProviderError;
use crate::providers::retry::{ProviderRetry, RetryConfig};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::utils::RequestLog;
use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_bedrockruntime::config::ProvideCredentials;
use aws_sdk_bedrockruntime::operation::converse::ConverseError;
use aws_sdk_bedrockruntime::{types as bedrock, Client};
use futures::{Stream, StreamExt};
use rmcp::model::Tool;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use crate::providers::base::MessageStream;

// Import the migrated helper functions from providers/formats/bedrock.rs
use crate::providers::formats::bedrock::{
    from_bedrock_message, from_bedrock_usage, to_bedrock_message, to_bedrock_tool_config,
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
        // Set up the channel for streaming responses
        let (tx, rx) = mpsc::channel::<Result<(Option<Message>, Option<ProviderUsage>), ProviderError>>(100);
        let stream_receiver = ReceiverStream::new(rx);
        
        // Create the streaming task
        let task_tx = tx.clone();
        let client = self.client.clone();
        let model_name = self.model.model_name.clone();
        let system_prompt = system.to_string();
        let messages_clone = messages.to_vec();
        let tools_clone = tools.to_vec();
        
        tokio::spawn(async move {
            // Due to limitations or complexity with actual Bedrock streaming API,
            // we'll simulate the streaming behavior here by making the API call
            // and returning it as a single-stream event for compatibility
            
            let result = async {
                // Convert messages to Bedrock format
                let mut bedrock_messages = Vec::new();
                
                // Add system message if provided and not empty  
                if !system_prompt.is_empty() {
                    // Create proper Bedrock Message for system prompt
                    bedrock_messages.push(bedrock::Message::builder()
                        .role(bedrock::ConversationRole::User)
                        .content(bedrock::ContentBlock::Text(system_prompt))
                        .build()
                        .map_err(|e| anyhow::anyhow!("Failed to build system message: {}", e))?);
                }
                
                // Add user messages 
                for message in &messages_clone {
                    if message.is_agent_visible() {
                        bedrock_messages.push(to_bedrock_message(message)?);
                    }
                }

                // Prepare tool config if needed
                let tool_config = if !tools_clone.is_empty() {
                    Some(to_bedrock_tool_config(&tools_clone)?)
                } else {
                    None
                };

                // Make the converse call (similar to what complete_with_model does)
                let mut request = client
                    .converse()
                    .model_id(model_name.clone())
                    .set_messages(Some(bedrock_messages));

                if let Some(tc) = tool_config {
                    request = request.tool_config(tc);
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
                    Some(bedrock::ConverseOutput::Message(message)) => {
                        // Convert the Bedrock message to Goose message
                        let bedrock_message = message;
                        let bedrock_usage = response.usage;
                        
                        // Convert bedrock message to our format
                        let converted_message = from_bedrock_message(&bedrock_message)?;
                        
                        // Convert usage if present
                        let usage = bedrock_usage
                            .as_ref()
                            .map(from_bedrock_usage)
                            .unwrap_or_default();
                        
                        let provider_usage = ProviderUsage::new(model_name, usage);
                        
                        // Send the message and usage - this is how we simulate streaming
                        task_tx.send(Ok((Some(converted_message), Some(provider_usage)))).await.unwrap();
                        
                        // Also send a final None,None signal to indicate stream completion
                        task_tx.send(Ok((None, None))).await.unwrap();
                        
                        Ok::<(), ProviderError>(())
                    }
                    _ => {
                        task_tx.send(Err(ProviderError::RequestFailed("No valid output from Bedrock".to_string()))).await.unwrap();
                        Ok(())
                    }
                }
            }.await;
            
            if let Err(e) = result {
                task_tx.send(Err(e)).await.unwrap();
            }
        });
        
        Ok(Box::pin(stream_receiver))
    }
    
    fn supports_streaming(&self) -> bool {
        true  // Indicate that this Bedrock provider supports streaming
    }
}
