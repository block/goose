use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

use super::base::{Moderation, ModerationResult, Provider, ProviderUsage, Usage};
use super::configs::ModelConfig;
use super::model_pricing::{cost, model_pricing_for};
use super::oauth;
use super::utils::{check_bedrock_context_length_error, get_model, handle_response, ImageFormat};
use crate::message::Message;
use crate::providers::openai_utils::{
    check_openai_context_length_error, get_openai_usage, messages_to_openai_spec,
    openai_response_to_message, tools_to_openai_spec,
};
use mcp_core::tool::Tool;

const DEFAULT_CLIENT_ID: &str = "databricks-cli";
const DEFAULT_REDIRECT_URL: &str = "http://localhost:8020";
const DEFAULT_SCOPES: &[&str] = &["all-apis"];
pub const DATABRICKS_DEFAULT_MODEL: &str = "claude-3-5-sonnet-2";
pub const INPUT_GUARDRAIL: &str = "input_guardrail";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabricksAuth {
    Token(String),
    OAuth {
        host: String,
        client_id: String,
        redirect_url: String,
        scopes: Vec<String>,
    },
}

impl DatabricksAuth {
    /// Create a new OAuth configuration with default values
    pub fn oauth(host: String) -> Self {
        Self::OAuth {
            host,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            redirect_url: DEFAULT_REDIRECT_URL.to_string(),
            scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }
    pub fn token(token: String) -> Self {
        Self::Token(token)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DatabricksProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    auth: DatabricksAuth,
    model: ModelConfig,
    image_format: ImageFormat,
}

impl DatabricksProvider {
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("DATABRICKS_HOST")
            .unwrap_or_else(|_| "https://api.databricks.com".to_string());
        let model_name = std::env::var("DATABRICKS_MODEL")
            .unwrap_or_else(|_| DATABRICKS_DEFAULT_MODEL.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        // If we find a databricks token we prefer that
        if let Ok(api_key) =
            crate::key_manager::get_keyring_secret("DATABRICKS_TOKEN", Default::default())
        {
            return Ok(Self {
                client,
                host: host.clone(),
                auth: DatabricksAuth::token(api_key),
                model: ModelConfig::new(model_name),
                image_format: ImageFormat::Anthropic,
            });
        }

        // Otherwise use Oauth flow
        Ok(Self {
            client,
            host: host.clone(),
            auth: DatabricksAuth::oauth(host),
            model: ModelConfig::new(model_name),
            image_format: ImageFormat::Anthropic,
        })
    }

    async fn ensure_auth_header(&self) -> Result<String> {
        match &self.auth {
            DatabricksAuth::Token(token) => Ok(format!("Bearer {}", token)),
            DatabricksAuth::OAuth {
                host,
                client_id,
                redirect_url,
                scopes,
            } => {
                let token =
                    oauth::get_oauth_token_async(host, client_id, redirect_url, scopes).await?;
                Ok(format!("Bearer {}", token))
            }
        }
    }

    async fn post(&self, payload: Value) -> Result<Value> {
        let url = format!(
            "{}/serving-endpoints/{}/invocations",
            self.host.trim_end_matches('/'),
            self.model.model_name
        );

        let auth_header = self.ensure_auth_header().await?;
        let response = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .json(&payload)
            .send()
            .await?;

        handle_response(payload, response).await
    }

    async fn handle_moderation_response(&self, response: reqwest::Response) -> Result<Value> {
        match response.status() {
            reqwest::StatusCode::OK => {
                let payload = response.json().await?;
                Ok(payload)
            }
            reqwest::StatusCode::BAD_REQUEST => {
                let error_body: Value = response.json().await?;

                // Check if this is a moderation error
                if let Some(finish_reason) = error_body.get("finishReason") {
                    if finish_reason == "input_guardrail_triggered" {
                        return Ok(error_body);
                    }
                }
                // Not a moderation error, return the original error
                Err(anyhow::anyhow!("Bad request: {}", error_body))
            }
            status => {
                let error_body: Value = response.json().await?;
                Err(anyhow::anyhow!(
                    "Moderation request failed with status: {}\nPayload {}",
                    status,
                    error_body
                ))
            }
        }
    }
}

#[async_trait]
impl Provider for DatabricksProvider {
    fn get_model_config(&self) -> &ModelConfig {
        &self.model
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(
            model_config,
            input,
            output,
            input_tokens,
            output_tokens,
            total_tokens,
            cost
        )
    )]
    async fn complete_internal(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage)> {
        // Prepare messages and tools
        let concat_tool_response_contents = false;
        let messages_spec =
            messages_to_openai_spec(messages, &self.image_format, concat_tool_response_contents);
        let tools_spec = if !tools.is_empty() {
            tools_to_openai_spec(tools)?
        } else {
            vec![]
        };

        // Build payload with system message
        let mut messages_array = vec![json!({ "role": "system", "content": system })];
        messages_array.extend(messages_spec);

        let mut payload = json!({ "messages": messages_array });

        // Add optional parameters
        if !tools_spec.is_empty() {
            payload["tools"] = json!(tools_spec);
        }
        if let Some(temp) = self.model.temperature {
            payload["temperature"] = json!(temp);
        }
        if let Some(tokens) = self.model.max_tokens {
            payload["max_tokens"] = json!(tokens);
        }

        // Remove null values
        let payload = serde_json::Value::Object(
            payload
                .as_object()
                .unwrap()
                .iter()
                .filter(|&(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        // Make request
        let response = self.post(payload.clone()).await?;

        // Raise specific error if context length is exceeded
        if let Some(error) = response.get("error") {
            if let Some(err) = check_openai_context_length_error(error) {
                return Err(err.into());
            } else if let Some(err) = check_bedrock_context_length_error(error) {
                return Err(err.into());
            }
            return Err(anyhow!("Databricks API error: {}", error));
        }

        // Parse response
        let message = openai_response_to_message(response.clone())?;
        let usage = self.get_usage(&response)?;
        let model = get_model(&response);
        let cost = cost(&usage, &model_pricing_for(&model));
        super::utils::emit_debug_trace(self, &payload, &response, &usage, cost);

        Ok((message, ProviderUsage::new(model, usage, cost)))
    }

    fn get_usage(&self, data: &Value) -> Result<Usage> {
        get_openai_usage(data)
    }
}

#[async_trait]
impl Moderation for DatabricksProvider {
    async fn moderate_content_internal(&self, content: &str) -> Result<ModerationResult> {
        let url = format!(
            "{}/serving-endpoints/moderation/invocations",
            self.host.trim_end_matches('/')
        );

        let auth_header = self.ensure_auth_header().await?;
        let payload = json!({
            "messages": [
                {
                    "role": "user",
                    "content": content
                }
            ]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .json(&payload)
            .send()
            .await?;

        // let response: Value = response.json().await?;
        // let response = handle_response(payload, response).await??;
        let response = self.handle_moderation_response(response).await?;

        // Check if we got a moderation result
        if let Some(input_guardrail) = response.get(INPUT_GUARDRAIL) {
            if let Some(first_result) = input_guardrail.as_array().and_then(|arr| arr.first()) {
                if let Some(flagged) = first_result.get("flagged").and_then(|f| f.as_bool()) {
                    // Extract categories if they exist and if content is flagged
                    let categories = if flagged {
                        first_result
                            .get("categories")
                            .and_then(|cats| cats.as_object())
                            .map(|cats| {
                                cats.iter()
                                    .filter(|(_, v)| v.as_bool().unwrap_or(false))
                                    .map(|(k, _)| k.to_string())
                                    .collect::<Vec<_>>()
                            })
                    } else {
                        None
                    };

                    return Ok(ModerationResult::new(flagged, categories, None));
                }
            }
        }

        // If we get here, there was no moderation result, so the content is considered safe
        Ok(ModerationResult::new(false, None, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use crate::providers::mock_server::{
        create_mock_open_ai_response, TEST_INPUT_TOKENS, TEST_OUTPUT_TOKENS, TEST_TOTAL_TOKENS,
    };

    use serde_json::json;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_provider(mock_server: &MockServer) -> DatabricksProvider {
        DatabricksProvider {
            client: Client::builder().build().unwrap(),
            host: mock_server.uri(),
            auth: DatabricksAuth::Token("test_token".to_string()),
            model: ModelConfig::new("my-databricks-model".to_string()),
            image_format: ImageFormat::Anthropic,
        }
    }

    #[tokio::test]
    async fn test_moderation_flagged_content() -> Result<()> {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Mock response for moderation with flagged content
        let mock_response = json!({
            "usage": {
                "prompt_tokens": 199,
                "total_tokens": 199
            },
            "input_guardrail": [{
                "flagged": true,
                "categories": {
                    "violent-crimes": false,
                    "non-violent-crimes": true,
                    "sex-crimes": false,
                    "child-exploitation": false,
                    "specialized-advice": false,
                    "privacy": false,
                    "intellectual-property": false,
                    "indiscriminate-weapons": false,
                    "hate": false,
                    "self-harm": false,
                    "sexual-content": false
                }
            }],
            "finishReason": "input_guardrail_triggered"
        });

        // Set up the mock
        Mock::given(method("POST"))
            .and(path("/serving-endpoints/moderation/invocations"))
            .and(header("Authorization", "Bearer test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);

        // Test moderation
        let result = provider.moderate_content("test content").await?;

        assert!(result.flagged);
        assert_eq!(result.categories.unwrap(), vec!["non-violent-crimes"]);
        assert!(result.category_scores.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_moderation_safe_content() -> Result<()> {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Mock response for safe content (regular chat response)
        let mock_response = json!({
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            },
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "This is a safe response"
                },
                "finish_reason": "stop"
            }]
        });

        // Set up the mock
        Mock::given(method("POST"))
            .and(path("/serving-endpoints/moderation/invocations"))
            .and(header("Authorization", "Bearer test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);

        // Test moderation
        let result = provider.moderate_content("safe content").await?;

        assert!(!result.flagged);
        assert!(result.categories.is_none());
        assert!(result.category_scores.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_moderation_explicit_safe() -> Result<()> {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Mock response for explicitly safe content
        let mock_response = json!({
            "usage": {
                "prompt_tokens": 199,
                "total_tokens": 199
            },
            "input_guardrail": [{
                "flagged": false,
                "categories": {
                    "violent-crimes": false,
                    "non-violent-crimes": false,
                    "sex-crimes": false,
                    "child-exploitation": false,
                    "specialized-advice": false,
                    "privacy": false,
                    "intellectual-property": false,
                    "indiscriminate-weapons": false,
                    "hate": false,
                    "self-harm": false,
                    "sexual-content": false
                }
            }]
        });

        // Set up the mock
        Mock::given(method("POST"))
            .and(path("/serving-endpoints/moderation/invocations"))
            .and(header("Authorization", "Bearer test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);

        // Test moderation
        let result = provider.moderate_content("explicitly safe content").await?;

        assert!(!result.flagged);
        assert!(result.categories.is_none());
        assert!(result.category_scores.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_databricks_completion_with_token() -> Result<()> {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Mock response for completion
        let mock_response = create_mock_open_ai_response("my-databricks-model", "Hello!");

        let _moderator_mock_response = json!({
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            },
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "This is a safe response"
                },
                "finish_reason": "stop"
            }]
        });

        // Expected request body
        let system = "You are a helpful assistant.";
        let expected_request_body = json!({
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": "Hello"}
            ]
        });
        // Set up the mock to intercept the request and respond with the mocked response
        Mock::given(method("POST"))
            .and(path("/serving-endpoints/my-databricks-model/invocations"))
            .and(header("Authorization", "Bearer test_token"))
            .and(body_json(expected_request_body.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .expect(1) // Expect exactly one matching request
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);

        // Prepare input
        let messages = vec![Message::user().with_text("Hello")];
        let tools = vec![]; // Empty tools list

        // Call the complete method
        let (reply_message, reply_usage) = provider.complete(system, &messages, &tools).await?;

        // Assert the response
        if let MessageContent::Text(text) = &reply_message.content[0] {
            assert_eq!(text.text, "Hello!");
        } else {
            panic!("Expected Text content");
        }
        assert_eq!(reply_usage.usage.input_tokens, Some(TEST_INPUT_TOKENS));
        assert_eq!(reply_usage.usage.output_tokens, Some(TEST_OUTPUT_TOKENS));
        assert_eq!(reply_usage.usage.total_tokens, Some(TEST_TOTAL_TOKENS));

        Ok(())
    }
}
