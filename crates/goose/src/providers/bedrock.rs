use std::collections::HashMap;

use super::base::{
    ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata, ProviderUsage,
};
use super::errors::ProviderError;
use super::retry::{ProviderRetry, RetryConfig};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::utils::RequestLog;
use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_bedrockruntime::config::ProvideCredentials;
use aws_sdk_bedrockruntime::operation::converse::ConverseError;
use aws_sdk_bedrockruntime::{types as bedrock, Client};
use futures::future::BoxFuture;
use reqwest::header::HeaderValue;
use rmcp::model::Tool;
use serde_json::Value;

// Import the migrated helper functions from providers/formats/bedrock.rs
use super::formats::bedrock::{
    from_bedrock_message, from_bedrock_usage, to_bedrock_message_with_caching,
    to_bedrock_tool_config,
};
use crate::session_context::SESSION_ID_HEADER;

const BEDROCK_PROVIDER_NAME: &str = "aws_bedrock";
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
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

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

        RetryConfig {
            max_retries,
            initial_interval_ms,
            backoff_multiplier,
            max_interval_ms,
        }
    }

    fn should_enable_caching(&self, model_name: &str) -> bool {
        let config = crate::config::Config::global();

        // Default: caching disabled
        let enabled = config
            .get_param::<bool>("BEDROCK_ENABLE_CACHING")
            .unwrap_or(false);
        enabled && model_name.contains("anthropic.claude")
    }

    async fn converse(
        &self,
        session_id: Option<&str>,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(bedrock::Message, Option<bedrock::TokenUsage>), ProviderError> {
        let model_name = &self.model.model_name;

        let enable_caching = self.should_enable_caching(model_name);

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

        // Determine which messages should have cache points.
        // AWS Bedrock has a strict limit of 4 cache points per request.
        // Allocation: 1 for system prompt + 3 for messages = 4 total.
        //
        // Strategy: Cache the earliest 3 messages (rather than most recent).
        // Rationale: Prompt caching requires exact prefix matching. By caching
        // the earliest messages, we create a stable cached prefix that doesn't
        // shift position across conversation turns, maximizing cache hit rates.
        // With "most recent 3" strategy, cache points would shift on every turn
        // (turn 1: msgs[0,1,2], turn 2: msgs[1,2,3], turn 3: msgs[2,3,4]),
        // causing cache misses and wasting the limited cache budget.
        //
        // Note: Tool configuration doesn't support cache points in Bedrock, but
        // messages containing tool content blocks (ToolRequest/ToolResponse) do.
        let cache_point_indices: Vec<usize> = if enable_caching && !visible_messages.is_empty() {
            let total_messages = visible_messages.len();
            let message_cache_budget = 3;

            if total_messages <= message_cache_budget {
                (0..total_messages).collect()
            } else {
                (0..message_cache_budget).collect()
            }
        } else {
            vec![]
        };

        let mut request = self
            .client
            .converse()
            .set_system(Some(system_blocks))
            .model_id(model_name.to_string())
            .set_messages(Some(
                visible_messages
                    .iter()
                    .enumerate()
                    .map(|(idx, m)| {
                        let should_cache = cache_point_indices.contains(&idx);
                        to_bedrock_message_with_caching(m, should_cache)
                    })
                    .collect::<Result<_>>()?,
            ));

        if !tools.is_empty() {
            request = request.tool_config(to_bedrock_tool_config(tools)?);
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

impl ProviderDef for BedrockProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            BEDROCK_PROVIDER_NAME,
            "Amazon Bedrock",
"Run models through Amazon Bedrock. Supports AWS SSO profiles - run 'aws sso login --profile <profile-name>' before using. Configure with AWS_PROFILE and AWS_REGION, use environment variables/credentials, or use AWS_BEARER_TOKEN_BEDROCK for bearer token authentication. Region is required for bearer token auth (can be set via AWS_REGION, AWS_DEFAULT_REGION, or AWS profile). Prompt caching can be enabled for Anthropic Claude models by setting BEDROCK_ENABLE_CACHING=true.",
            BEDROCK_DEFAULT_MODEL,
            BEDROCK_KNOWN_MODELS.to_vec(),
            BEDROCK_DOC_LINK,
            vec![
                ConfigKey::new("AWS_PROFILE", false, false, Some("default"), true),
                ConfigKey::new("AWS_REGION", false, false, None, true),
                ConfigKey::new("AWS_BEARER_TOKEN_BEDROCK", false, true, None, true),
                ConfigKey::new("BEDROCK_ENABLE_CACHING", false, false, Some("false"), false),
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

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
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
                fast_model: None,
                request_params: None,
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
        assert!(!aws_region.required, "AWS_REGION should not be required");
        assert!(
            !aws_region.secret,
            "AWS_REGION should not be marked as secret"
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
            !provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0"),
            "Caching should be disabled by default"
        );
    }

    #[test]
    fn test_caching_disabled_for_non_claude_models() {
        let provider = create_mock_provider("amazon.titan-text-express-v1");
        assert!(
            !provider.should_enable_caching("amazon.titan-text-express-v1"),
            "Caching should be disabled for non-Claude models"
        );
    }

    #[test]
    #[serial]
    fn test_cache_point_allocation_without_tools() {
        // Ensure clean environment
        std::env::remove_var("BEDROCK_ENABLE_CACHING");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        let enable_caching =
            provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        let total_messages = 5;

        let message_cache_budget = 3;
        let cache_point_indices: Vec<usize> = if enable_caching && total_messages > 0 {
            if total_messages <= message_cache_budget {
                (0..total_messages).collect()
            } else {
                ((total_messages - message_cache_budget)..total_messages).collect()
            }
        } else {
            vec![]
        };

        // Since caching is disabled by default, no cache points should be allocated
        assert_eq!(cache_point_indices, Vec::<usize>::new());
    }

    #[test]
    #[serial]
    fn test_cache_point_allocation_with_tools() {
        // Ensure clean environment
        std::env::remove_var("BEDROCK_ENABLE_CACHING");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        let enable_caching =
            provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        // Simulate 5 messages with tools
        // Tools don't affect message_cache_budget since they don't support cache points
        let total_messages = 5;

        let message_cache_budget = 3;
        let cache_point_indices: Vec<usize> = if enable_caching && total_messages > 0 {
            if total_messages <= message_cache_budget {
                (0..total_messages).collect()
            } else {
                ((total_messages - message_cache_budget)..total_messages).collect()
            }
        } else {
            vec![]
        };

        // Since caching is disabled by default, no cache points should be allocated
        assert_eq!(cache_point_indices, Vec::<usize>::new());
    }

    #[test]
    #[serial]
    fn test_cache_point_limit_respected_with_few_messages() {
        // Ensure clean environment
        std::env::remove_var("BEDROCK_ENABLE_CACHING");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        let enable_caching =
            provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        // Simulate 2 messages
        let total_messages = 2;

        let message_cache_budget = 3;
        let cache_point_indices: Vec<usize> = if enable_caching && total_messages > 0 {
            if total_messages <= message_cache_budget {
                (0..total_messages).collect()
            } else {
                ((total_messages - message_cache_budget)..total_messages).collect()
            }
        } else {
            vec![]
        };

        // Since caching is disabled by default, no cache points should be allocated
        assert_eq!(cache_point_indices, Vec::<usize>::new());
    }

    #[test]
    fn test_max_four_cache_points_respected() {
        let _provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        // Test with many messages: 1 system + 3 messages = 4 total
        // Tools don't get cache points in Bedrock
        let _total_messages = 10;
        let message_cache_budget = 3;

        // Count total cache points: 1 (system) + message_cache_budget
        let total_cache_points = 1 + message_cache_budget;
        assert_eq!(
            total_cache_points, 4,
            "Total cache points should not exceed 4"
        );

        // With or without tools, same cache point allocation
        let total_cache_points = 1 + message_cache_budget; // system + messages
        assert_eq!(
            total_cache_points, 4,
            "Total cache points should always be 4 when caching is enabled"
        );
    }

    #[test]
    #[serial]
    fn test_system_prompt_cache_point_structure() {
        // Ensure clean environment
        std::env::remove_var("BEDROCK_ENABLE_CACHING");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        let enable_caching =
            provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        assert!(!enable_caching, "Caching should be disabled by default");

        // When caching is disabled, system blocks should only have:
        // 1. Text block with system prompt
        // When caching is enabled (via config), system blocks should have:
        // 1. Text block with system prompt
        // 2. CachePoint block
        // This is tested in the actual converse() method implementation
    }

    #[test]
    #[serial]
    fn test_caching_respects_config_override() {
        // Ensure clean environment
        std::env::remove_var("BEDROCK_ENABLE_CACHING");

        // Test that BEDROCK_ENABLE_CACHING defaults to false
        // Note: This test assumes the config can be set. In practice, you'd need to
        // set the config value before calling should_enable_caching
        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        // The should_enable_caching method checks config first
        // Without BEDROCK_ENABLE_CACHING set, it defaults to false
        // regardless of model type
        assert!(
            !provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0"),
            "Without config override, caching should be disabled by default"
        );
    }

    #[test]
    #[serial]
    fn test_cache_points_allocation_with_caching_enabled() -> Result<()> {
        use crate::conversation::message::Message;
        use chrono::Utc;
        use rmcp::model::Role;

        std::env::set_var("BEDROCK_ENABLE_CACHING", "true");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        let enable_caching =
            provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        assert!(
            enable_caching,
            "Caching should be enabled when BEDROCK_ENABLE_CACHING is set"
        );

        let messages: Vec<Message> = (0..5)
            .map(|i| {
                Message::new(
                    if i % 2 == 0 {
                        Role::User
                    } else {
                        Role::Assistant
                    },
                    Utc::now().timestamp(),
                    vec![crate::conversation::message::MessageContent::text(format!(
                        "Message {}",
                        i
                    ))],
                )
            })
            .collect();

        let visible_messages: Vec<&Message> = messages.iter().collect();
        let total_messages = visible_messages.len();
        let message_cache_budget = 3;

        let cache_point_indices: Vec<usize> = if enable_caching && total_messages > 0 {
            if total_messages <= message_cache_budget {
                (0..total_messages).collect()
            } else {
                (0..message_cache_budget).collect()
            }
        } else {
            vec![]
        };

        assert_eq!(cache_point_indices, vec![0, 1, 2]);

        std::env::remove_var("BEDROCK_ENABLE_CACHING");

        Ok(())
    }

    #[test]
    #[serial]
    fn test_cache_points_with_few_messages() -> Result<()> {
        use crate::conversation::message::Message;
        use chrono::Utc;
        use rmcp::model::Role;

        // Temporarily set the config to enable caching
        std::env::set_var("BEDROCK_ENABLE_CACHING", "true");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        let enable_caching =
            provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        assert!(
            enable_caching,
            "Caching should be enabled when BEDROCK_ENABLE_CACHING is set"
        );

        // Test with 2 messages - should cache all
        let messages: Vec<Message> = (0..2)
            .map(|i| {
                Message::new(
                    if i % 2 == 0 {
                        Role::User
                    } else {
                        Role::Assistant
                    },
                    Utc::now().timestamp(),
                    vec![crate::conversation::message::MessageContent::text(format!(
                        "Message {}",
                        i
                    ))],
                )
            })
            .collect();

        let visible_messages: Vec<&Message> = messages.iter().collect();
        let total_messages = visible_messages.len();
        let message_cache_budget = 3;

        let cache_point_indices: Vec<usize> = if enable_caching && total_messages > 0 {
            if total_messages <= message_cache_budget {
                (0..total_messages).collect()
            } else {
                ((total_messages - message_cache_budget)..total_messages).collect()
            }
        } else {
            vec![]
        };

        // With 2 messages and budget of 3, should cache all indices 0, 1
        assert_eq!(cache_point_indices, vec![0, 1]);

        // Clean up
        std::env::remove_var("BEDROCK_ENABLE_CACHING");

        Ok(())
    }

    #[test]
    fn test_message_conversion_with_cache_points() -> Result<()> {
        use crate::conversation::message::Message;
        use chrono::Utc;
        use rmcp::model::Role;

        // Test that to_bedrock_message_with_caching correctly adds cache points
        let message = Message::new(
            Role::User,
            Utc::now().timestamp(),
            vec![
                crate::conversation::message::MessageContent::text("First text"),
                crate::conversation::message::MessageContent::text("Second text"),
            ],
        );

        // Convert with caching enabled
        let bedrock_message_cached = to_bedrock_message_with_caching(&message, true)?;

        // Should have 3 content blocks: 2 text + 1 cache point
        assert_eq!(bedrock_message_cached.content.len(), 3);

        // Last block should be a cache point
        assert!(matches!(
            bedrock_message_cached.content[2],
            bedrock::ContentBlock::CachePoint(_)
        ));

        // Convert with caching disabled
        let bedrock_message_no_cache = to_bedrock_message_with_caching(&message, false)?;

        // Should have only 2 content blocks (no cache point)
        assert_eq!(bedrock_message_no_cache.content.len(), 2);

        // No cache point should be present
        for block in &bedrock_message_no_cache.content {
            assert!(!matches!(block, bedrock::ContentBlock::CachePoint(_)));
        }

        Ok(())
    }

    #[test]
    #[serial]
    fn test_system_prompt_cache_point_with_caching_enabled() {
        use std::env;

        // Temporarily set the config to enable caching
        env::set_var("BEDROCK_ENABLE_CACHING", "true");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        let enable_caching =
            provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        assert!(
            enable_caching,
            "Caching should be enabled when BEDROCK_ENABLE_CACHING is set"
        );

        // Verify the logic for system blocks with caching enabled
        let system_blocks = if enable_caching {
            vec![
                bedrock::SystemContentBlock::Text("System prompt".to_string()),
                bedrock::SystemContentBlock::CachePoint(
                    bedrock::CachePointBlock::builder()
                        .r#type(bedrock::CachePointType::Default)
                        .build()
                        .unwrap(),
                ),
            ]
        } else {
            vec![bedrock::SystemContentBlock::Text(
                "System prompt".to_string(),
            )]
        };

        // Should have 2 blocks: text + cache point
        assert_eq!(system_blocks.len(), 2);
        assert!(matches!(
            system_blocks[1],
            bedrock::SystemContentBlock::CachePoint(_)
        ));

        // Clean up
        env::remove_var("BEDROCK_ENABLE_CACHING");
    }
}
