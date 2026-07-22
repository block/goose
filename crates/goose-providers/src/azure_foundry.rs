use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;

use crate::anthropic::{AnthropicProvider, AnthropicProviderBuilder, ANTHROPIC_API_VERSION};
use crate::api_client::{ApiClient, AuthMethod, RequestBuilderDecorator, TlsConfig};
use crate::base::{
    ConfigKey, MessageStream, ModelInfo, Provider, ProviderDescriptor, ProviderMetadata,
};
use crate::conversation::message::Message;
use crate::errors::ProviderError;
use crate::model::ModelConfig;
use crate::openai::{OpenAiProvider, OpenAiProviderBuilder};
use crate::openai_compatible::{handle_response_openai_compat, OpenAiCompatibleProvider};

pub const AZURE_FOUNDRY_PROVIDER_NAME: &str = "azure_foundry";
pub const AZURE_FOUNDRY_DEFAULT_MODEL: &str = "Phi-4";
pub const AZURE_FOUNDRY_DOC_URL: &str =
    "https://learn.microsoft.com/azure/ai-foundry/foundry-models/how-to/inference";

pub const AZURE_FOUNDRY_KNOWN_MODELS: &[&str] = &[
    "Phi-4",
    "Phi-4-mini",
    "Meta-Llama-3.3-70B-Instruct",
    "Mistral-large-2411",
    "Cohere-command-r-plus-08-2024",
    "AI21-Jamba-1.5-Large",
    "DeepSeek-R1",
    "DeepSeek-V3",
    "glm-4.7",
    "Kimi-K2-Instruct",
    "claude-sonnet-4-6",
    "claude-opus-4-6",
    "gpt-5",
    "o3",
];

pub fn is_project_endpoint(endpoint: &str) -> bool {
    endpoint.contains("/api/projects/")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPublisher {
    OpenAi,
    Anthropic,
    Partner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeploymentMetadata {
    publisher: ModelPublisher,
    model_name: String,
}

impl ModelPublisher {
    fn from_azure(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "openai" => Self::OpenAi,
            "anthropic" => Self::Anthropic,
            _ => Self::Partner,
        }
    }

    fn from_model_name(value: &str) -> Self {
        let value = value.to_ascii_lowercase();
        if value.starts_with("claude") {
            Self::Anthropic
        } else if value.starts_with("gpt-")
            || value == "o1"
            || value.starts_with("o1-")
            || value == "o3"
            || value.starts_with("o3-")
            || value == "o4"
            || value.starts_with("o4-")
        {
            Self::OpenAi
        } else {
            Self::Partner
        }
    }
}

pub struct AzureFoundryProvider {
    chat: OpenAiCompatibleProvider,
    responses: Option<OpenAiProvider>,
    anthropic: Option<AnthropicProvider>,
    deployments_client: ApiClient,
    endpoint: String,
    api_version: Option<String>,
    deployments: Mutex<HashMap<String, DeploymentMetadata>>,
}

impl ProviderDescriptor for AzureFoundryProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            AZURE_FOUNDRY_PROVIDER_NAME,
            "Azure AI Foundry",
            "OpenAI, Anthropic, and partner models deployed through Azure AI Foundry",
            AZURE_FOUNDRY_DEFAULT_MODEL,
            AZURE_FOUNDRY_KNOWN_MODELS.to_vec(),
            AZURE_FOUNDRY_DOC_URL,
            vec![
                ConfigKey::new("AZURE_FOUNDRY_ENDPOINT", true, false, None, true),
                ConfigKey::new("AZURE_FOUNDRY_API_KEY", false, true, Some(""), true),
                ConfigKey::new("AZURE_FOUNDRY_AD_TOKEN", false, true, Some(""), false),
                ConfigKey::new("AZURE_FOUNDRY_API_VERSION", false, false, None, false),
            ],
        )
    }
}

impl AzureFoundryProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        endpoint: String,
        api_version: Option<String>,
        chat_auth: AuthMethod,
        responses_auth: AuthMethod,
        anthropic_auth: AuthMethod,
        deployments_auth: AuthMethod,
        tls_config: Option<TlsConfig>,
        request_builder: Option<RequestBuilderDecorator>,
    ) -> Result<Self> {
        let endpoint = endpoint.trim_end_matches('/').to_string();
        let project = is_project_endpoint(&endpoint);
        let chat_prefix = if project { "openai/v1/" } else { "" };

        let chat_client = configured_client(
            endpoint.clone(),
            chat_auth,
            tls_config.clone(),
            request_builder.clone(),
        )?;
        let chat = OpenAiCompatibleProvider::new(
            AZURE_FOUNDRY_PROVIDER_NAME.to_string(),
            chat_client,
            chat_prefix.to_string(),
        );

        let (responses, anthropic) = if project {
            let responses_client = configured_client(
                endpoint.clone(),
                responses_auth,
                tls_config.clone(),
                request_builder.clone(),
            )?;
            let responses = OpenAiProviderBuilder::new(responses_client)
                .name(AZURE_FOUNDRY_PROVIDER_NAME)
                .base_path("openai/v1/responses")
                .skip_canonical_filtering(true)
                .build();

            let hub = endpoint.split("/api/projects/").next().unwrap_or(&endpoint);
            let anthropic_client = configured_client(
                format!("{hub}/anthropic"),
                anthropic_auth,
                tls_config.clone(),
                request_builder.clone(),
            )?
            .with_header("anthropic-version", ANTHROPIC_API_VERSION)?;
            let anthropic = AnthropicProviderBuilder::new(anthropic_client)
                .name(AZURE_FOUNDRY_PROVIDER_NAME)
                .skip_canonical_filtering(true)
                .build();
            (Some(responses), Some(anthropic))
        } else {
            (None, None)
        };

        let deployments_client = configured_client(
            endpoint.clone(),
            deployments_auth,
            tls_config,
            request_builder,
        )?;

        Ok(Self {
            chat,
            responses,
            anthropic,
            deployments_client,
            endpoint,
            api_version,
            deployments: Mutex::new(HashMap::new()),
        })
    }

    async fn fetch_deployments(
        &self,
    ) -> Result<(Vec<String>, HashMap<String, DeploymentMetadata>), ProviderError> {
        let version = self
            .api_version
            .as_deref()
            .or_else(|| is_project_endpoint(&self.endpoint).then_some("v1"));
        let mut next = Some(match version {
            Some(version) => format!("deployments?api-version={version}"),
            None => "deployments".to_string(),
        });
        let mut models = Vec::new();
        let mut deployments = HashMap::new();

        while let Some(path) = next {
            let response = self
                .deployments_client
                .response_get(&path)
                .await
                .map_err(|error| ProviderError::NetworkError(error.to_string()))?;
            let json = handle_response_openai_compat(response).await?;
            if let Some(items) = json.get("value").and_then(|value| value.as_array()) {
                for item in items {
                    let Some(name) = item.get("name").and_then(|value| value.as_str()) else {
                        continue;
                    };
                    let model_name = item
                        .get("modelName")
                        .and_then(|value| value.as_str())
                        .unwrap_or(name);
                    let publisher = item
                        .get("modelPublisher")
                        .and_then(|value| value.as_str())
                        .map(ModelPublisher::from_azure)
                        .unwrap_or_else(|| ModelPublisher::from_model_name(model_name));
                    models.push(name.to_string());
                    deployments.insert(
                        name.to_string(),
                        DeploymentMetadata {
                            publisher,
                            model_name: model_name.to_string(),
                        },
                    );
                }
            }
            next = json
                .get("nextLink")
                .and_then(|value| value.as_str())
                .map(|link| with_api_version(link, version));
        }

        models.sort();
        models.dedup();
        Ok((models, deployments))
    }

    async fn deployment_for(&self, deployment_name: &str) -> Option<DeploymentMetadata> {
        if let Some(deployment) = self
            .deployments
            .lock()
            .expect("Azure Foundry deployment cache poisoned")
            .get(deployment_name)
            .cloned()
        {
            return Some(deployment);
        }

        let (_, deployments) = self.fetch_deployments().await.ok()?;
        let deployment = deployments.get(deployment_name).cloned();
        *self
            .deployments
            .lock()
            .expect("Azure Foundry deployment cache poisoned") = deployments;
        deployment
    }
}

fn with_api_version(link: &str, api_version: Option<&str>) -> String {
    let Some(api_version) = api_version else {
        return link.to_string();
    };
    if link.contains("api-version=") {
        return link.to_string();
    }
    let separator = if link.contains('?') { '&' } else { '?' };
    format!("{link}{separator}api-version={api_version}")
}

fn model_info_for_deployment(deployment_name: &str, model_name: &str) -> ModelInfo {
    let canonical = crate::canonical::maybe_get_canonical_model("azure_foundry", model_name)
        .or_else(|| {
            crate::canonical::maybe_get_canonical_model(
                "azure_foundry",
                &model_name.to_ascii_lowercase(),
            )
        });
    ModelInfo {
        name: deployment_name.to_string(),
        resolved_model: Some(model_name.to_string()),
        context_limit: canonical
            .as_ref()
            .map(|model| model.limit.context)
            .unwrap_or_else(|| ModelConfig::new(model_name).context_limit()),
        input_token_cost: None,
        output_token_cost: None,
        currency: None,
        supports_cache_control: None,
        reasoning: canonical
            .and_then(|model| model.reasoning)
            .unwrap_or_else(|| ModelConfig::new(model_name).is_reasoning_model()),
    }
}

fn configured_client(
    host: String,
    auth: AuthMethod,
    tls_config: Option<TlsConfig>,
    request_builder: Option<RequestBuilderDecorator>,
) -> Result<ApiClient> {
    let mut client = ApiClient::new_with_tls(host, auth, tls_config)?;
    if let Some(request_builder) = request_builder {
        client = client.with_request_builder(request_builder);
    }
    Ok(client)
}

#[async_trait]
impl Provider for AzureFoundryProvider {
    fn get_name(&self) -> &str {
        AZURE_FOUNDRY_PROVIDER_NAME
    }

    fn skip_canonical_filtering(&self) -> bool {
        true
    }

    async fn refresh_credentials(&self) -> Result<(), ProviderError> {
        self.deployments_client
            .refresh_credentials()
            .await
            .map_err(|error| ProviderError::Authentication(error.to_string()))
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        if !is_project_endpoint(&self.endpoint) {
            return Ok(AZURE_FOUNDRY_KNOWN_MODELS
                .iter()
                .map(ToString::to_string)
                .collect());
        }
        let (models, deployments) = self.fetch_deployments().await?;
        *self
            .deployments
            .lock()
            .expect("Azure Foundry deployment cache poisoned") = deployments;
        Ok(models)
    }

    async fn fetch_supported_model_info(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        if !is_project_endpoint(&self.endpoint) {
            return Ok(AZURE_FOUNDRY_KNOWN_MODELS
                .iter()
                .map(|model| model_info_for_deployment(model, model))
                .collect());
        }
        let (models, deployments) = self.fetch_deployments().await?;
        let model_info = models
            .iter()
            .filter_map(|name| {
                deployments
                    .get(name)
                    .map(|deployment| model_info_for_deployment(name, &deployment.model_name))
            })
            .collect();
        *self
            .deployments
            .lock()
            .expect("Azure Foundry deployment cache poisoned") = deployments;
        Ok(model_info)
    }

    async fn fetch_model_info(&self, model_name: &str) -> Result<ModelInfo, ProviderError> {
        let resolved_model = if is_project_endpoint(&self.endpoint) {
            self.deployment_for(model_name)
                .await
                .map(|deployment| deployment.model_name)
                .unwrap_or_else(|| model_name.to_string())
        } else {
            model_name.to_string()
        };
        Ok(model_info_for_deployment(model_name, &resolved_model))
    }

    async fn get_context_limit(&self, model_config: &ModelConfig) -> Result<usize, ProviderError> {
        if let Some(context_limit) = model_config.context_limit {
            return Ok(context_limit);
        }
        Ok(self
            .fetch_model_info(&model_config.model_name)
            .await?
            .context_limit)
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let deployment = if self.responses.is_some() {
            self.deployment_for(&model_config.model_name).await
        } else {
            None
        };
        let publisher = deployment
            .as_ref()
            .map(|deployment| deployment.publisher)
            .unwrap_or_else(|| ModelPublisher::from_model_name(&model_config.model_name));
        let capability_config = deployment
            .filter(|deployment| deployment.model_name != model_config.model_name)
            .map(|deployment| {
                model_config
                    .clone()
                    .with_capability_model_name(deployment.model_name)
            });
        let request_config = capability_config.as_ref().unwrap_or(model_config);

        match publisher {
            ModelPublisher::OpenAi if self.responses.is_some() => {
                self.responses
                    .as_ref()
                    .expect("checked above")
                    .stream(request_config, system, messages, tools)
                    .await
            }
            ModelPublisher::Anthropic if self.anthropic.is_some() => {
                self.anthropic
                    .as_ref()
                    .expect("checked above")
                    .stream(request_config, system, messages, tools)
                    .await
            }
            _ => {
                self.chat
                    .stream(request_config, system, messages, tools)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn project_endpoint(server: &MockServer) -> String {
        format!("{}/api/projects/test", server.uri())
    }

    fn project_provider(server: &MockServer) -> AzureFoundryProvider {
        AzureFoundryProvider::create(
            project_endpoint(server),
            None,
            AuthMethod::NoAuth,
            AuthMethod::NoAuth,
            AuthMethod::NoAuth,
            AuthMethod::NoAuth,
            None,
            None,
        )
        .unwrap()
    }

    fn chat_stream() -> String {
        [
            json!({"id":"c1","object":"chat.completion.chunk","model":"test","choices":[{"delta":{"role":"assistant","content":"Hello"},"index":0}]}),
            json!({"id":"c1","object":"chat.completion.chunk","choices":[{"delta":{},"finish_reason":"stop","index":0}]}),
            json!({"id":"c1","object":"chat.completion.chunk","choices":[],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}),
        ]
        .into_iter()
        .map(|chunk| format!("data: {chunk}\n\n"))
        .chain(std::iter::once("data: [DONE]\n\n".to_string()))
        .collect()
    }

    fn responses_stream() -> String {
        let created = r#"data: {"type":"response.created","sequence_number":1,"response":{"id":"resp_1","object":"response","created_at":0,"status":"in_progress","model":"gpt-5","output":[]}}"#;
        let delta = r#"data: {"type":"response.output_text.delta","sequence_number":2,"item_id":"m1","output_index":0,"content_index":0,"delta":"Hello"}"#;
        let completed = r#"data: {"type":"response.completed","sequence_number":3,"response":{"id":"resp_1","object":"response","created_at":0,"status":"completed","model":"gpt-5","output":[],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}"#;
        format!("{created}\n\n{delta}\n\n{completed}\n\ndata: [DONE]\n\n")
    }

    fn anthropic_stream() -> String {
        let start = r#"data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-6","stop_reason":null,"usage":{"input_tokens":1,"output_tokens":0}}}"#;
        let block_start = r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#;
        let delta = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let block_stop = r#"data: {"type":"content_block_stop","index":0}"#;
        let message_delta = r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":1}}"#;
        let stop = r#"data: {"type":"message_stop"}"#;
        format!(
            "{start}\n\n{block_start}\n\n{delta}\n\n{block_stop}\n\n{message_delta}\n\n{stop}\n\n"
        )
    }

    #[test]
    fn publisher_routing_covers_all_surfaces() {
        assert_eq!(ModelPublisher::from_azure("OpenAI"), ModelPublisher::OpenAi);
        assert_eq!(
            ModelPublisher::from_azure("Anthropic"),
            ModelPublisher::Anthropic
        );
        assert_eq!(
            ModelPublisher::from_azure("MistralAI"),
            ModelPublisher::Partner
        );
        assert_eq!(ModelPublisher::from_azure("Zhipu"), ModelPublisher::Partner);
        assert_eq!(
            ModelPublisher::from_azure("Moonshot AI"),
            ModelPublisher::Partner
        );
    }

    #[test]
    fn model_fallback_only_routes_known_native_families() {
        assert_eq!(
            ModelPublisher::from_model_name("gpt-5"),
            ModelPublisher::OpenAi
        );
        assert_eq!(
            ModelPublisher::from_model_name("o3-mini"),
            ModelPublisher::OpenAi
        );
        assert_eq!(
            ModelPublisher::from_model_name("claude-sonnet-4-6"),
            ModelPublisher::Anthropic
        );
        assert_eq!(
            ModelPublisher::from_model_name("Mistral-large"),
            ModelPublisher::Partner
        );
        assert_eq!(
            ModelPublisher::from_model_name("glm-4.7"),
            ModelPublisher::Partner
        );
        assert_eq!(
            ModelPublisher::from_model_name("Kimi-K2"),
            ModelPublisher::Partner
        );
    }

    #[test]
    fn endpoint_type_is_detected() {
        assert!(is_project_endpoint(
            "https://hub.services.ai.azure.com/api/projects/project"
        ));
        assert!(!is_project_endpoint(
            "https://deployment.eastus.models.ai.azure.com"
        ));
    }

    #[test]
    fn pagination_link_keeps_api_version() {
        assert_eq!(
            with_api_version("https://example.test/deployments?page=2", Some("v1")),
            "https://example.test/deployments?page=2&api-version=v1"
        );
        assert_eq!(
            with_api_version(
                "https://example.test/deployments?page=2&api-version=2025-05-01",
                Some("v1")
            ),
            "https://example.test/deployments?page=2&api-version=2025-05-01"
        );
    }

    #[test]
    fn deployment_metadata_enriches_context_without_pricing() {
        let info = model_info_for_deployment("production-chat", "gpt-5");
        assert_eq!(info.name, "production-chat");
        assert_eq!(info.resolved_model.as_deref(), Some("gpt-5"));
        assert_eq!(info.context_limit, 400_000);
        assert_eq!(info.input_token_cost, None);
        assert_eq!(info.output_token_cost, None);
    }

    #[tokio::test]
    async fn deployment_discovery_is_paginated_and_preserves_underlying_model() {
        let server = MockServer::start().await;
        let page_two = format!("{}/api/projects/test/deployments?page=2", server.uri());
        Mock::given(method("GET"))
            .and(path("/api/projects/test/deployments"))
            .and(query_param("api-version", "v1"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{
                    "type": "ModelDeployment",
                    "name": "partner-prod",
                    "modelName": "Mistral-large",
                    "modelVersion": "1",
                    "modelPublisher": "MistralAI"
                }]
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/projects/test/deployments"))
            .and(query_param("api-version", "v1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{
                    "type": "ModelDeployment",
                    "name": "openai-prod",
                    "modelName": "gpt-5",
                    "modelVersion": "1",
                    "modelPublisher": "OpenAI"
                }],
                "nextLink": page_two
            })))
            .expect(1)
            .mount(&server)
            .await;

        let provider = project_provider(&server);
        let (names, deployments) = provider.fetch_deployments().await.unwrap();
        assert_eq!(names, vec!["openai-prod", "partner-prod"]);
        assert_eq!(deployments["openai-prod"].model_name, "gpt-5");
        assert_eq!(deployments["openai-prod"].publisher, ModelPublisher::OpenAi);
    }

    #[tokio::test]
    async fn custom_deployment_context_uses_underlying_model() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/projects/test/deployments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{
                    "type": "ModelDeployment",
                    "name": "production-chat",
                    "modelName": "gpt-5",
                    "modelVersion": "1",
                    "modelPublisher": "OpenAI"
                }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let provider = project_provider(&server);
        assert_eq!(
            provider
                .get_context_limit(&ModelConfig::new("production-chat"))
                .await
                .unwrap(),
            400_000
        );
    }

    #[tokio::test]
    async fn project_inventory_failure_is_propagated() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/projects/test/deployments"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": {"message": "expired"}
            })))
            .mount(&server)
            .await;

        let provider = project_provider(&server);
        assert!(provider.fetch_supported_models().await.is_err());
        assert!(provider.fetch_supported_model_info().await.is_err());
    }

    #[tokio::test]
    async fn custom_openai_deployment_uses_alias_and_underlying_capabilities() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/projects/test/deployments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{
                    "type": "ModelDeployment",
                    "name": "production-chat",
                    "modelName": "gpt-5",
                    "modelPublisher": "OpenAI"
                }]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/projects/test/openai/v1/responses"))
            .and(body_partial_json(json!({"model": "production-chat"})))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(responses_stream())
                    .append_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider = project_provider(&server);
        let config = ModelConfig::new("production-chat").with_temperature(Some(0.7));
        provider
            .complete(&config, "system", &[], &[])
            .await
            .unwrap();
        let requests = server.received_requests().await.unwrap();
        let payload: serde_json::Value = requests
            .iter()
            .find(|request| request.url.path().ends_with("/responses"))
            .unwrap()
            .body_json()
            .unwrap();
        assert_eq!(payload["model"], "production-chat");
        assert!(payload.get("temperature").is_none());
    }

    #[tokio::test]
    async fn custom_deployment_names_route_to_all_three_surfaces() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/projects/test/deployments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [
                    {"type":"ModelDeployment","name":"openai-prod","modelName":"gpt-5","modelVersion":"1","modelPublisher":"OpenAI"},
                    {"type":"ModelDeployment","name":"claude-prod","modelName":"claude-sonnet-4-6","modelVersion":"1","modelPublisher":"Anthropic"},
                    {"type":"ModelDeployment","name":"partner-prod","modelName":"Mistral-large","modelVersion":"1","modelPublisher":"MistralAI"}
                ]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/projects/test/openai/v1/responses"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(responses_stream())
                    .append_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/anthropic/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(anthropic_stream())
                    .append_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/projects/test/openai/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(chat_stream())
                    .append_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider = project_provider(&server);
        for deployment in ["openai-prod", "claude-prod", "partner-prod"] {
            provider
                .complete(&ModelConfig::new(deployment), "system", &[], &[])
                .await
                .unwrap();
        }
    }
}
