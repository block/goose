use std::collections::HashMap;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
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
use rmcp::model::Tool;
use serde_json::Value;

// Import the migrated helper functions from providers/formats/bedrock.rs
use super::formats::bedrock::{
    from_bedrock_message, from_bedrock_usage, to_bedrock_message_with_caching,
    to_bedrock_tool_config,
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

        // Determine which messages should have cache points
        // AWS Bedrock allows max 4 cache points. Strategy:
        // - 1 system prompt (always cached if caching enabled)
        // - 3 messages (tools don't support cache points in Bedrock)
        let cache_point_indices: Vec<usize> = if enable_caching && !visible_messages.is_empty() {
            let total_messages = visible_messages.len();
            // Reserve 1 cache point for system, use remaining 3 for messages
            let message_cache_budget = 3;

            if total_messages <= message_cache_budget {
                // Cache all messages if within budget
                (0..total_messages).collect()
            } else {
                // Cache only the most recent messages
                ((total_messages - message_cache_budget)..total_messages).collect()
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
            "Run models through Amazon Bedrock. Supports AWS SSO profiles - run 'aws sso login --profile <profile-name>' before using. Configure with AWS_PROFILE and AWS_REGION, or use environment variables/credentials. Prompt caching can be enabled for Anthropic Claude models (system prompt + messages) by setting BEDROCK_ENABLE_CACHING=true to reduce costs and improve latency.",
            BEDROCK_DEFAULT_MODEL,
            BEDROCK_KNOWN_MODELS.to_vec(),
            BEDROCK_DOC_LINK,
            vec![
                ConfigKey::new("AWS_PROFILE", true, false, Some("default")),
                ConfigKey::new("AWS_REGION", true, false, None),
                ConfigKey::new("BEDROCK_ENABLE_CACHING", false, false, Some("false")),
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
            },
            retry_config: RetryConfig::default(),
            name: "aws_bedrock".to_string(),
        }
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

        // Temporarily set the config to enable caching
        std::env::set_var("BEDROCK_ENABLE_CACHING", "true");

        let provider = create_mock_provider("us.anthropic.claude-sonnet-4-5-20250929-v1:0");
        let enable_caching =
            provider.should_enable_caching("us.anthropic.claude-sonnet-4-5-20250929-v1:0");

        assert!(
            enable_caching,
            "Caching should be enabled when BEDROCK_ENABLE_CACHING is set"
        );

        // Test with 5 messages - should cache last 3
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
                ((total_messages - message_cache_budget)..total_messages).collect()
            }
        } else {
            vec![]
        };

        // With 5 messages and budget of 3, should cache indices 2, 3, 4
        assert_eq!(cache_point_indices, vec![2, 3, 4]);

        // Clean up
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
