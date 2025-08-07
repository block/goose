use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{get_model, handle_response_openai_compat};
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::impl_provider_default;
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use crate::utils::safe_truncate;
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use rmcp::model::Tool;
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_TIMEOUT: u64 = 600; // seconds
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen2.5";
// Ollama can run many models, we only provide the default
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[OLLAMA_DEFAULT_MODEL];
pub const OLLAMA_DOC_URL: &str = "https://ollama.com/library";

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
}

impl_provider_default!(OllamaProvider);

impl OllamaProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let host: String = config
            .get_param("OLLAMA_HOST")
            .unwrap_or_else(|_| OLLAMA_HOST.to_string());

        let timeout: Duration =
            Duration::from_secs(config.get_param("OLLAMA_TIMEOUT").unwrap_or(OLLAMA_TIMEOUT));

        // OLLAMA_HOST is sometimes just the 'host' or 'host:port' without a scheme
        let base = if host.starts_with("http://") || host.starts_with("https://") {
            host.clone()
        } else {
            format!("http://{}", host)
        };

        let mut base_url =
            Url::parse(&base).map_err(|e| anyhow::anyhow!("Invalid base URL: {e}"))?;

        // Set the default port if missing
        // Don't add default port if:
        // 1. URL explicitly ends with standard ports (:80 or :443)
        // 2. URL uses HTTPS (which implicitly uses port 443)
        let explicit_default_port = host.ends_with(":80") || host.ends_with(":443");
        let is_https = base_url.scheme() == "https";

        if base_url.port().is_none() && !explicit_default_port && !is_https {
            base_url
                .set_port(Some(OLLAMA_DEFAULT_PORT))
                .map_err(|_| anyhow::anyhow!("Failed to set default port"))?;
        }

        // No authentication for Ollama
        let auth = AuthMethod::Custom(Box::new(NoAuth));
        let api_client = ApiClient::with_timeout(base_url.to_string(), auth, timeout)?;

        Ok(Self { api_client, model })
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post("v1/chat/completions", payload)
            .await?;
        handle_response_openai_compat(response).await
    }
}

// No authentication provider for Ollama
struct NoAuth;

#[async_trait]
impl super::api_client::AuthProvider for NoAuth {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        // Return a dummy header that won't be used
        Ok(("X-No-Auth".to_string(), "true".to_string()))
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "ollama",
            "Ollama",
            "Local open source models",
            OLLAMA_DEFAULT_MODEL,
            OLLAMA_KNOWN_MODELS.to_vec(),
            OLLAMA_DOC_URL,
            vec![
                ConfigKey::new("OLLAMA_HOST", true, false, Some(OLLAMA_HOST)),
                ConfigKey::new(
                    "OLLAMA_TIMEOUT",
                    false,
                    false,
                    Some(&(OLLAMA_TIMEOUT.to_string())),
                ),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let config = crate::config::Config::global();
        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());
        let filtered_tools = if goose_mode == "chat" { &[] } else { tools };

        let payload = create_request(
            &self.model,
            system,
            messages,
            filtered_tools,
            &super::utils::ImageFormat::OpenAi,
        )?;
        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post(&payload_clone).await
            })
            .await?;
        let message = response_to_message(&response.clone())?;

        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let model = get_model(&response);
        super::utils::emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }

    /// Generate a session name based on the conversation history
    /// This override filters out reasoning tokens that some Ollama models produce
    async fn generate_session_name(
        &self,
        messages: &Conversation,
    ) -> Result<String, ProviderError> {
        let context = self.get_initial_user_messages(messages);
        let message = Message::user().with_text(self.create_session_name_prompt(&context));
        let result = self
            .complete(
                "You are a title generator. Output only the requested title of 4 words or less, with no additional text, reasoning, or explanations.",
                &[message],
                &[],
            )
            .await?;

        let mut description = result.0.as_concat_text();
        description = Self::filter_reasoning_tokens(&description);

        Ok(safe_truncate(&description, 100))
    }
}

impl OllamaProvider {
    /// Filter out reasoning tokens and thinking patterns from model responses
    fn filter_reasoning_tokens(text: &str) -> String {
        let mut filtered = text.to_string();

        // Remove common reasoning patterns
        let reasoning_patterns = [
            r"<think>.*?</think>",
            r"<thinking>.*?</thinking>",
            r"Let me think.*?\n",
            r"I need to.*?\n",
            r"First, I.*?\n",
            r"Okay, .*?\n",
            r"So, .*?\n",
            r"Well, .*?\n",
            r"Hmm, .*?\n",
            r"Actually, .*?\n",
            r"Based on.*?I think",
            r"Looking at.*?I would say",
        ];

        for pattern in reasoning_patterns {
            if let Ok(re) = Regex::new(pattern) {
                filtered = re.replace_all(&filtered, "").to_string();
            }
        }
        // Remove any remaining thinking markers
        filtered = filtered
            .replace("<think>", "")
            .replace("</think>", "")
            .replace("<thinking>", "")
            .replace("</thinking>", "");
        // Clean up extra whitespace
        filtered = filtered
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        filtered
    }
}
