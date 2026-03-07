//! Mesh provider — OpenAI-compatible endpoint with emulated tool calling.
//!
//! Sends requests to an external llama-server / mesh-llm endpoint but uses
//! the same emulated tool prompting as local inference: tools are described
//! in the system prompt and the model emits `$ command` or ` ```execute ``` `
//! blocks which are parsed into tool calls.
//!
//! This works with any model served by llama-server — no native tool-calling
//! support needed.

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ModelInfo, Provider, ProviderDef, ProviderMetadata, ProviderUsage, Usage};
use super::local_inference::inference_emulated_tools::{
    action_to_message, build_emulator_tool_description, load_tiny_model_prompt,
    StreamingEmulatorParser, CODE_EXECUTION_TOOL,
};
use super::errors::ProviderError;
use super::openai_compatible::handle_status_openai_compat;
use crate::config::declarative_providers::DeclarativeProviderConfig;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::utils::RequestLog;
use anyhow::Result;
use async_stream::try_stream;
use futures::future::BoxFuture;
use futures::TryStreamExt;
use tokio_stream::StreamExt;
use rmcp::model::{Role, Tool};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;
use uuid::Uuid;

const MESH_PROVIDER_NAME: &str = "mesh";
const MESH_DEFAULT_MODEL: &str = "GLM-4.7-Flash-Q4_K_M";
const MESH_DEFAULT_HOST: &str = "http://localhost:9337";

pub struct MeshProvider {
    api_client: ApiClient,
    model: ModelConfig,
    name: String,
}

impl MeshProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let host = std::env::var("MESH_HOST")
            .or_else(|_| std::env::var("OPENAI_HOST"))
            .unwrap_or_else(|_| MESH_DEFAULT_HOST.to_string());

        let timeout_secs: u64 = std::env::var("MESH_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(600);

        let api_client = ApiClient::with_timeout(
            host,
            AuthMethod::NoAuth,
            std::time::Duration::from_secs(timeout_secs),
        )?;

        Ok(Self {
            api_client,
            model,
            name: MESH_PROVIDER_NAME.to_string(),
        })
    }

    pub fn from_custom_config(
        model: ModelConfig,
        config: DeclarativeProviderConfig,
    ) -> Result<Self> {
        let url = url::Url::parse(&config.base_url)?;
        let host = if let Some(port) = url.port() {
            format!("{}://{}:{}", url.scheme(), url.host_str().unwrap_or(""), port)
        } else {
            format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""))
        };

        let timeout_secs = config.timeout_seconds.unwrap_or(600);
        let api_client = ApiClient::with_timeout(
            host,
            AuthMethod::NoAuth,
            std::time::Duration::from_secs(timeout_secs),
        )?;

        Ok(Self {
            api_client,
            model,
            name: config.name,
        })
    }
}

impl ProviderDef for MeshProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::with_models(
            MESH_PROVIDER_NAME,
            "mesh-llm",
            "Local mesh-llm or any llama-server endpoint with emulated tool calling",
            MESH_DEFAULT_MODEL,
            vec![ModelInfo::new(MESH_DEFAULT_MODEL, 32768)],
            "https://github.com/michaelneale/decentralized-inference",
            vec![
                super::base::ConfigKey::new(
                    "MESH_HOST",
                    true,
                    false,
                    Some(MESH_DEFAULT_HOST),
                    false,
                ),
                super::base::ConfigKey::new("MESH_TIMEOUT", false, false, Some("600"), false),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(async move { Self::from_env(model) })
    }
}

#[derive(Deserialize)]
struct SseDelta {
    choices: Vec<SseChoice>,
    usage: Option<Value>,
}

#[derive(Deserialize)]
struct SseChoice {
    delta: SseDeltaContent,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct SseDeltaContent {
    content: Option<String>,
}

fn extract_text_content(msg: &Message) -> String {
    msg.content
        .iter()
        .filter_map(|c| c.as_text())
        .collect::<Vec<_>>()
        .join("")
}

#[async_trait::async_trait]
impl Provider for MeshProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        _system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<super::base::MessageStream, ProviderError> {
        let code_mode_enabled = tools.iter().any(|t| t.name == CODE_EXECUTION_TOOL);

        // Build system prompt with tool descriptions (emulated mode)
        let mut system_prompt = load_tiny_model_prompt();
        if !tools.is_empty() {
            system_prompt.push_str(&build_emulator_tool_description(tools, code_mode_enabled));
        }

        // Build messages array — no tools parameter
        let mut messages_array = vec![json!({
            "role": "system",
            "content": system_prompt
        })];

        for msg in messages {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };
            let content = extract_text_content(msg);
            if !content.trim().is_empty() {
                messages_array.push(json!({
                    "role": role,
                    "content": content
                }));
            }
        }

        let payload = json!({
            "model": model_config.model_name,
            "messages": messages_array,
            "stream": true,
            "max_tokens": model_config.max_output_tokens(),
        });

        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .api_client
            .request(Some(session_id), "v1/chat/completions")
            .response_post(&payload)
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Request failed: {}", e)))?;

        let response = handle_status_openai_compat(response).await?;

        let message_id = Uuid::new_v4().to_string();

        let model_name = model_config.model_name.clone();

        Ok(Box::pin(try_stream! {
            let stream = response.bytes_stream().map_err(std::io::Error::other);
            let stream_reader = StreamReader::new(stream);
            let mut framed = FramedRead::new(stream_reader, LinesCodec::new());

            let mut parser = StreamingEmulatorParser::new(code_mode_enabled);
            let mut tool_call_emitted = false;
            let mut prompt_tokens = 0i32;
            let mut completion_tokens = 0i32;

            while let Some(line_result) = framed.next().await {
                if tool_call_emitted {
                    break;
                }

                let line = match line_result {
                    Ok(l) => l,
                    Err(_) => continue,
                };

                let line = line.trim();
                if !line.starts_with("data: ") {
                    continue;
                }
                let data = &line["data: ".len()..];
                if data == "[DONE]" {
                    break;
                }

                let chunk: SseDelta = match serde_json::from_str(data) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                // Capture usage if present
                if let Some(ref usage) = chunk.usage {
                    prompt_tokens = usage.get("prompt_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32;
                    completion_tokens = usage.get("completion_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32;
                }

                if chunk.choices.is_empty() {
                    continue;
                }

                let delta = &chunk.choices[0].delta;
                if let Some(ref text) = delta.content {
                    let actions = parser.process_chunk(text);
                    for action in &actions {
                        let (message, is_tool) = action_to_message(action, &message_id);
                        if is_tool {
                            tool_call_emitted = true;
                        }
                        yield (Some(message), None);
                        if tool_call_emitted {
                            break;
                        }
                    }
                }
            }

            // Flush remaining buffer
            if !tool_call_emitted {
                for action in parser.flush() {
                    let (message, _) = action_to_message(&action, &message_id);
                    yield (Some(message), None);
                }
            }

            // Emit usage
            let usage = Usage { input_tokens: Some(prompt_tokens), output_tokens: Some(completion_tokens), total_tokens: None };
            let provider_usage = ProviderUsage::new(model_name.clone(), usage);
            let _ = log.write(&json!({}), Some(&usage));
            yield (None, Some(provider_usage));
        }))
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request(None, "v1/models")
            .api_get()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to fetch models: {}", e)))?;

        let json = response.payload.unwrap_or_default();

        let models = json
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}
