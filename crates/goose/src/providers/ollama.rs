use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::errors::ProviderError;
use super::openai_compatible::handle_status_openai_compat;
use super::retry::ProviderRetry;
use super::utils::{ImageFormat, RequestLog};
use crate::config::declarative_providers::DeclarativeProviderConfig;
use crate::config::GooseMode;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::model::ModelConfig;
use crate::providers::formats::ollama::{create_request, response_to_streaming_message_ollama};
use crate::utils::safe_truncate;
use anyhow::{Error, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use futures::future::BoxFuture;
use futures::TryStreamExt;
use regex::Regex;
use reqwest::Response;
use rmcp::model::Tool;
use serde_json::Value;
use std::time::Duration;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;
use url::Url;

const OLLAMA_PROVIDER_NAME: &str = "ollama";
pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_TIMEOUT: u64 = 600;
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen3";
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[
    OLLAMA_DEFAULT_MODEL,
    "qwen3-vl",
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

        let base = if host.starts_with("http://") || host.starts_with("https://") {
            host.clone()
        } else {
            format!("http://{}", host)
        };

        let mut base_url =
            Url::parse(&base).map_err(|e| anyhow::anyhow!("Invalid base URL: {e}"))?;

        let explicit_port = host.contains(':');
        let is_localhost = host == "localhost" || host == "127.0.0.1" || host == "::1";

        if base_url.port().is_none() && !explicit_port && !host.starts_with("http") && is_localhost
        {
            base_url
                .set_port(Some(OLLAMA_DEFAULT_PORT))
                .map_err(|_| anyhow::anyhow!("Failed to set default port"))?;
        }

        let api_client =
            ApiClient::with_timeout(base_url.to_string(), AuthMethod::NoAuth, timeout)?;

        Ok(Self {
            api_client,
            model,
            supports_streaming: true,
            name: OLLAMA_PROVIDER_NAME.to_string(),
        })
    }

    pub fn from_custom_config(
        model: ModelConfig,
        config: DeclarativeProviderConfig,
    ) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(OLLAMA_TIMEOUT));

        let base =
            if config.base_url.starts_with("http://") || config.base_url.starts_with("https://") {
                config.base_url.clone()
            } else {
                format!("http://{}", config.base_url)
            };

        let mut base_url = Url::parse(&base)
            .map_err(|e| anyhow::anyhow!("Invalid base URL '{}': {}", config.base_url, e))?;

        let explicit_default_port =
            config.base_url.ends_with(":80") || config.base_url.ends_with(":443");
        let is_https = base_url.scheme() == "https";

        if base_url.port().is_none() && !explicit_default_port && !is_https {
            base_url
                .set_port(Some(OLLAMA_DEFAULT_PORT))
                .map_err(|_| anyhow::anyhow!("Failed to set default port"))?;
        }

        let mut api_client =
            ApiClient::with_timeout(base_url.to_string(), AuthMethod::NoAuth, timeout)?;

        if let Some(headers) = &config.headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
                let header_value = reqwest::header::HeaderValue::from_str(value)?;
                header_map.insert(header_name, header_value);
            }
            api_client = api_client.with_headers(header_map)?;
        }

        let supports_streaming = config.supports_streaming.unwrap_or(true);

        if !supports_streaming {
            return Err(anyhow::anyhow!(
                "Ollama provider does not support non-streaming mode. All Ollama models support streaming. \
                Please remove 'supports_streaming: false' from your provider configuration."
            ));
        }

        Ok(Self {
            api_client,
            model,
            supports_streaming,
            name: config.name.clone(),
        })
    }
}

impl ProviderDef for OllamaProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            OLLAMA_PROVIDER_NAME,
            "Ollama",
            "Local open source models",
            OLLAMA_DEFAULT_MODEL,
            OLLAMA_KNOWN_MODELS.to_vec(),
            OLLAMA_DOC_URL,
            vec![
                ConfigKey::new("OLLAMA_HOST", true, false, Some(OLLAMA_HOST), true),
                ConfigKey::new(
                    "OLLAMA_TIMEOUT",
                    false,
                    false,
                    Some(&(OLLAMA_TIMEOUT.to_string())),
                    false,
                ),
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
impl Provider for OllamaProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn generate_session_name(
        &self,
        session_id: &str,
        messages: &Conversation,
    ) -> Result<String, ProviderError> {
        let context = self.get_initial_user_messages(messages);
        let message = Message::user().with_text(self.create_session_name_prompt(&context));
        let model_config = self.get_model_config();
        let result = self
            .complete(
                &model_config,
                session_id,
                "You are a title generator. Output only the requested title of 4 words or less, with no additional text, reasoning, or explanations.",
                &[message],
                &[],
            )
            .await?;

        let mut description = result.0.as_concat_text();
        description = Self::filter_reasoning_tokens(&description);

        Ok(safe_truncate(&description, 100))
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let config = crate::config::Config::global();
        let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);
        let filtered_tools = if goose_mode == GooseMode::Chat {
            &[]
        } else {
            tools
        };

        let payload = create_request(
            model_config,
            system,
            messages,
            filtered_tools,
            &ImageFormat::OpenAi,
            true,
        )?;
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post(Some(session_id), "v1/chat/completions", &payload)
                    .await?;
                handle_status_openai_compat(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;
        stream_ollama(response, log)
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request(None, "api/tags")
            .response_get()
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

        model_names.sort();

        Ok(model_names)
    }
}

impl OllamaProvider {
    fn filter_reasoning_tokens(text: &str) -> String {
        let mut filtered = text.to_string();

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
        filtered = filtered
            .replace("<think>", "")
            .replace("</think>", "")
            .replace("<thinking>", "")
            .replace("</thinking>", "");
        filtered = filtered
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        filtered
    }
}

/// Ollama-specific streaming handler with XML tool call fallback.
/// Uses the Ollama format module which buffers text when XML tool calls are detected,
/// preventing duplicate content from being emitted to the UI.
fn stream_ollama(response: Response, mut log: RequestLog) -> Result<MessageStream, ProviderError> {
    let stream = response.bytes_stream().map_err(std::io::Error::other);

    Ok(Box::pin(try_stream! {
        let stream_reader = StreamReader::new(stream);
        let framed = FramedRead::new(stream_reader, LinesCodec::new())
            .map_err(Error::from);

        let message_stream = response_to_streaming_message_ollama(framed);
        pin!(message_stream);
        while let Some(message) = message_stream.next().await {
            let (message, usage) = message.map_err(|e|
                ProviderError::RequestFailed(format!("Stream decode error: {}", e))
            )?;
            log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
            yield (message, usage);
        }
    }))
}
