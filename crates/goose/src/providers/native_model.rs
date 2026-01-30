use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use crate::conversation::message::Message;
use crate::impl_provider_default;
use crate::model::ModelConfig;
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub const NATIVE_DEFAULT_MODEL: &str = "qwen2.5:7b-instruct";
pub const NATIVE_KNOWN_MODELS: &[&str] = &[
    "llama3.2:1b",
    "llama3.2:3b",
    "qwen2.5:3b-instruct",
    "qwen2.5:7b-instruct",
    "qwen2.5:7b",
];
pub const NATIVE_DOC_URL: &str = "https://ollama.com/library";

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    top_p: f32,
    top_k: i32,
    num_predict: i32,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    done: bool,
}

/// Provider for locally hosted models using Ollama (full Metal GPU support + LoRA)
#[derive(serde::Serialize)]
pub struct NativeModelProvider {
    model: ModelConfig,
    ollama_host: String,
    lora_adapter_path: Option<PathBuf>,
    #[serde(skip)]
    lora_server_port: Option<u16>,
}

impl_provider_default!(NativeModelProvider);

impl NativeModelProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        // Get Ollama host from environment
        let ollama_host = config
            .get_param("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        tracing::info!("Creating Native Model Provider (Ollama backend) at: {}", ollama_host);

        // Check for LoRA adapter path
        let lora_adapter_path = config
            .get_param::<String>("NATIVE_LORA_ADAPTER_PATH")
            .ok()
            .map(PathBuf::from);

        if let Some(ref path) = lora_adapter_path {
            tracing::info!("LoRA adapter configured: {}", path.display());
        }

        Ok(Self {
            model,
            ollama_host,
            lora_adapter_path,
            lora_server_port: None,
        })
    }

    fn build_prompt(&self, system: &str, messages: &[Message]) -> String {
        // Qwen2 chat template
        let mut prompt = String::new();
        
        if !system.trim().is_empty() {
            prompt.push_str("<|im_start|>system\n");
            prompt.push_str(system.trim());
            prompt.push_str("\n<|im_end|>\n");
        }
        
        for m in messages {
            match m.role {
                rmcp::model::Role::User => {
                    prompt.push_str("<|im_start|>user\n");
                    prompt.push_str(&m.as_concat_text());
                    prompt.push_str("\n<|im_end|>\n");
                }
                rmcp::model::Role::Assistant => {
                    prompt.push_str("<|im_start|>assistant\n");
                    prompt.push_str(&m.as_concat_text());
                    prompt.push_str("\n<|im_end|>\n");
                }
            }
        }
        
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }

    fn get_model_name(&self) -> String {
        // If LoRA adapter is configured, use a custom model name
        if let Some(ref lora_path) = self.lora_adapter_path {
            // Create a custom model name based on the LoRA adapter
            let adapter_name = lora_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("custom");
            format!("{}-{}", self.model.model_name, adapter_name)
        } else {
            self.model.model_name.clone()
        }
    }

    async fn ensure_lora_model(&self) -> Result<String, ProviderError> {
        if let Some(ref lora_path) = self.lora_adapter_path {
            let custom_model_name = self.get_model_name();
            
            // Check if custom model already exists
            let client = reqwest::Client::new();
            let list_url = format!("{}/api/tags", self.ollama_host);
            
            match client.get(&list_url).send().await {
                Ok(resp) => {
                    if let Ok(body) = resp.text().await {
                        if body.contains(&custom_model_name) {
                            tracing::info!("LoRA model {} already exists", custom_model_name);
                            return Ok(custom_model_name);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to check existing models: {}", e);
                }
            }

            // Create Modelfile with LoRA adapter
            let modelfile = format!(
                "FROM {}\nADAPTER {}",
                self.model.model_name,
                lora_path.display()
            );

            tracing::info!("Creating LoRA model: {}", custom_model_name);
            tracing::info!("Modelfile:\n{}", modelfile);

            // Create the model via Ollama API
            let create_url = format!("{}/api/create", self.ollama_host);
            let create_req = serde_json::json!({
                "name": custom_model_name,
                "modelfile": modelfile,
            });

            client
                .post(&create_url)
                .json(&create_req)
                .send()
                .await
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to create LoRA model: {}", e)))?;

            tracing::info!("LoRA model created successfully: {}", custom_model_name);
            Ok(custom_model_name)
        } else {
            Ok(self.model.model_name.clone())
        }
    }
}

#[async_trait]
impl Provider for NativeModelProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "native_model",
            "Native Model",
            "Locally hosted models with full Metal GPU support and LoRA fine-tuning",
            NATIVE_DEFAULT_MODEL,
            NATIVE_KNOWN_MODELS.to_vec(),
            NATIVE_DOC_URL,
            vec![
                ConfigKey::new(
                    "OLLAMA_HOST",
                    false,
                    false,
                    Some("http://localhost:11434"),
                ),
                ConfigKey::new("NATIVE_LORA_ADAPTER_PATH", false, false, None),
                ConfigKey::new("NATIVE_MAX_TOKENS", false, false, Some("512")),
                ConfigKey::new("NATIVE_TEMPERATURE", false, false, Some("0.7")),
                ConfigKey::new("NATIVE_TOP_P", false, false, Some("0.9")),
                ConfigKey::new("NATIVE_TOP_K", false, false, Some("40")),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn complete_with_model(
        &self,
        _model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        tracing::info!("Native Model Provider: request with {} messages", messages.len());

        let prompt = self.build_prompt(system, messages);
        let model_name = self.ensure_lora_model().await?;

        // Get config
        let config = crate::config::Config::global();
        let max_tokens: i32 = config.get_param("NATIVE_MAX_TOKENS").unwrap_or(512);
        let temperature: f32 = config.get_param("NATIVE_TEMPERATURE").unwrap_or(0.7);
        let top_p: f32 = config.get_param("NATIVE_TOP_P").unwrap_or(0.9);
        let top_k: i32 = config.get_param("NATIVE_TOP_K").unwrap_or(40);

        let request = OllamaRequest {
            model: model_name,
            prompt: prompt.clone(),
            stream: false,
            options: OllamaOptions {
                temperature,
                top_p,
                top_k,
                num_predict: max_tokens,
            },
        };

        let client = reqwest::Client::new();
        let url = format!("{}/api/generate", self.ollama_host);

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::ExecutionError(format!("Ollama request failed: {}", e)))?;

        let ollama_resp: OllamaResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to parse response: {}", e)))?;

        let response_message = Message::assistant().with_text(ollama_resp.response.clone());
        let usage = Usage {
            input_tokens: Some((prompt.len() / 4) as i32),
            output_tokens: Some((ollama_resp.response.len() / 4) as i32),
            total_tokens: None,
        };
        let provider_usage = ProviderUsage::new(self.model.model_name.clone(), usage);

        Ok((response_message, provider_usage))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        tracing::info!("Native Model Provider STREAM: request with {} messages", messages.len());

        let prompt = self.build_prompt(system, messages);
        let model_name = self.ensure_lora_model().await?;
        let provider_model_name = self.model.model_name.clone();

        // Get config
        let config = crate::config::Config::global();
        let max_tokens: i32 = config.get_param("NATIVE_MAX_TOKENS").unwrap_or(512);
        let temperature: f32 = config.get_param("NATIVE_TEMPERATURE").unwrap_or(0.7);
        let top_p: f32 = config.get_param("NATIVE_TOP_P").unwrap_or(0.9);
        let top_k: i32 = config.get_param("NATIVE_TOP_K").unwrap_or(40);

        let request = OllamaRequest {
            model: model_name,
            prompt,
            stream: true,
            options: OllamaOptions {
                temperature,
                top_p,
                top_k,
                num_predict: max_tokens,
            },
        };

        let (tx, rx) = mpsc::unbounded_channel::<
            Result<(Option<Message>, Option<ProviderUsage>), ProviderError>,
        >();

        let ollama_host = self.ollama_host.clone();
        
        // Generate a unique message ID for this streaming response
        let message_id = format!("native-{}", uuid::Uuid::new_v4());

        // Spawn task to handle streaming
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let url = format!("{}/api/generate", ollama_host);

            match client.post(&url).json(&request).send().await {
                Ok(response) => {
                    let mut stream = response.bytes_stream();
                    let mut total_tokens = 0;

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                // Parse each line as JSON
                                let text = String::from_utf8_lossy(&chunk);
                                for line in text.lines() {
                                    if line.trim().is_empty() {
                                        continue;
                                    }

                                    match serde_json::from_str::<OllamaResponse>(line) {
                                        Ok(ollama_resp) => {
                                            if !ollama_resp.response.is_empty() {
                                                total_tokens += 1;
                                                
                                                // Send only the delta (new text), not accumulated
                                                // Include the message ID so the frontend can accumulate deltas
                                                let chunk = Message::assistant()
                                                    .with_text(ollama_resp.response)
                                                    .with_id(message_id.clone());
                                                let usage = Usage {
                                                    input_tokens: None,
                                                    output_tokens: Some(1),
                                                    total_tokens: None,
                                                };
                                                let provider_usage = ProviderUsage::new(provider_model_name.clone(), usage);

                                                if tx.send(Ok((Some(chunk), Some(provider_usage)))).is_err() {
                                                    tracing::error!("Failed to send chunk: channel closed");
                                                    return;
                                                }
                                            }

                                            if ollama_resp.done {
                                                tracing::info!("Streaming completed with {} tokens", total_tokens);
                                                return;
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!("Failed to parse streaming response: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(Err(ProviderError::ExecutionError(format!("Stream error: {}", e))));
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(ProviderError::ExecutionError(format!("Failed to start stream: {}", e))));
                }
            }
        });

        let stream = UnboundedReceiverStream::new(rx).map(|item| item);
        Ok(Box::pin(stream))
    }
}

impl NativeModelProvider {
    /// Load a PEFT/Axolotl LoRA adapter and prepare to apply during inference
    /// This creates a new Ollama model with the adapter applied
    pub fn load_adapter(&mut self, adapter_path: &str) -> Result<()> {
        let p = PathBuf::from(adapter_path);
        if !p.exists() {
            return Err(anyhow::anyhow!("LoRA adapter not found: {}", adapter_path));
        }
        self.lora_adapter_path = Some(p);
        tracing::info!("LoRA adapter configured for native model: {}", adapter_path);
        tracing::info!("Adapter will be applied via Ollama Modelfile on next inference");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_provider_creation() {
        let model_config = ModelConfig::new("qwen2.5:7b-instruct").unwrap();
        let provider = NativeModelProvider::from_env(model_config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_metadata() {
        let metadata = NativeModelProvider::metadata();
        assert_eq!(metadata.name, "native_model");
        assert_eq!(metadata.display_name, "Native Model");
    }
}
