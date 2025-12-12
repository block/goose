use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use super::api_client::{ApiClient, AuthMethod, AuthProvider};
use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage};
use super::embedding::EmbeddingCapable;
use super::errors::ProviderError;
use super::formats::sap_ai_core::{create_request, get_usage, response_to_message};
use super::pricing::PricingInfo;

use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

const DEFAULT_TIMEOUT_SECS: u64 = 600;

// Default models for SAP AI Core (you can customize these)
pub const SAP_AI_CORE_NAME: &str = "sap_ai_core";
pub const SAP_AI_CORE_DEFAULT_MODEL: &str = "anthropic--claude-4-sonnet";
pub const SAP_AI_CORE_DEFAULT_FAST_MODEL: &str = "gpt-5-mini";
pub const SAP_AI_CORE_KNOWN_MODELS: &[&str] = &["anthropic--claude-4-sonnet"];

pub const SAP_AI_CORE_DOC_URL: &str = "https://help.sap.com/docs/ai-core";

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum CostItem {
    Input {
        #[serde(rename = "inputCost")]
        input_cost: String,
    },
    Output {
        #[serde(rename = "outputCost")]
        output_cost: String,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelScenario {
    pub executable_id: String,
    pub scenario_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersion {
    pub capabilities: Option<Vec<String>>,
    pub context_length: Option<usize>,
    pub cost: Option<Vec<CostItem>>,
    pub input_types: Option<Vec<String>>,
    pub is_latest: bool,
    pub streaming_supported: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelResource {
    pub access_type: String,
    pub allowed_scenarios: Vec<ModelScenario>,
    pub description: String,
    pub display_name: String,
    pub model: String,
    pub versions: Vec<ModelVersion>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModelResponse {
    pub count: usize,
    pub resources: Vec<ModelResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SAPAICoreAuth {
    pub oauth_token_url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientCredentialsTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    scope: Option<String>,
    jti: Option<String>, // JWT ID
}

// Token management structs
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

#[derive(Clone, Debug)]
struct AccessToken {
    token: String,
    expires_at: SystemTime,
}

impl AccessToken {
    fn new(token: String, expires_in_seconds: u64) -> Self {
        Self {
            token,
            expires_at: SystemTime::now() + Duration::from_secs(expires_in_seconds),
        }
    }

    fn is_expired(&self) -> bool {
        SystemTime::now() >= self.expires_at
    }
}

#[derive(Debug)]
struct TokenManager {
    token: Arc<RwLock<Option<AccessToken>>>,
}

impl TokenManager {
    fn new() -> Self {
        Self {
            token: Arc::new(RwLock::new(None)),
        }
    }

    fn set_token(&self, token: String, expires_in_seconds: u64) {
        let access_token = AccessToken::new(token, expires_in_seconds);
        let mut current_token = self.token.write().unwrap();
        *current_token = Some(access_token);
    }

    fn is_expired(&self) -> bool {
        let token = self.token.read().unwrap();
        match &*token {
            None => true, // No token means expired
            Some(access_token) => access_token.is_expired(),
        }
    }

    fn get_token(&self) -> Option<String> {
        let token = self.token.read().unwrap();
        match &*token {
            None => None,
            Some(access_token) => {
                if access_token.is_expired() {
                    None
                } else {
                    Some(access_token.token.clone())
                }
            }
        }
    }
}

impl Clone for TokenManager {
    fn clone(&self) -> Self {
        Self {
            token: Arc::clone(&self.token),
        }
    }
}

struct SAPAICoreAuthProvider {
    auth: SAPAICoreAuth,
    token_manager: TokenManager,
}

#[async_trait]
impl AuthProvider for SAPAICoreAuthProvider {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        let token = self.get_access_token().await?;
        Ok(("Authorization".to_string(), format!("Bearer {}", token)))
    }
}

impl SAPAICoreAuthProvider {
    async fn get_access_token(&self) -> Result<String> {
        if !self.token_manager.is_expired() {
            return Ok(self.token_manager.get_token().unwrap());
        }

        let client = reqwest::Client::new();

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.auth.client_id),
            ("client_secret", &self.auth.client_secret),
        ];

        // Construct the token URL by appending /oauth/token to the base URL
        let token_url = if self.auth.oauth_token_url.ends_with("/oauth/token") {
            self.auth.oauth_token_url.clone()
        } else {
            format!(
                "{}/oauth/token",
                self.auth.oauth_token_url.trim_end_matches('/')
            )
        };

        let response = client
            .post(&token_url)
            .header("content-type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "OAuth2 token request failed: {} - {}",
                status,
                error_text
            ));
        }

        let token_response: ClientCredentialsTokenResponse = response.json().await?;
        self.token_manager
            .set_token(token_response.access_token, token_response.expires_in);
        Ok(self.token_manager.get_token().unwrap())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SAPAICoreProvider {
    name: String,
    #[serde(skip)]
    api_client: ApiClient,
    auth: SAPAICoreAuth,
    orchestration_url: String,
    models_url: String,
    resource_group: String,
    model: ModelConfig,
    models: Mutex<HashMap<String, ModelResource>>,
}

impl SAPAICoreProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        // Get required configuration parameters
        let oauth_token_url: String = config.get_param("SAP_AI_CORE_OAUTH_TOKEN_DOMAIN")?;
        let client_id: String = config.get_param("SAP_AI_CORE_OAUTH_CLIENT_ID")?;
        let client_secret: String = config.get_secret("SAP_AI_CORE_OAUTH_CLIENT_SECRET")?;
        let ai_domain: String = config.get_param("SAP_AI_CORE_API_DOMAIN")?;
        let orchestration_path: String = config.get_param("SAP_AI_CORE_ORCHESTRATION_PATH")?;
        let models_path: String = config.get_param("SAP_AI_CORE_MODELS_PATH")?;
        let resource_group: String = config.get_param("SAP_AI_CORE_RESOURCE_GROUP")?;

        let auth = SAPAICoreAuth {
            oauth_token_url,
            client_id,
            client_secret,
        };

        let auth_method = AuthMethod::Custom(Box::new(SAPAICoreAuthProvider {
            auth: auth.clone(),
            token_manager: TokenManager::new(),
        }));

        let api_client = ApiClient::with_timeout(
            ai_domain.clone(),
            auth_method,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        )?;

        let model = model.with_fast(SAP_AI_CORE_DEFAULT_FAST_MODEL.to_string());
        let models = Mutex::new(HashMap::new());

        let orchestration_url = format!("{}{}", ai_domain, orchestration_path);
        let models_url = format!("{}{}", ai_domain, models_path);

        let provider = Self {
            name: Self::metadata().name,
            api_client,
            auth,
            orchestration_url,
            models_url,
            resource_group,
            model,
            models,
        };

        // Load models synchronously using tokio::task::block_in_place
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(provider.fetch_supported_models())
        })?;

        Ok(provider)
    }

    async fn post(&self, payload: &serde_json::Value) -> Result<serde_json::Value, ProviderError> {
        // SAP AI Core orchestration endpoint path
        let path = format!("{}/v2/completion", self.orchestration_url);

        let response = self
            .api_client
            .request(&path)
            .header("ai-resource-group", &self.resource_group)?
            .response_post(payload)
            .await?;

        // Handle response similar to other providers
        if response.status().is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;
            serde_json::from_str(&body).map_err(|e| ProviderError::RequestFailed(e.to_string()))
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ProviderError::RequestFailed(format!(
                "SAP AI Core request failed: {} - {}",
                status, error_text
            )))
        }
    }

    async fn fetch_models(&self) -> Result<HashMap<String, ModelResource>, ProviderError> {
        let response = match self
            .api_client
            .request(&self.models_url)
            .header("ai-resource-group", &self.resource_group)?
            .response_get()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("Failed to fetch models from SAP API: {}", e);
                return Ok(HashMap::new());
            }
        };

        if !response.status().is_success() {
            tracing::error!(
                "Failed to fetch models from SAP API: {}",
                response.text().await?
            );
            return Ok(HashMap::new());
        }

        let text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                tracing::error!("Failed to get response text: {}", e);
                return Ok(HashMap::new());
            }
        };

        let model_response: ModelResponse = match serde_json::from_str(&text) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to fetch models from SAP API: {}", e);
                tracing::error!("Error at line {}, column {}", e.line(), e.column());
                return Ok(HashMap::new());
            }
        };

        if model_response.count == 0 {
            return Ok(HashMap::new());
        }

        let _models: HashMap<String, ModelResource> = model_response
            .resources
            .into_iter()
            .filter(|res| res.versions.iter().any(|v| v.is_latest))
            .filter(|res| {
                res.allowed_scenarios
                    .iter()
                    .any(|sc| sc.scenario_id == "orchestration")
            })
            .map(|mut res| {
                res.versions.retain(|v| v.is_latest);
                (res.model.clone(), res)
            })
            .collect();
        Ok(_models)
    }
}

#[async_trait]
impl Provider for SAPAICoreProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            SAP_AI_CORE_NAME,
            "SAP AI Core",
            "Access LLM models via SAP AI Core orchestration",
            SAP_AI_CORE_DEFAULT_MODEL,
            SAP_AI_CORE_KNOWN_MODELS.to_vec(),
            SAP_AI_CORE_DOC_URL,
            vec![
                ConfigKey::new("SAP_AI_CORE_OAUTH_TOKEN_DOMAIN", true, false, None),
                ConfigKey::new("SAP_AI_CORE_OAUTH_CLIENT_ID", true, false, None),
                ConfigKey::new("SAP_AI_CORE_OAUTH_CLIENT_SECRET", true, true, None),
                ConfigKey::new("SAP_AI_CORE_API_DOMAIN", true, false, None),
                ConfigKey::new("SAP_AI_CORE_ORCHESTRATION_PATH", true, false, None),
                ConfigKey::new("SAP_AI_CORE_MODELS_PATH", true, false, None),
                ConfigKey::new("SAP_AI_CORE_RESOURCE_GROUP", true, false, None),
            ],
        )
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Create all messages including system message
        let mut all_messages = vec![];

        // Add conversation messages
        all_messages.extend_from_slice(messages);

        // Create SAP AI Core request using our format
        let request = create_request(&all_messages, tools, model_config, system, false)
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        // Send request to SAP AI Core
        let response = self.post(&request).await?;

        // Parse response using our format
        let response_str = serde_json::to_string(&response)
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let message = response_to_message(&response_str)
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let usage =
            get_usage(&response_str).map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        Ok((message, usage))
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        use async_stream::try_stream;
        use futures::TryStreamExt;
        use std::io;
        use tokio_util::io::StreamReader;

        let model_config = self.model.clone();

        let mut all_messages = vec![];
        all_messages.extend_from_slice(messages);

        let request = create_request(&all_messages, tools, &model_config, system, true)
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let path = format!("{}/v2/completion", self.orchestration_url);

        let response = self
            .api_client
            .request(&path)
            .header("ai-resource-group", &self.resource_group)?
            .header("Accept", "text/event-stream")?
            .header("Cache-Control", "no-cache")?
            .response_post(&request)
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(format!(
                "SAP AI Core streaming request failed: {} - {}",
                status, error_text
            )));
        }

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = tokio_util::codec::FramedRead::new(stream_reader, tokio_util::codec::LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = super::formats::sap_ai_core::response_to_streaming_message(framed);
            tokio::pin!(message_stream);
            while let Some(message) = futures::StreamExt::next(&mut message_stream).await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                yield (message, usage);
            }
        }))
    }

    fn supports_streaming(&self) -> bool {
        let model_config = self.model.clone();

        let models = self.models.lock().unwrap();
        if let Some(model_resource) = models.get(&model_config.model_name) {
            if let Some(first_version) = model_resource.versions.first() {
                return first_version.streaming_supported;
            }
        }
        tracing::info!("Streaming is not supported");
        false
    }

    fn supports_embeddings(&self) -> bool {
        let model_config = self.model.clone();

        let models = self.models.lock().unwrap();
        if let Some(model_resource) = models.get(&model_config.model_name) {
            if let Some(first_version) = model_resource.versions.first() {
                if let Some(capabilities) = &first_version.capabilities {
                    return capabilities.contains(&"embedding".to_string());
                }
            }
        }
        false
    }

    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        EmbeddingCapable::create_embeddings(self, texts)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        if !self.models.lock().unwrap().is_empty() {
            let mut _model_names: Vec<String> =
                self.models.lock().unwrap().keys().cloned().collect();
            _model_names.sort_by(|a, b| b.cmp(a));
            return Ok(Some(_model_names));
        }

        let fetched_models = self.fetch_models().await?;

        let mut _models = self.models.lock().unwrap();
        _models.clear();
        _models.extend(fetched_models);

        let mut _model_names: Vec<String> = _models.keys().cloned().collect();
        _model_names.sort_by(|a, b| b.cmp(a));
        Ok(Some(_model_names))
    }

    async fn get_pricing(&self) -> Option<HashMap<String, HashMap<String, PricingInfo>>> {
        if self.models.lock().unwrap().is_empty() {
            self.fetch_supported_models()
                .await
                .inspect_err(|_e| {
                    tracing::error!("Cannot fetch models for pricing");
                })
                .ok()?;
        }

        if self.models.lock().unwrap().is_empty() {
            return None;
        }

        // - Try to get pricing from model cache
        let sap_pricing: HashMap<String, PricingInfo> = self
            .models
            .lock()
            .unwrap()
            .values()
            .filter(|&model| model.versions.iter().any(|v| v.cost.is_some()))
            .cloned()
            .map(|model| {
                let name = model.model.to_string();
                let first = model.versions.first().unwrap();
                let first_input_cost = first
                    .cost
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find(|&cost| matches!(cost, CostItem::Input { .. }))
                    .map(|cost_ref| match cost_ref {
                        CostItem::Input { input_cost } => input_cost.clone(),
                        _ => String::new(),
                    })
                    .and_then(|cost_str| cost_str.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let first_output_cost = first
                    .cost
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find(|&cost| matches!(cost, CostItem::Output { .. }))
                    .map(|cost_ref| match cost_ref {
                        CostItem::Output { output_cost } => output_cost.clone(),
                        _ => String::new(),
                    })
                    .and_then(|cost_str| cost_str.parse::<f64>().ok())
                    .unwrap_or(0.0);

                (
                    name,
                    PricingInfo {
                        input_cost: first_input_cost / 1000.0,
                        output_cost: first_output_cost / 1000.0,
                        context_length: first.context_length.map(|l| l as u32),
                    },
                )
            })
            .collect();

        if sap_pricing.is_empty() {
            return None;
        }

        let mut result: HashMap<String, HashMap<String, PricingInfo>> = HashMap::new();
        result.insert(SAP_AI_CORE_NAME.to_string(), sap_pricing);

        Some(result)
    }
}

#[async_trait]
impl EmbeddingCapable for SAPAICoreProvider {
    async fn create_embeddings(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let metadata = SAPAICoreProvider::metadata();
        assert_eq!(metadata.name, "sap_ai_core");
        assert_eq!(metadata.display_name, "SAP AI Core");
        assert_eq!(metadata.config_keys.len(), 7);

        // Check that all config keys are present
        let key_names: Vec<&String> = metadata.config_keys.iter().map(|k| &k.name).collect();
        assert!(key_names.contains(&&"SAP_AI_CORE_OAUTH_TOKEN_DOMAIN".to_string()));
        assert!(key_names.contains(&&"SAP_AI_CORE_OAUTH_CLIENT_ID".to_string()));
        assert!(key_names.contains(&&"SAP_AI_CORE_OAUTH_CLIENT_SECRET".to_string()));
        assert!(key_names.contains(&&"SAP_AI_CORE_API_DOMAIN".to_string()));
        assert!(key_names.contains(&&"SAP_AI_CORE_ORCHESTRATION_PATH".to_string()));
        assert!(key_names.contains(&&"SAP_AI_CORE_MODELS_PATH".to_string()));
        assert!(key_names.contains(&&"SAP_AI_CORE_RESOURCE_GROUP".to_string()));
    }

    #[test]
    fn test_cost_item_input_serialization() {
        let cost = CostItem::Input {
            input_cost: "0.015".to_string(),
        };
        let json = serde_json::to_string(&cost).unwrap();
        assert!(json.contains("inputCost"));
        assert!(json.contains("0.015"));
    }

    #[test]
    fn test_cost_item_output_serialization() {
        let cost = CostItem::Output {
            output_cost: "0.075".to_string(),
        };
        let json = serde_json::to_string(&cost).unwrap();
        assert!(json.contains("outputCost"));
        assert!(json.contains("0.075"));
    }

    #[test]
    fn test_cost_item_deserialization() {
        let input_json = r#"{"inputCost":"0.015"}"#;
        let cost: CostItem = serde_json::from_str(input_json).unwrap();
        match cost {
            CostItem::Input { input_cost } => assert_eq!(input_cost, "0.015"),
            _ => panic!("Expected Input variant"),
        }

        let output_json = r#"{"outputCost":"0.075"}"#;
        let cost: CostItem = serde_json::from_str(output_json).unwrap();
        match cost {
            CostItem::Output { output_cost } => assert_eq!(output_cost, "0.075"),
            _ => panic!("Expected Output variant"),
        }
    }

    #[test]
    fn test_model_scenario_serialization() {
        let scenario = ModelScenario {
            executable_id: "exec-123".to_string(),
            scenario_id: "orchestration".to_string(),
        };
        let json = serde_json::to_string(&scenario).unwrap();
        assert!(json.contains("executableId"));
        assert!(json.contains("scenarioId"));
    }

    #[test]
    fn test_model_version_serialization() {
        let version = ModelVersion {
            capabilities: Some(vec!["chat".to_string(), "embedding".to_string()]),
            context_length: Some(128000),
            cost: Some(vec![
                CostItem::Input {
                    input_cost: "0.015".to_string(),
                },
                CostItem::Output {
                    output_cost: "0.075".to_string(),
                },
            ]),
            input_types: Some(vec!["text".to_string()]),
            is_latest: true,
            streaming_supported: true,
        };
        let json = serde_json::to_string(&version).unwrap();
        assert!(json.contains("contextLength"));
        assert!(json.contains("isLatest"));
        assert!(json.contains("streamingSupported"));
    }

    #[test]
    fn test_model_version_minimal() {
        let version = ModelVersion {
            capabilities: None,
            context_length: None,
            cost: None,
            input_types: None,
            is_latest: false,
            streaming_supported: false,
        };
        assert!(!version.is_latest);
        assert!(!version.streaming_supported);
    }

    #[test]
    fn test_model_resource_deserialization() {
        let json = r#"{
            "accessType": "public",
            "allowedScenarios": [{
                "executableId": "exec-1",
                "scenarioId": "orchestration"
            }],
            "description": "Test model",
            "displayName": "Test Model",
            "model": "test-model",
            "versions": [{
                "isLatest": true,
                "streamingSupported": true
            }]
        }"#;
        let resource: ModelResource = serde_json::from_str(json).unwrap();
        assert_eq!(resource.model, "test-model");
        assert_eq!(resource.display_name, "Test Model");
        assert_eq!(resource.versions.len(), 1);
    }

    #[test]
    fn test_model_response_deserialization() {
        let json = r#"{
            "count": 2,
            "resources": [{
                "accessType": "public",
                "allowedScenarios": [],
                "description": "Model 1",
                "displayName": "Model 1",
                "model": "model-1",
                "versions": []
            }, {
                "accessType": "public",
                "allowedScenarios": [],
                "description": "Model 2",
                "displayName": "Model 2",
                "model": "model-2",
                "versions": []
            }]
        }"#;
        let response: ModelResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.count, 2);
        assert_eq!(response.resources.len(), 2);
    }

    #[test]
    fn test_sap_ai_core_auth_serialization() {
        let auth = SAPAICoreAuth {
            oauth_token_url: "https://auth.example.com".to_string(),
            client_id: "client123".to_string(),
            client_secret: "secret456".to_string(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains("oauth_token_url"));
        assert!(json.contains("client_id"));
        assert!(json.contains("client_secret"));
    }

    #[test]
    fn test_sap_ai_core_auth_clone() {
        let auth = SAPAICoreAuth {
            oauth_token_url: "https://auth.example.com".to_string(),
            client_id: "client123".to_string(),
            client_secret: "secret456".to_string(),
        };
        let cloned = auth.clone();
        assert_eq!(auth.oauth_token_url, cloned.oauth_token_url);
        assert_eq!(auth.client_id, cloned.client_id);
        assert_eq!(auth.client_secret, cloned.client_secret);
    }

    #[test]
    fn test_client_credentials_token_response_deserialization() {
        let json = r#"{
            "access_token": "token123",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "read write",
            "jti": "jwt-id-123"
        }"#;
        let response: ClientCredentialsTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "token123");
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 3600);
        assert_eq!(response.scope, Some("read write".to_string()));
        assert_eq!(response.jti, Some("jwt-id-123".to_string()));
    }

    #[test]
    fn test_client_credentials_token_response_minimal() {
        let json = r#"{
            "access_token": "token123",
            "token_type": "Bearer",
            "expires_in": 3600
        }"#;
        let response: ClientCredentialsTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "token123");
        assert!(response.scope.is_none());
        assert!(response.jti.is_none());
    }

    #[test]
    fn test_constants() {
        assert_eq!(SAP_AI_CORE_NAME, "sap_ai_core");
        assert_eq!(SAP_AI_CORE_DEFAULT_MODEL, "anthropic--claude-4-sonnet");
        assert_eq!(SAP_AI_CORE_DEFAULT_FAST_MODEL, "gpt-5-mini");
        assert!(SAP_AI_CORE_KNOWN_MODELS.contains(&"anthropic--claude-4-sonnet"));
        assert_eq!(DEFAULT_TIMEOUT_SECS, 600);
    }

    #[test]
    fn test_model_scenario_clone() {
        let scenario = ModelScenario {
            executable_id: "exec-123".to_string(),
            scenario_id: "orchestration".to_string(),
        };
        let cloned = scenario.clone();
        assert_eq!(scenario.executable_id, cloned.executable_id);
        assert_eq!(scenario.scenario_id, cloned.scenario_id);
    }

    #[test]
    fn test_model_version_clone() {
        let version = ModelVersion {
            capabilities: Some(vec!["chat".to_string()]),
            context_length: Some(4096),
            cost: Some(vec![CostItem::Input {
                input_cost: "0.01".to_string(),
            }]),
            input_types: Some(vec!["text".to_string()]),
            is_latest: true,
            streaming_supported: true,
        };
        let cloned = version.clone();
        assert_eq!(version.is_latest, cloned.is_latest);
        assert_eq!(version.streaming_supported, cloned.streaming_supported);
        assert_eq!(version.context_length, cloned.context_length);
    }

    #[test]
    fn test_model_resource_clone() {
        let resource = ModelResource {
            access_type: "public".to_string(),
            allowed_scenarios: vec![ModelScenario {
                executable_id: "exec-1".to_string(),
                scenario_id: "orchestration".to_string(),
            }],
            description: "Test".to_string(),
            display_name: "Test Model".to_string(),
            model: "test-model".to_string(),
            versions: vec![],
        };
        let cloned = resource.clone();
        assert_eq!(resource.model, cloned.model);
        assert_eq!(resource.display_name, cloned.display_name);
    }

    #[test]
    fn test_cost_item_clone() {
        let input_cost = CostItem::Input {
            input_cost: "0.015".to_string(),
        };
        let cloned = input_cost.clone();
        match (input_cost, cloned) {
            (CostItem::Input { input_cost: a }, CostItem::Input { input_cost: b }) => {
                assert_eq!(a, b);
            }
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_model_version_with_all_fields() {
        let version = ModelVersion {
            capabilities: Some(vec![
                "chat".to_string(),
                "embedding".to_string(),
                "tool_use".to_string(),
            ]),
            context_length: Some(200000),
            cost: Some(vec![
                CostItem::Input {
                    input_cost: "0.003".to_string(),
                },
                CostItem::Output {
                    output_cost: "0.015".to_string(),
                },
            ]),
            input_types: Some(vec!["text".to_string(), "image".to_string()]),
            is_latest: true,
            streaming_supported: true,
        };

        assert!(version
            .capabilities
            .as_ref()
            .unwrap()
            .contains(&"chat".to_string()));
        assert!(version
            .capabilities
            .as_ref()
            .unwrap()
            .contains(&"embedding".to_string()));
        assert_eq!(version.context_length, Some(200000));
        assert_eq!(version.cost.as_ref().unwrap().len(), 2);
        assert!(version.is_latest);
        assert!(version.streaming_supported);
    }

    #[test]
    fn test_model_resource_with_multiple_scenarios() {
        let resource = ModelResource {
            access_type: "restricted".to_string(),
            allowed_scenarios: vec![
                ModelScenario {
                    executable_id: "exec-1".to_string(),
                    scenario_id: "orchestration".to_string(),
                },
                ModelScenario {
                    executable_id: "exec-2".to_string(),
                    scenario_id: "completion".to_string(),
                },
            ],
            description: "Multi-scenario model".to_string(),
            display_name: "Multi Model".to_string(),
            model: "multi-model".to_string(),
            versions: vec![],
        };

        assert_eq!(resource.allowed_scenarios.len(), 2);
        assert_eq!(resource.allowed_scenarios[0].scenario_id, "orchestration");
        assert_eq!(resource.allowed_scenarios[1].scenario_id, "completion");
    }

    #[test]
    fn test_model_resource_with_multiple_versions() {
        let resource = ModelResource {
            access_type: "public".to_string(),
            allowed_scenarios: vec![],
            description: "Versioned model".to_string(),
            display_name: "Versioned".to_string(),
            model: "versioned-model".to_string(),
            versions: vec![
                ModelVersion {
                    capabilities: None,
                    context_length: Some(4096),
                    cost: None,
                    input_types: None,
                    is_latest: false,
                    streaming_supported: false,
                },
                ModelVersion {
                    capabilities: None,
                    context_length: Some(8192),
                    cost: None,
                    input_types: None,
                    is_latest: true,
                    streaming_supported: true,
                },
            ],
        };

        assert_eq!(resource.versions.len(), 2);
        assert!(!resource.versions[0].is_latest);
        assert!(resource.versions[1].is_latest);
    }

    #[test]
    fn test_model_response_empty() {
        let response = ModelResponse {
            count: 0,
            resources: vec![],
        };
        assert_eq!(response.count, 0);
        assert!(response.resources.is_empty());
    }

    #[test]
    fn test_cost_item_debug() {
        let input = CostItem::Input {
            input_cost: "0.01".to_string(),
        };
        let debug_str = format!("{:?}", input);
        assert!(debug_str.contains("Input"));
        assert!(debug_str.contains("0.01"));
    }

    // Token Manager tests
    use std::thread;

    #[test]
    fn test_access_token_new() {
        let token = AccessToken::new("test_token".to_string(), 3600);
        assert_eq!(token.token, "test_token");
        assert!(!token.is_expired());
    }

    #[test]
    fn test_access_token_not_expired() {
        let token = AccessToken::new("test_token".to_string(), 3600);
        assert!(!token.is_expired());
    }

    #[test]
    fn test_access_token_expired() {
        let token = AccessToken::new("test_token".to_string(), 0);
        thread::sleep(Duration::from_millis(10));
        assert!(token.is_expired());
    }

    #[test]
    fn test_access_token_expires_at_boundary() {
        let token = AccessToken::new("test_token".to_string(), 0);
        assert!(token.is_expired());
    }

    #[test]
    fn test_token_manager_new() {
        let manager = TokenManager::new();
        assert!(manager.is_expired());
        assert_eq!(manager.get_token(), None);
    }

    #[test]
    fn test_token_manager_set_token() {
        let manager = TokenManager::new();
        manager.set_token("my_token".to_string(), 3600);
        assert!(!manager.is_expired());
        assert_eq!(manager.get_token(), Some("my_token".to_string()));
    }

    #[test]
    fn test_token_manager_is_expired_no_token() {
        let manager = TokenManager::new();
        assert!(manager.is_expired());
    }

    #[test]
    fn test_token_manager_is_expired_with_valid_token() {
        let manager = TokenManager::new();
        manager.set_token("valid_token".to_string(), 3600);
        assert!(!manager.is_expired());
    }

    #[test]
    fn test_token_manager_is_expired_with_expired_token() {
        let manager = TokenManager::new();
        manager.set_token("expired_token".to_string(), 0);
        thread::sleep(Duration::from_millis(10));
        assert!(manager.is_expired());
    }

    #[test]
    fn test_token_manager_get_token_none() {
        let manager = TokenManager::new();
        assert_eq!(manager.get_token(), None);
    }

    #[test]
    fn test_token_manager_get_token_valid() {
        let manager = TokenManager::new();
        manager.set_token("valid_token".to_string(), 3600);
        assert_eq!(manager.get_token(), Some("valid_token".to_string()));
    }

    #[test]
    fn test_token_manager_get_token_expired_returns_none() {
        let manager = TokenManager::new();
        manager.set_token("expired_token".to_string(), 0);
        thread::sleep(Duration::from_millis(10));
        assert_eq!(manager.get_token(), None);
    }

    #[test]
    fn test_token_manager_replace_token() {
        let manager = TokenManager::new();
        manager.set_token("token1".to_string(), 3600);
        assert_eq!(manager.get_token(), Some("token1".to_string()));

        manager.set_token("token2".to_string(), 3600);
        assert_eq!(manager.get_token(), Some("token2".to_string()));
    }

    #[test]
    fn test_token_manager_clone_shares_state() {
        let manager1 = TokenManager::new();
        manager1.set_token("shared_token".to_string(), 3600);

        let manager2 = manager1.clone();
        assert_eq!(manager2.get_token(), Some("shared_token".to_string()));

        manager2.set_token("new_token".to_string(), 3600);
        assert_eq!(manager1.get_token(), Some("new_token".to_string()));
    }

    #[test]
    fn test_token_manager_thread_safety() {
        let manager = TokenManager::new();
        manager.set_token("initial_token".to_string(), 3600);

        let manager_clone = manager.clone();
        let handle = thread::spawn(move || {
            manager_clone.set_token("thread_token".to_string(), 3600);
        });

        handle.join().unwrap();
        assert_eq!(manager.get_token(), Some("thread_token".to_string()));
    }

    #[test]
    fn test_access_token_clone() {
        let token1 = AccessToken::new("test_token".to_string(), 3600);
        let token2 = token1.clone();
        assert_eq!(token1.token, token2.token);
        assert_eq!(token1.expires_at, token2.expires_at);
    }

    #[test]
    fn test_token_manager_long_token_string() {
        let manager = TokenManager::new();
        let long_token = "a".repeat(10000);
        manager.set_token(long_token.clone(), 3600);
        assert_eq!(manager.get_token(), Some(long_token));
    }

    #[test]
    fn test_token_manager_empty_token_string() {
        let manager = TokenManager::new();
        manager.set_token("".to_string(), 3600);
        assert_eq!(manager.get_token(), Some("".to_string()));
        assert!(!manager.is_expired());
    }

    #[test]
    fn test_token_manager_zero_expiry() {
        let manager = TokenManager::new();
        manager.set_token("zero_expiry".to_string(), 0);
        assert!(manager.is_expired());
    }
}
