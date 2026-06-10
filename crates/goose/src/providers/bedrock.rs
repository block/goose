use std::collections::HashMap;

use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::formats::openai_responses::create_responses_request;
use super::openai_compatible::{handle_status, stream_responses_compat};
use super::retry::{ProviderRetry, RetryConfig};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::utils::RequestLog;
use crate::session_context::SESSION_ID_HEADER;
use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_bedrockruntime::config::ProvideCredentials;
use aws_sdk_bedrockruntime::operation::converse::ConverseError;
use aws_sdk_bedrockruntime::{types as bedrock, Client};
use futures::future::BoxFuture;
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::errors::ProviderError;
use goose_providers::formats::openai::extract_reasoning_effort;
use reqwest::header::{HeaderName, HeaderValue, AUTHORIZATION};
use rmcp::model::Tool;
use serde_json::Value;
use smithy_transport_reqwest::ReqwestHttpClient;

use super::formats::bedrock::{
    from_bedrock_message, from_bedrock_usage, to_bedrock_message_with_caching,
    to_bedrock_tool_config,
};

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
    "openai.gpt-5.5",
    "openai.gpt-5.4",
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
    #[serde(skip)]
    region: Option<String>,
    #[serde(skip)]
    bearer_token: Option<String>,
    #[serde(skip)]
    http_client: reqwest::Client,
    #[serde(skip)]
    mantle_base_url: Option<String>,
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

        let resolved_region = sdk_config.region().map(|r| r.to_string());

        let client = if let Some(ref token) = bearer_token {
            // Build from sdk_config to inherit all settings (endpoint overrides, timeouts, etc.)
            // then override authentication with bearer token
            let bedrock_config = aws_sdk_bedrockruntime::Config::new(&sdk_config)
                .to_builder()
                .bearer_token(aws_sdk_bedrockruntime::config::Token::new(
                    token.clone(),
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
            region: resolved_region,
            bearer_token,
            http_client: reqwest::Client::new(),
            mantle_base_url: None,
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

    async fn post_mantle_streaming(
        &self,
        session_id: Option<&str>,
        payload: &Value,
    ) -> Result<reqwest::Response, ProviderError> {
        let region = self.region.as_deref().ok_or_else(|| {
            ProviderError::Authentication(
                "AWS region is required for Bedrock mantle endpoint".to_string(),
            )
        })?;
        let token = self.bearer_token.as_deref().ok_or_else(|| {
            ProviderError::Authentication(
                "AWS_BEARER_TOKEN_BEDROCK is required for openai.gpt-* models".to_string(),
            )
        })?;

        let url = self.mantle_base_url.clone().unwrap_or_else(|| {
            format!(
                "https://bedrock-mantle.{}.api.aws/openai/v1/responses",
                region
            )
        });

        let mut req = self
            .http_client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .json(payload);

        if let Some(id) = session_id.filter(|id| !id.is_empty()) {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(SESSION_ID_HEADER.as_bytes()),
                HeaderValue::from_str(id),
            ) {
                req = req.header(name, value);
            }
        }

        let response = req
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Mantle request failed: {}", e)))?;

        handle_status(response).await
    }

    async fn converse(
        &self,
        session_id: Option<&str>,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(bedrock::Message, Option<bedrock::TokenUsage>), ProviderError> {
        let model_name = &self.model.model_name;

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
                        to_bedrock_message_with_caching(m, enable_caching && idx == last_idx)
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
                ConfigKey::new("AWS_REGION", true, false, Some("us-east-1"), true),
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

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let session_id_opt = if session_id.is_empty() {
            None
        } else {
            Some(session_id)
        };

        let without_prefix = model_config
            .model_name
            .strip_prefix("openai.")
            .unwrap_or(&model_config.model_name);
        let (base_name, _effort) = extract_reasoning_effort(without_prefix);
        let bedrock_model_id = format!("openai.{}", base_name);

        let is_mantle_model = BEDROCK_KNOWN_MODELS.contains(&bedrock_model_id.as_str());

        if is_mantle_model {
            let normalized_config = ModelConfig {
                model_name: base_name,
                ..model_config.clone()
            };
            let mut payload =
                create_responses_request(&normalized_config, system, messages, tools)?;
            payload["model"] = Value::String(bedrock_model_id.clone());
            payload["stream"] = Value::Bool(true);
            let mut log = RequestLog::start(model_config, &payload)?;

            let response = self
                .with_retry(|| self.post_mantle_streaming(session_id_opt, &payload))
                .await
                .inspect_err(|e| {
                    let _ = log.error(e);
                })?;

            return stream_responses_compat(response, log);
        }

        let model_name = model_config.model_name.clone();

        let (bedrock_message, bedrock_usage) = self
            .with_retry(|| self.converse(session_id_opt, system, messages, tools))
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
                fast_model_config: None,
                request_params: None,
                reasoning: None,
            },
            retry_config: RetryConfig::default(),
            name: "aws_bedrock".to_string(),
            region: None,
            bearer_token: None,
            http_client: reqwest::Client::new(),
            mantle_base_url: None,
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

    #[tokio::test]
    async fn test_post_mantle_streaming_missing_region() {
        let provider = create_mock_provider("openai.gpt-5.5");
        let payload = serde_json::json!({"model": "openai.gpt-5.5"});
        let result = provider.post_mantle_streaming(None, &payload).await;
        assert!(result.is_err());
        if let Err(ProviderError::Authentication(msg)) = result {
            assert!(
                msg.contains("region"),
                "Error message should mention region: {}",
                msg
            );
        } else {
            panic!("Expected ProviderError::Authentication");
        }
    }

    #[tokio::test]
    async fn test_post_mantle_streaming_missing_bearer_token() {
        let mut provider = create_mock_provider("openai.gpt-5.5");
        provider.region = Some("us-east-1".to_string());

        let payload = serde_json::json!({"model": "openai.gpt-5.5"});
        let result = provider.post_mantle_streaming(None, &payload).await;
        assert!(result.is_err());
        if let Err(ProviderError::Authentication(msg)) = result {
            assert!(
                msg.contains("AWS_BEARER_TOKEN_BEDROCK"),
                "Error message should mention AWS_BEARER_TOKEN_BEDROCK: {}",
                msg
            );
        } else {
            panic!("Expected ProviderError::Authentication");
        }
    }

    #[tokio::test]
    async fn test_mantle_stream_returns_text_message() {
        use crate::conversation::message::MessageContent;
        use futures::StreamExt;
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        let sse_body = [
            r#"data: {"type":"response.output_text.delta","sequence_number":1,"item_id":"msg_1","output_index":0,"content_index":0,"delta":"Hello"}"#,
            r#"data: {"type":"response.output_text.delta","sequence_number":2,"item_id":"msg_1","output_index":0,"content_index":0,"delta":" world"}"#,
            "data: [DONE]",
        ]
        .join("\n");

        Mock::given(method("POST"))
            .and(path("/openai/v1/responses"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
            .mount(&server)
            .await;

        let sdk_config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new("us-east-1"))
            .build();

        let provider = BedrockProvider {
            client: Client::new(&sdk_config),
            model: ModelConfig::new("openai.gpt-5.5").unwrap(),
            retry_config: RetryConfig::default(),
            name: "aws_bedrock".to_string(),
            region: Some("us-east-1".to_string()),
            bearer_token: Some("test-token".to_string()),
            http_client: reqwest::Client::new(),
            mantle_base_url: Some(format!("{}/openai/v1/responses", server.uri())),
        };

        let messages = vec![crate::conversation::message::Message::user().with_text("hi")];
        let mut stream = provider
            .stream(&provider.model.clone(), "", "", &messages, &[])
            .await
            .unwrap();

        let mut text = String::new();
        while let Some(item) = stream.next().await {
            let (msg, _usage) = item.unwrap();
            if let Some(m) = msg {
                for c in m.content {
                    if let MessageContent::Text(t) = c {
                        text.push_str(&t.text);
                    }
                }
            }
        }

        assert_eq!(text, "Hello world");

        let received = server.received_requests().await.unwrap();
        assert_eq!(received.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();
        assert_eq!(body["model"].as_str().unwrap(), "openai.gpt-5.5");
    }
}
