use std::collections::HashMap;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
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
use aws_sdk_bedrockruntime::{types as bedrock, Client as RuntimeClient};
use aws_sdk_bedrock::Client;
use aws_sdk_bedrock::operation::list_inference_profiles::ListInferenceProfilesError;
use aws_sdk_bedrock::operation::list_foundation_models::ListFoundationModelsError;
use aws_config::SdkConfig;
use rmcp::model::Tool;
use serde_json::Value;

// Import the migrated helper functions from providers/formats/bedrock.rs
use super::formats::bedrock::{
    from_bedrock_message, from_bedrock_usage, to_bedrock_message, to_bedrock_tool_config,
};

pub const BEDROCK_DOC_LINK: &str =
    "https://docs.aws.amazon.com/bedrock/latest/userguide/models-supported.html";

// Picked this default since it's available on most regions
pub const BEDROCK_DEFAULT_MODEL: &str = "us.anthropic.claude-opus-4-20250514-v1:0";
// A selection of models that are available for on-demand inference
pub const BEDROCK_KNOWN_MODELS: &[&str] = &[
    "anthropic.claude-3-5-sonnet-20241022-v2:0",
    "us.anthropic.claude-opus-4-20250514-v1:0",
    "deepseek.v3-v1:0",
    "meta.llama3-1-405b-instruct-v1:0",
    "mistral.mixtral-8x7b-instruct-v0:1",
    "openai.gpt-oss-120b-1:0"
];

pub const BEDROCK_DEFAULT_MAX_RETRIES: usize = 6;
pub const BEDROCK_DEFAULT_INITIAL_RETRY_INTERVAL_MS: u64 = 2000;
pub const BEDROCK_DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;
pub const BEDROCK_DEFAULT_MAX_RETRY_INTERVAL_MS: u64 = 120_000;
pub const BEDROCK_MAX_MODEL_RESULTS: i32 = 500;

#[derive(Debug, serde::Serialize)]
pub struct BedrockProvider {
    #[serde(skip)]
    client: RuntimeClient,
    model: ModelConfig,
    #[serde(skip)]
    sdk_config: SdkConfig,
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
        let client = RuntimeClient::new(&sdk_config);

        let retry_config = Self::load_retry_config(config);

        Ok(Self {
            client,
            model,
            retry_config,
            sdk_config
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

    async fn fetch_inference_profiles(&self, profile_type: aws_sdk_bedrock::types::InferenceProfileType) -> Result<Vec<String>, ProviderError> {
        let bedrock_control_client = Client::new(&self.sdk_config);
        let response = bedrock_control_client
            .list_inference_profiles()
            .max_results(BEDROCK_MAX_MODEL_RESULTS)
            .type_equals(profile_type.clone())
            .send()
            .await
            .map_err(|err| { match err.into_service_error() {
                    ListInferenceProfilesError::AccessDeniedException(bedrock_err) => {
                        ProviderError::Authentication(format!("Failed to call Bedrock: {:?}", bedrock_err))
                    }
                    ListInferenceProfilesError::ThrottlingException(bedrock_err) => {
                        ProviderError::RateLimitExceeded(format!(
                            "Bedrock throttling error: {:?}",
                            bedrock_err
                        ))
                    }
                    err => ProviderError::ServerError(format!("Failed to call Bedrock: {:?}", err)),
                }
            });

        match response {
            Ok(profiles_result) => {
                let profiles: Vec<String> = match profiles_result.inference_profile_summaries {
                    Some(summaries) => {
                        summaries
                            .into_iter()
                            .map(|s| {
                                match profile_type {
                                    aws_sdk_bedrock::types::InferenceProfileType::SystemDefined => s.inference_profile_id().to_string(),
                                    aws_sdk_bedrock::types::InferenceProfileType::Application => s.inference_profile_arn().to_string(),
                                    _ => s.inference_profile_arn().to_string()
                                }
                            })
                            .collect()
                    }
                    None => {
                        Vec::new()
                    }
                };

                Ok(profiles)
            }
            Err(err) => {
                return Err(err)
            }
        }
    }

    async fn fetch_foundation_models(&self) -> Result<Vec<String>, ProviderError> {
        let bedrock_control_client = Client::new(&self.sdk_config);
        let response = bedrock_control_client
            .list_foundation_models()
            .by_inference_type(aws_sdk_bedrock::types::InferenceType::OnDemand)
            .send()
            .await
            .map_err(|err| { match err.into_service_error() {
                    ListFoundationModelsError::AccessDeniedException(bedrock_err) => {
                        ProviderError::Authentication(format!("Failed to call Bedrock: {:?}", bedrock_err))
                    }
                    ListFoundationModelsError::ThrottlingException(bedrock_err) => {
                        ProviderError::RateLimitExceeded(format!(
                            "Bedrock throttling error: {:?}",
                            bedrock_err
                        ))
                    }
                    err => ProviderError::ServerError(format!("Failed to call Bedrock: {:?}", err)),
                }
            });

        match response {
            Ok(models_result) => {
                let models: Vec<String> = match models_result.model_summaries {
                    Some(summaries) => {
                        summaries
                            .into_iter()
                            .map(|s| {
                                s.model_id().to_string()
                            })
                            .collect()
                    }
                    None => {
                        Vec::new()
                    }
                };

                Ok(models)
            }
            Err(err) => {
                return Err(err)
            }
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

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let mut application_profiles = match self.fetch_inference_profiles(aws_sdk_bedrock::types::InferenceProfileType::Application).await {
            Ok(profiles) => profiles,
            Err(profile_error) => return Err(profile_error)
        };

        let mut system_profiles = match self.fetch_inference_profiles(aws_sdk_bedrock::types::InferenceProfileType::SystemDefined).await {
            Ok(profiles) => profiles,
            Err(profile_error) => return Err(profile_error)
        };

        let mut foundation_models = match self.fetch_foundation_models().await {
            Ok(models) => models,
            Err(model_error) => return Err(model_error)
        };

        // System profiles and foundation models are invokable
        // by ID not ARN. But some models have a profile and
        // some don't. Some models require the profile, so they
        // aren't in the bare list of foundation models. As a result,
        // we need to combine the two lists and dedupe.
        system_profiles.append(&mut foundation_models);
        
        system_profiles.sort_unstable();
        system_profiles.dedup();

        // We want application inference profiles before
        // the list of available on demand models since
        // they are user/account created and more likely
        // to be what they want to use.
        application_profiles.sort_unstable();        
        application_profiles.append(&mut system_profiles);

        Ok(Some(application_profiles))
    }
}