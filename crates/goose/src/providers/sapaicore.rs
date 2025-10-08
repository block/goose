use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ModelInfo, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{emit_debug_trace, get_model, ImageFormat};
use crate::conversation::message::Message;
use crate::impl_provider_default;
use crate::model::ModelConfig;
use rmcp::model::Tool;

pub const SAP_AI_CORE_DEFAULT_MODEL: &str = "gpt-4o";
pub const SAP_AI_CORE_DEFAULT_FAST_MODEL: &str = "gpt-4o-mini";

// Known models that can be deployed on SAP AI Core
pub const SAP_AI_CORE_KNOWN_MODELS: &[(&str, usize)] = &[
    // OpenAI models
    ("gpt-4o", 128_000),
    ("gpt-4o-mini", 128_000),
    ("gpt-4-turbo", 128_000),
    ("gpt-3.5-turbo", 16_385),
    // Anthropic models
    ("claude-3-5-sonnet-20241022", 200_000),
    ("claude-3-5-haiku-20241022", 200_000),
    ("claude-3-opus-20240229", 200_000),
    ("claude-3-sonnet-20240229", 200_000),
    ("claude-3-haiku-20240307", 200_000),
    // Google models
    ("gemini-1.5-pro", 2_000_000),
    ("gemini-1.5-flash", 1_000_000),
    ("gemini-1.0-pro", 32_000),
];

pub const SAP_AI_CORE_DOC_URL: &str = "https://help.sap.com/docs/sap-ai-core";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Debug, Clone)]
struct CachedToken {
    token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Deployment {
    id: String,
    configuration_id: String,
    scenario_id: String,
    status: String,
    deployment_url: Option<String>,
    target_status: String,
    created_at: String,
    modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeploymentList {
    count: u32,
    resources: Vec<Deployment>,
}

#[derive(Debug, Clone)]
enum ApiType {
    OpenAI,
    Anthropic,
    Gemini,
}

#[derive(Debug, Clone)]
struct ModelDeployment {
    deployment_id: String,
    deployment_url: String,
    api_type: ApiType,
    model_name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SapAiCoreProvider {
    #[serde(skip)]
    api_client: ApiClient,
    base_url: String,
    client_id: String,
    client_secret: String,
    resource_group: String,
    model: ModelConfig,
    #[serde(skip)]
    token_cache: Arc<RwLock<Option<CachedToken>>>,
    #[serde(skip)]
    deployment_cache: Arc<RwLock<HashMap<String, ModelDeployment>>>,
}

impl_provider_default!(SapAiCoreProvider);

impl SapAiCoreProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let model = model.with_fast(SAP_AI_CORE_DEFAULT_FAST_MODEL.to_string());

        let config = crate::config::Config::global();
        let base_url: String = config.get_param("SAP_AI_CORE_BASE_URL")?;
        let client_id: String = config.get_secret("SAP_AI_CORE_CLIENT_ID")?;
        let client_secret: String = config.get_secret("SAP_AI_CORE_CLIENT_SECRET")?;
        let resource_group: String = config
            .get_param("SAP_AI_CORE_RESOURCE_GROUP")
            .unwrap_or_else(|_| "default".to_string());

        // Use a dummy auth method initially - we'll handle OAuth separately
        let auth = AuthMethod::BearerToken("dummy".to_string());
        let api_client = ApiClient::new(base_url.clone(), auth)?;

        Ok(Self {
            api_client,
            base_url,
            client_id,
            client_secret,
            resource_group,
            model,
            token_cache: Arc::new(RwLock::new(None)),
            deployment_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn get_access_token(&self) -> Result<String, ProviderError> {
        // Check if we have a valid cached token
        {
            let cache = self.token_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                if cached.expires_at > Utc::now() + chrono::Duration::minutes(5) {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Get new token using OAuth 2.0 client credentials flow
        let token_url = format!("{}/oauth/token", self.base_url);
        let client = reqwest::Client::new();
        
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let response = client
            .post(&token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Token request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Authentication(format!(
                "Failed to get access token: {}",
                error_text
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse token response: {}", e)))?;

        let expires_at = Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64);
        let cached_token = CachedToken {
            token: token_response.access_token.clone(),
            expires_at,
        };

        // Cache the token
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(cached_token);
        }

        Ok(token_response.access_token)
    }

    async fn discover_deployments(&self) -> Result<(), ProviderError> {
        let token = self.get_access_token().await?;
        let deployments_url = format!(
            "{}/v2/lm/deployments?resourceGroupId={}",
            self.base_url, self.resource_group
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&deployments_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("AI-Resource-Group", &self.resource_group)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Deployment discovery failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(format!(
                "Failed to discover deployments: {}",
                error_text
            )));
        }

        let deployment_list: DeploymentList = response
            .json()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse deployments: {}", e)))?;

        let mut cache = self.deployment_cache.write().await;
        cache.clear();

        for deployment in deployment_list.resources {
            if deployment.status == "RUNNING" && deployment.deployment_url.is_some() {
                let deployment_url = deployment.deployment_url.unwrap();
                
                // Determine API type and model name from configuration_id
                let (api_type, model_name) = self.parse_deployment_info(&deployment.configuration_id);
                
                let model_deployment = ModelDeployment {
                    deployment_id: deployment.id,
                    deployment_url,
                    api_type,
                    model_name: model_name.clone(),
                };

                cache.insert(model_name, model_deployment);
            }
        }

        Ok(())
    }

    fn parse_deployment_info(&self, configuration_id: &str) -> (ApiType, String) {
        // Parse configuration ID to determine API type and model name
        // This is a simplified implementation - in practice, you might need more sophisticated parsing
        if configuration_id.contains("openai") || configuration_id.contains("gpt") {
            let model_name = if configuration_id.contains("gpt-4o-mini") {
                "gpt-4o-mini"
            } else if configuration_id.contains("gpt-4o") {
                "gpt-4o"
            } else if configuration_id.contains("gpt-4-turbo") {
                "gpt-4-turbo"
            } else if configuration_id.contains("gpt-3.5-turbo") {
                "gpt-3.5-turbo"
            } else {
                "gpt-4o" // default
            };
            (ApiType::OpenAI, model_name.to_string())
        } else if configuration_id.contains("claude") || configuration_id.contains("anthropic") {
            let model_name = if configuration_id.contains("claude-3-5-sonnet") {
                "claude-3-5-sonnet-20241022"
            } else if configuration_id.contains("claude-3-5-haiku") {
                "claude-3-5-haiku-20241022"
            } else if configuration_id.contains("claude-3-opus") {
                "claude-3-opus-20240229"
            } else if configuration_id.contains("claude-3-sonnet") {
                "claude-3-sonnet-20240229"
            } else if configuration_id.contains("claude-3-haiku") {
                "claude-3-haiku-20240307"
            } else {
                "claude-3-5-sonnet-20241022" // default
            };
            (ApiType::Anthropic, model_name.to_string())
        } else if configuration_id.contains("gemini") || configuration_id.contains("google") {
            let model_name = if configuration_id.contains("gemini-1.5-pro") {
                "gemini-1.5-pro"
            } else if configuration_id.contains("gemini-1.5-flash") {
                "gemini-1.5-flash"
            } else if configuration_id.contains("gemini-1.0-pro") {
                "gemini-1.0-pro"
            } else {
                "gemini-1.5-pro" // default
            };
            (ApiType::Gemini, model_name.to_string())
        } else {
            // Default to OpenAI API
            (ApiType::OpenAI, "gpt-4o".to_string())
        }
    }

    async fn get_deployment_for_model(&self, model_name: &str) -> Result<ModelDeployment, ProviderError> {
        // Check cache first
        {
            let cache = self.deployment_cache.read().await;
            if let Some(deployment) = cache.get(model_name) {
                return Ok(deployment.clone());
            }
        }

        // If not in cache, discover deployments
        self.discover_deployments().await?;

        // Check cache again
        let cache = self.deployment_cache.read().await;
        cache.get(model_name)
            .cloned()
            .ok_or_else(|| ProviderError::RequestFailed(format!("No deployment found for model: {}", model_name)))
    }

    async fn call_openai_api(
        &self,
        deployment: &ModelDeployment,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let token = self.get_access_token().await?;
        let payload = create_request(model_config, system, messages, tools, &ImageFormat::OpenAi)?;

        let client = reqwest::Client::new();
        let response = client
            .post(&deployment.deployment_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("AI-Resource-Group", &self.resource_group)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("OpenAI API call failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let json_response: Value = response
            .json()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse OpenAI response: {}", e)))?;

        let message = response_to_message(&json_response)?;
        let usage = json_response
            .get("usage")
            .map(get_usage)
            .unwrap_or_else(|| {
                tracing::debug!("Failed to get usage data from OpenAI API");
                Usage::default()
            });
        let model = get_model(&json_response);
        emit_debug_trace(&self.model, &payload, &json_response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }

    async fn call_anthropic_api(
        &self,
        deployment: &ModelDeployment,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let token = self.get_access_token().await?;
        
        // Convert to Anthropic format
        let anthropic_payload = self.create_anthropic_request(model_config, system, messages, tools)?;

        let client = reqwest::Client::new();
        let response = client
            .post(&deployment.deployment_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("AI-Resource-Group", &self.resource_group)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_payload)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Anthropic API call failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(format!(
                "Anthropic API error: {}",
                error_text
            )));
        }

        let json_response: Value = response
            .json()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse Anthropic response: {}", e)))?;

        // Convert Anthropic response to standard format
        let message = self.anthropic_response_to_message(&json_response)?;
        let usage = self.get_anthropic_usage(&json_response);
        emit_debug_trace(&self.model, &anthropic_payload, &json_response, &usage);
        Ok((message, ProviderUsage::new(deployment.model_name.clone(), usage)))
    }

    async fn call_gemini_api(
        &self,
        deployment: &ModelDeployment,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let token = self.get_access_token().await?;
        
        // Convert to Gemini format
        let gemini_payload = self.create_gemini_request(model_config, system, messages, tools)?;

        let client = reqwest::Client::new();
        let response = client
            .post(&deployment.deployment_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("AI-Resource-Group", &self.resource_group)
            .header("Content-Type", "application/json")
            .json(&gemini_payload)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Gemini API call failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(format!(
                "Gemini API error: {}",
                error_text
            )));
        }

        let json_response: Value = response
            .json()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse Gemini response: {}", e)))?;

        // Convert Gemini response to standard format
        let message = self.gemini_response_to_message(&json_response)?;
        let usage = self.get_gemini_usage(&json_response);
        emit_debug_trace(&self.model, &gemini_payload, &json_response, &usage);
        Ok((message, ProviderUsage::new(deployment.model_name.clone(), usage)))
    }

    fn create_anthropic_request(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<Value, ProviderError> {
        // This is a simplified implementation - you would need to properly convert
        // the OpenAI format to Anthropic's format
        let mut anthropic_messages = Vec::new();
        
        for message in messages {
            let role = match message.role {
                rmcp::model::Role::User => "user",
                rmcp::model::Role::Assistant => "assistant",
            };
            
            anthropic_messages.push(json!({
                "role": role,
                "content": message.as_concat_text()
            }));
        }

        let mut payload = json!({
            "model": model_config.model_name,
            "messages": anthropic_messages,
            "max_tokens": model_config.max_tokens.unwrap_or(4096),
        });

        if !system.is_empty() {
            payload["system"] = json!(system);
        }

        if let Some(temp) = model_config.temperature {
            payload["temperature"] = json!(temp);
        }

        if !tools.is_empty() {
            // Convert tools to Anthropic format
            let anthropic_tools: Vec<Value> = tools.iter().map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.input_schema
                })
            }).collect();
            payload["tools"] = json!(anthropic_tools);
        }

        Ok(payload)
    }

    fn create_gemini_request(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<Value, ProviderError> {
        // This is a simplified implementation - you would need to properly convert
        // the OpenAI format to Gemini's format
        let mut gemini_contents = Vec::new();
        
        if !system.is_empty() {
            gemini_contents.push(json!({
                "role": "user",
                "parts": [{"text": system}]
            }));
        }

        for message in messages {
            let role = match message.role {
                rmcp::model::Role::User => "user",
                rmcp::model::Role::Assistant => "model",
            };
            
            gemini_contents.push(json!({
                "role": role,
                "parts": [{"text": message.as_concat_text()}]
            }));
        }

        let mut payload = json!({
            "contents": gemini_contents,
            "generationConfig": {
                "maxOutputTokens": model_config.max_tokens.unwrap_or(4096),
            }
        });

        if let Some(temp) = model_config.temperature {
            payload["generationConfig"]["temperature"] = json!(temp);
        }

        if !tools.is_empty() {
            // Convert tools to Gemini format
            let gemini_tools: Vec<Value> = tools.iter().map(|tool| {
                json!({
                    "functionDeclarations": [{
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.input_schema
                    }]
                })
            }).collect();
            payload["tools"] = json!(gemini_tools);
        }

        Ok(payload)
    }

    fn anthropic_response_to_message(&self, response: &Value) -> Result<Message, ProviderError> {
        // Convert Anthropic response to standard Message format
        // This is a simplified implementation
        let content = response
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        Ok(Message::assistant().with_text(content))
    }

    fn gemini_response_to_message(&self, response: &Value) -> Result<Message, ProviderError> {
        // Convert Gemini response to standard Message format
        // This is a simplified implementation
        let content = response
            .get("candidates")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|candidate| candidate.get("content"))
            .and_then(|content| content.get("parts"))
            .and_then(|parts| parts.as_array())
            .and_then(|arr| arr.first())
            .and_then(|part| part.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        Ok(Message::assistant().with_text(content))
    }

    fn get_anthropic_usage(&self, response: &Value) -> Usage {
        let input_tokens = response
            .get("usage")
            .and_then(|u| u.get("input_tokens"))
            .and_then(|t| t.as_i64())
            .map(|t| t as i32);

        let output_tokens = response
            .get("usage")
            .and_then(|u| u.get("output_tokens"))
            .and_then(|t| t.as_i64())
            .map(|t| t as i32);

        let total_tokens = match (input_tokens, output_tokens) {
            (Some(input), Some(output)) => Some(input + output),
            _ => None,
        };

        Usage::new(input_tokens, output_tokens, total_tokens)
    }

    fn get_gemini_usage(&self, response: &Value) -> Usage {
        let input_tokens = response
            .get("usageMetadata")
            .and_then(|u| u.get("promptTokenCount"))
            .and_then(|t| t.as_i64())
            .map(|t| t as i32);

        let output_tokens = response
            .get("usageMetadata")
            .and_then(|u| u.get("candidatesTokenCount"))
            .and_then(|t| t.as_i64())
            .map(|t| t as i32);

        let total_tokens = response
            .get("usageMetadata")
            .and_then(|u| u.get("totalTokenCount"))
            .and_then(|t| t.as_i64())
            .map(|t| t as i32);

        Usage::new(input_tokens, output_tokens, total_tokens)
    }
}

#[async_trait]
impl Provider for SapAiCoreProvider {
    fn metadata() -> ProviderMetadata {
        let models = SAP_AI_CORE_KNOWN_MODELS
            .iter()
            .map(|(name, limit)| ModelInfo::new(*name, *limit))
            .collect();
        ProviderMetadata::with_models(
            "sapaicore",
            "SAP AI Core",
            "Access multiple AI models (OpenAI, Anthropic, Google) through SAP AI Core with OAuth 2.0 authentication",
            SAP_AI_CORE_DEFAULT_MODEL,
            models,
            SAP_AI_CORE_DOC_URL,
            vec![
                ConfigKey::new("SAP_AI_CORE_BASE_URL", true, false, None),
                ConfigKey::new("SAP_AI_CORE_CLIENT_ID", true, true, None),
                ConfigKey::new("SAP_AI_CORE_CLIENT_SECRET", true, true, None),
                ConfigKey::new("SAP_AI_CORE_RESOURCE_GROUP", false, false, Some("default")),
            ],
        )
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
        let deployment = self.get_deployment_for_model(&model_config.model_name).await?;

        match deployment.api_type {
            ApiType::OpenAI => {
                self.call_openai_api(&deployment, model_config, system, messages, tools).await
            }
            ApiType::Anthropic => {
                self.call_anthropic_api(&deployment, model_config, system, messages, tools).await
            }
            ApiType::Gemini => {
                self.call_gemini_api(&deployment, model_config, system, messages, tools).await
            }
        }
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // Discover deployments to get available models
        self.discover_deployments().await?;
        
        let cache = self.deployment_cache.read().await;
        let models: Vec<String> = cache.keys().cloned().collect();
        Ok(Some(models))
    }

    fn supports_streaming(&self) -> bool {
        // Streaming support would depend on the specific deployment
        // For now, return false as a conservative default
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelConfig;

    #[test]
    fn test_provider_metadata() {
        let metadata = SapAiCoreProvider::metadata();
        assert_eq!(metadata.name, "sapaicore");
        assert_eq!(metadata.display_name, "SAP AI Core");
        assert_eq!(metadata.default_model, SAP_AI_CORE_DEFAULT_MODEL);
        assert!(!metadata.known_models.is_empty());
        assert_eq!(metadata.config_keys.len(), 4);
        
        // Check that required config keys are present
        let config_key_names: Vec<&str> = metadata.config_keys.iter().map(|k| k.name.as_str()).collect();
        assert!(config_key_names.contains(&"SAP_AI_CORE_BASE_URL"));
        assert!(config_key_names.contains(&"SAP_AI_CORE_CLIENT_ID"));
        assert!(config_key_names.contains(&"SAP_AI_CORE_CLIENT_SECRET"));
        assert!(config_key_names.contains(&"SAP_AI_CORE_RESOURCE_GROUP"));
    }

    #[test]
    fn test_parse_deployment_info() {
        let model = ModelConfig::new_or_fail("gpt-4o");
        let provider = SapAiCoreProvider {
            api_client: ApiClient::new("https://test.com".to_string(), AuthMethod::BearerToken("test".to_string())).unwrap(),
            base_url: "https://test.com".to_string(),
            client_id: "test".to_string(),
            client_secret: "test".to_string(),
            resource_group: "default".to_string(),
            model,
            token_cache: Arc::new(RwLock::new(None)),
            deployment_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Test OpenAI model parsing
        let (api_type, model_name) = provider.parse_deployment_info("openai-gpt-4o-config");
        assert!(matches!(api_type, ApiType::OpenAI));
        assert_eq!(model_name, "gpt-4o");

        // Test Anthropic model parsing
        let (api_type, model_name) = provider.parse_deployment_info("anthropic-claude-3-5-sonnet-config");
        assert!(matches!(api_type, ApiType::Anthropic));
        assert_eq!(model_name, "claude-3-5-sonnet-20241022");

        // Test Gemini model parsing
        let (api_type, model_name) = provider.parse_deployment_info("google-gemini-1.5-pro-config");
        assert!(matches!(api_type, ApiType::Gemini));
        assert_eq!(model_name, "gemini-1.5-pro");

        // Test default case
        let (api_type, model_name) = provider.parse_deployment_info("unknown-config");
        assert!(matches!(api_type, ApiType::OpenAI));
        assert_eq!(model_name, "gpt-4o");
    }

    #[test]
    fn test_known_models() {
        // Test that all known models are included
        assert!(SAP_AI_CORE_KNOWN_MODELS.len() > 0);
        
        // Test specific models
        let model_names: Vec<&str> = SAP_AI_CORE_KNOWN_MODELS.iter().map(|(name, _)| *name).collect();
        assert!(model_names.contains(&"gpt-4o"));
        assert!(model_names.contains(&"claude-3-5-sonnet-20241022"));
        assert!(model_names.contains(&"gemini-1.5-pro"));
    }

    #[test]
    fn test_constants() {
        assert_eq!(SAP_AI_CORE_DEFAULT_MODEL, "gpt-4o");
        assert_eq!(SAP_AI_CORE_DEFAULT_FAST_MODEL, "gpt-4o-mini");
        assert!(!SAP_AI_CORE_DOC_URL.is_empty());
    }
}
