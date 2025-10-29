use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{
    get_model, handle_response_openai_compat, handle_status_openai_compat, RequestLog,
};
use crate::config::declarative_providers::DeclarativeProviderConfig;
use crate::conversation::message::Message;
use crate::conversation::Conversation;

use crate::model::ModelConfig;
use crate::providers::formats::openai::{
    create_request, get_usage, response_to_message, response_to_streaming_message,
};
use crate::utils::safe_truncate;
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use regex::Regex;
use rmcp::model::Tool;
use serde_json::{json, Value};
use std::io;
use std::time::Duration;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;
use url::Url;

pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_TIMEOUT: u64 = 600; // seconds
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen3";
// Ollama can run many models, we only provide the default
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[
    OLLAMA_DEFAULT_MODEL,
    "qwen3-coder:30b",
    "qwen3-coder:480b-cloud",
];
pub const OLLAMA_DOC_URL: &str = "https://ollama.com/library";

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    supports_streaming: bool,
    name: String,
}

impl OllamaProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
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
        // 1. URL/host explicitly contains ports
        // 2. URL/host uses HTTP/S
        // 3. only set it for localhost
        let explicit_port = host.contains(':');
        let is_localhost = host == "localhost" || host == "127.0.0.1" || host == "::1";

        if base_url.port().is_none() && !explicit_port && !host.starts_with("http") && is_localhost
        {
            base_url
                .set_port(Some(OLLAMA_DEFAULT_PORT))
                .map_err(|_| anyhow::anyhow!("Failed to set default port"))?;
        }

        // No authentication for Ollama
        let auth = AuthMethod::Custom(Box::new(NoAuth));
        let api_client = ApiClient::with_timeout(base_url.to_string(), auth, timeout)?;

        Ok(Self {
            api_client,
            model,
            supports_streaming: true,
            name: Self::metadata().name,
        })
    }

    pub fn from_custom_config(
        model: ModelConfig,
        config: DeclarativeProviderConfig,
    ) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(OLLAMA_TIMEOUT));

        // Parse and normalize the custom URL
        let base =
            if config.base_url.starts_with("http://") || config.base_url.starts_with("https://") {
                config.base_url.clone()
            } else {
                format!("http://{}", config.base_url)
            };

        let mut base_url = Url::parse(&base)
            .map_err(|e| anyhow::anyhow!("Invalid base URL '{}': {}", config.base_url, e))?;

        // Set default port if missing and not using standard ports
        let explicit_default_port =
            config.base_url.ends_with(":80") || config.base_url.ends_with(":443");
        let is_https = base_url.scheme() == "https";

        if base_url.port().is_none() && !explicit_default_port && !is_https {
            base_url
                .set_port(Some(OLLAMA_DEFAULT_PORT))
                .map_err(|_| anyhow::anyhow!("Failed to set default port"))?;
        }

        // No authentication for Ollama
        let auth = AuthMethod::Custom(Box::new(NoAuth));
        let api_client = ApiClient::with_timeout(base_url.to_string(), auth, timeout)?;

        Ok(Self {
            api_client,
            model,
            supports_streaming: config.supports_streaming.unwrap_or(true),
            name: config.name.clone(),
        })
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

    fn get_name(&self) -> &str {
        &self.name
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
        let response_model = get_model(&response);
        let mut log = RequestLog::start(model_config, &payload)?;
        log.write(&response, Some(&usage))?;
        Ok((message, ProviderUsage::new(response_model, usage)))
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

    fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_request(
            &self.model,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
        )?;
        payload["stream"] = json!(true);
        payload["stream_options"] = json!({
            "include_usage": true,
        });

        let response = self
            .api_client
            .response_post("v1/chat/completions", &payload)
            .await?;
        let response = handle_status_openai_compat(response).await?;
        let stream = response.bytes_stream().map_err(io::Error::other);
        let model_config = self.model.clone();

        Ok(Box::pin(try_stream! {
            let mut log = RequestLog::start(&model_config, &payload)?;
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);
            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                log.write(&message, usage.as_ref().map(|f| &f.usage))?;
                yield (message, usage);
            }
        }))
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let response = self
            .api_client
            .response_get("api/tags")
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            return Err(ProviderError::RequestFailed(format!(
                "Failed to fetch models: HTTP {}",
                response.status()
            )));
        }

        let json_response = response.json::<Value>().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse response: {}", e))
        })?;

        let models = json_response
            .get("models")
            .and_then(|m| m.as_array())
            .ok_or_else(|| {
                ProviderError::RequestFailed("No models array in response".to_string())
            })?;

        let mut model_names: Vec<String> = models
            .iter()
            .filter_map(|model| model.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect();

        // Sort alphabetically
        model_names.sort();

        Ok(Some(model_names))
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
