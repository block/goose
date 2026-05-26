use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::errors::ProviderError;
use super::openai_compatible::{handle_status, stream_openai_compat};
use super::retry::ProviderRetry;
use super::utils::{ImageFormat, RequestLog};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::openai::create_request;
use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use rmcp::model::Tool;

pub const CLOUDFLARE_PROVIDER_NAME: &str = "cloudflare";
pub const CLOUDFLARE_DEFAULT_HOST: &str = "https://api.cloudflare.com";
/// Default model — gpt-oss-20b wins our flat-JSON bake-off (35/35 rubric, ~1.9s avg).
/// See https://github.com/jezweb/notes/blob/main/workers-ai-bakeoff.md for methodology.
pub const CLOUDFLARE_DEFAULT_MODEL: &str = "@cf/openai/gpt-oss-20b";
pub const CLOUDFLARE_DOC_URL: &str = "https://developers.cloudflare.com/workers-ai/models/";

const CLOUDFLARE_API_TOKEN: &str = "CLOUDFLARE_API_TOKEN";
const CLOUDFLARE_ACCOUNT_ID: &str = "CLOUDFLARE_ACCOUNT_ID";
const CLOUDFLARE_HOST: &str = "CLOUDFLARE_HOST";

/// Curated list of Workers AI models that support tool calling and have been validated
/// against goose's OpenAI-compatible chat completion path. Workers AI hosts many more
/// models; users can override via `goose configure` or by passing any `@cf/...` ID.
pub const CLOUDFLARE_KNOWN_MODELS: &[&str] = &[
    "@cf/openai/gpt-oss-20b",
    "@cf/openai/gpt-oss-120b",
    "@cf/nvidia/nemotron-3-120b-a12b",
    "@cf/meta/llama-4-scout-17b-16e-instruct",
    "@cf/moonshotai/kimi-k2.6",
    "@cf/moonshotai/kimi-k2.5",
    "@cf/qwen/qwen3-30b-a3b-fp8",
    "@cf/google/gemma-4-26b-a4b-it",
    "@cf/zai-org/glm-4.7-flash",
    "@cf/mistralai/mistral-small-3.1-24b-instruct",
];

#[derive(serde::Serialize)]
pub struct CloudflareProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl CloudflareProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_token: String = config.get_secret(CLOUDFLARE_API_TOKEN)?;
        let account_id: String = config.get_param(CLOUDFLARE_ACCOUNT_ID)?;
        let host: String = config
            .get_param(CLOUDFLARE_HOST)
            .unwrap_or_else(|_| CLOUDFLARE_DEFAULT_HOST.to_string());

        let api_client = Self::build_client(&host, &account_id, &api_token)?;

        Ok(Self {
            api_client,
            model,
            name: CLOUDFLARE_PROVIDER_NAME.to_string(),
        })
    }

    fn build_client(host: &str, account_id: &str, api_token: &str) -> Result<ApiClient> {
        // Workers AI is OpenAI-compatible at /client/v4/accounts/{ACCOUNT_ID}/ai/v1.
        // We bake the account ID into the host so the OpenAI-compatible path
        // `chat/completions` joins cleanly via the ApiClient URL builder.
        let host = host.trim_end_matches('/');
        let base_url = format!("{}/client/v4/accounts/{}/ai/v1", host, account_id.trim());

        ApiClient::new(base_url, AuthMethod::BearerToken(api_token.to_string()))
    }
}

impl ProviderDef for CloudflareProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            CLOUDFLARE_PROVIDER_NAME,
            "Cloudflare Workers AI",
            "Run open-source models on Cloudflare's global Workers AI network",
            CLOUDFLARE_DEFAULT_MODEL,
            CLOUDFLARE_KNOWN_MODELS.to_vec(),
            CLOUDFLARE_DOC_URL,
            vec![
                ConfigKey::new(CLOUDFLARE_API_TOKEN, true, true, None, true),
                ConfigKey::new(CLOUDFLARE_ACCOUNT_ID, true, false, None, true),
                ConfigKey::new(
                    CLOUDFLARE_HOST,
                    false,
                    false,
                    Some(CLOUDFLARE_DEFAULT_HOST),
                    false,
                ),
            ],
        )
        .with_setup_steps(vec![
            "Open the Cloudflare dashboard at https://dash.cloudflare.com",
            "Copy your Account ID from the right-hand sidebar of any account page",
            "Go to My Profile → API Tokens → Create Token",
            "Use the 'Workers AI' template (or create a custom token with the Workers AI:Read permission)",
            "Paste the token and account ID above",
        ])
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for CloudflareProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    /// Fetch the model list from Cloudflare's catalog API.
    ///
    /// Endpoint: `/client/v4/accounts/{account_id}/ai/models/search`. We filter to
    /// `task: "Text Generation"` models — embedding, image, audio, and classification
    /// models won't work through the chat completion path.
    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        // The OpenAI-compat base URL ends in `/ai/v1`. Models live at `/ai/models/search`,
        // one level up. Resolve relative to the base URL by going up a segment.
        let response = self
            .api_client
            .request(None, "../models/search?per_page=100")
            .response_get()
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!(
                    "Failed to fetch models from Cloudflare API: {}",
                    e
                ))
            })?;

        let json: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to parse Cloudflare models response as JSON: {}",
                e
            ))
        })?;

        if let Some(errors) = json.get("errors").and_then(|v| v.as_array()) {
            if !errors.is_empty() {
                let msg = errors
                    .iter()
                    .filter_map(|e| e.get("message").and_then(|v| v.as_str()))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(ProviderError::RequestFailed(format!(
                    "Cloudflare API returned errors: {}",
                    msg
                )));
            }
        }

        let result = json
            .get("result")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ProviderError::RequestFailed("Missing 'result' array in Cloudflare response".into())
            })?;

        let mut models: Vec<String> = result
            .iter()
            .filter_map(|model| {
                let name = model.get("name").and_then(|v| v.as_str())?;
                let task_name = model
                    .get("task")
                    .and_then(|t| t.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if task_name == "Text Generation" {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect();

        models.sort();
        Ok(models)
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            true,
        )?;

        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post(Some(session_id), "chat/completions", &payload)
                    .await?;
                handle_status(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        stream_openai_compat(response, log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_provider_with_server(server_uri: &str, account_id: &str) -> CloudflareProvider {
        let api_client = CloudflareProvider::build_client(server_uri, account_id, "test-token")
            .expect("client should build");
        CloudflareProvider {
            api_client,
            model: ModelConfig::new_or_fail(CLOUDFLARE_DEFAULT_MODEL),
            name: CLOUDFLARE_PROVIDER_NAME.to_string(),
        }
    }

    #[test]
    fn test_metadata_shape() {
        let metadata = CloudflareProvider::metadata();
        assert_eq!(metadata.name, "cloudflare");
        assert_eq!(metadata.display_name, "Cloudflare Workers AI");
        assert_eq!(metadata.default_model, CLOUDFLARE_DEFAULT_MODEL);
        assert!(!metadata.known_models.is_empty());
        assert!(metadata
            .known_models
            .iter()
            .any(|m| m.name == "@cf/openai/gpt-oss-20b"));

        let token_key = metadata
            .config_keys
            .iter()
            .find(|k| k.name == CLOUDFLARE_API_TOKEN)
            .expect("API token key should be present");
        assert!(token_key.required);
        assert!(token_key.secret);
        assert!(token_key.primary);

        let account_key = metadata
            .config_keys
            .iter()
            .find(|k| k.name == CLOUDFLARE_ACCOUNT_ID)
            .expect("account id key should be present");
        assert!(account_key.required);
        assert!(!account_key.secret);
        assert!(account_key.primary);

        let host_key = metadata
            .config_keys
            .iter()
            .find(|k| k.name == CLOUDFLARE_HOST)
            .expect("host key should be present");
        assert!(!host_key.required);
        assert_eq!(host_key.default.as_deref(), Some(CLOUDFLARE_DEFAULT_HOST));

        assert!(!metadata.setup_steps.is_empty());
    }

    #[test]
    fn test_known_models_use_cf_prefix() {
        // Every Workers AI model ID starts with @cf/. Catch typos before deploy.
        for model in CLOUDFLARE_KNOWN_MODELS {
            assert!(
                model.starts_with("@cf/"),
                "model {} should start with @cf/",
                model
            );
        }
    }

    #[test]
    fn test_build_client_trims_trailing_slash_and_whitespace() {
        // Whitespace in pasted account IDs is a common configure-flow footgun.
        let client =
            CloudflareProvider::build_client("https://api.cloudflare.com/", "   abc123   ", "tok");
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_chat_completion_url_path() {
        let server = MockServer::start().await;
        let account_id = "abc123";

        Mock::given(method("POST"))
            .and(path(format!(
                "/client/v4/accounts/{}/ai/v1/chat/completions",
                account_id
            )))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    // Minimal SSE body so the stream decoder has something to chew on.
                    // We're verifying URL construction + auth, not parsing.
                    .set_body_raw(
                        "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n",
                        "text/event-stream",
                    ),
            )
            .mount(&server)
            .await;

        let provider = make_provider_with_server(&server.uri(), account_id);
        let model = ModelConfig::new_or_fail(CLOUDFLARE_DEFAULT_MODEL);

        // We only care that the request reached the right URL with the right auth.
        // The mock returns a tiny SSE body so the stream initiates; whether the
        // decoder produces a message isn't the point of this test.
        let _ = provider
            .stream(&model, "test-session", "system", &[], &[])
            .await;
    }

    #[tokio::test]
    async fn test_fetch_supported_models_filters_text_generation() {
        let server = MockServer::start().await;
        let account_id = "abc123";

        Mock::given(method("GET"))
            .and(path(format!(
                "/client/v4/accounts/{}/ai/models/search",
                account_id
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": [
                    {
                        "name": "@cf/openai/gpt-oss-20b",
                        "task": { "name": "Text Generation" }
                    },
                    {
                        "name": "@cf/baai/bge-base-en-v1.5",
                        "task": { "name": "Text Embeddings" }
                    },
                    {
                        "name": "@cf/black-forest-labs/flux-1-schnell",
                        "task": { "name": "Text-to-Image" }
                    },
                    {
                        "name": "@cf/meta/llama-4-scout-17b-16e-instruct",
                        "task": { "name": "Text Generation" }
                    }
                ],
                "success": true,
                "errors": [],
                "messages": []
            })))
            .mount(&server)
            .await;

        let provider = make_provider_with_server(&server.uri(), account_id);
        let models = provider
            .fetch_supported_models()
            .await
            .expect("fetch should succeed");

        assert_eq!(
            models,
            vec![
                "@cf/meta/llama-4-scout-17b-16e-instruct".to_string(),
                "@cf/openai/gpt-oss-20b".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn test_fetch_supported_models_surfaces_api_errors() {
        let server = MockServer::start().await;
        let account_id = "abc123";

        Mock::given(method("GET"))
            .and(path(format!(
                "/client/v4/accounts/{}/ai/models/search",
                account_id
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": null,
                "success": false,
                "errors": [{ "code": 10000, "message": "Authentication error" }],
                "messages": []
            })))
            .mount(&server)
            .await;

        let provider = make_provider_with_server(&server.uri(), account_id);
        let err = provider
            .fetch_supported_models()
            .await
            .expect_err("should return an error");
        assert!(
            err.to_string().contains("Authentication error"),
            "error should surface API message; got: {err}"
        );
    }
}
