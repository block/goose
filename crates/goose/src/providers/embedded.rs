use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{get_model, handle_response_openai_compat, handle_status_openai_compat};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::openai::{
    create_request, get_usage, response_to_message, response_to_streaming_message,
};
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use rmcp::model::Tool;
use serde_json::{json, Value};
use std::io;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::pin;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;
use url::Url;

pub const EMBEDDED_HOST: &str = "127.0.0.1";
pub const EMBEDDED_DEFAULT_PORT: u16 = 8080;
pub const EMBEDDED_DEFAULT_CTX_SIZE: u32 = 8192;
pub const EMBEDDED_DEFAULT_GPU_LAYERS: u32 = 60;
pub const EMBEDDED_DEFAULT_BATCH_SIZE: u32 = 512;
pub const EMBEDDED_DEFAULT_THREADS: u32 = 8;
pub const EMBEDDED_TIMEOUT: u64 = 600; // seconds
pub const EMBEDDED_STARTUP_TIMEOUT: u64 = 30; // seconds to wait for server to start
pub const EMBEDDED_DOC_URL: &str =
    "https://github.com/ggerganov/llama.cpp/blob/master/examples/server/README.md";

/// Manages a local llama-server process
struct ServerProcess {
    child: Option<Child>,
    port: u16,
}

impl ServerProcess {
    fn new(port: u16) -> Self {
        Self { child: None, port }
    }

    fn start(
        &mut self,
        model_path: &str,
        host: &str,
        ctx_size: u32,
        gpu_layers: u32,
        batch_size: u32,
        threads: u32,
    ) -> Result<()> {
        if self.child.is_some() {
            return Ok(()); // Already running
        }

        tracing::info!(
            "Starting llama-server with model: {} on {}:{}",
            model_path,
            host,
            self.port
        );

        let child = Command::new("llama-server")
            .arg("--model")
            .arg(model_path)
            .arg("--host")
            .arg(host)
            .arg("--port")
            .arg(self.port.to_string())
            .arg("--ctx-size")
            .arg(ctx_size.to_string())
            .arg("--n-gpu-layers")
            .arg(gpu_layers.to_string())
            .arg("--batch-size")
            .arg(batch_size.to_string())
            .arg("--threads")
            .arg(threads.to_string())
            .arg("--jinja")
            .arg("--json-schema")
            .arg("{}")
            .arg("--verbose")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start llama-server: {}", e))?;

        self.child = Some(child);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            tracing::info!("Stopping llama-server on port {}", self.port);
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn is_running(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(serde::Serialize)]
pub struct EmbeddedProvider {
    #[serde(skip)]
    api_client: ApiClient,
    #[serde(skip)]
    server_process: Arc<Mutex<ServerProcess>>,
    model: ModelConfig,
    model_path: String,
    host: String,
    port: u16,
    ctx_size: u32,
    gpu_layers: u32,
    batch_size: u32,
    threads: u32,
    supports_streaming: bool,
}

impl EmbeddedProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        // Determine the model path
        // Priority: EMBEDDED_MODEL_PATH > model_name from GOOSE_MODEL in ~/.models
        let model_path: String = if let Ok(path) = config.get_param::<String>("EMBEDDED_MODEL_PATH")
        {
            // If EMBEDDED_MODEL_PATH is set, use it as-is
            path
        } else {
            // Otherwise, look for the model in ~/.models directory
            let model_name = &model.model_name;

            // Expand home directory
            let home_dir = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
            let models_dir = home_dir.join(".models");

            // If model_name already ends with .gguf, use it directly
            // Otherwise, try to find a matching file
            let model_file = if model_name.ends_with(".gguf") {
                models_dir.join(model_name)
            } else {
                // Try with .gguf extension
                let with_extension = models_dir.join(format!("{}.gguf", model_name));
                if with_extension.exists() {
                    with_extension
                } else {
                    // Fall back to the name as-is
                    models_dir.join(model_name)
                }
            };

            // Verify the file exists
            if !model_file.exists() {
                return Err(anyhow::anyhow!(
                    "Model file not found: {}. Please ensure the GGUF file exists in ~/.models/ or set EMBEDDED_MODEL_PATH",
                    model_file.display()
                ));
            }

            model_file.to_string_lossy().to_string()
        };

        let host: String = config
            .get_param("EMBEDDED_HOST")
            .unwrap_or_else(|_| EMBEDDED_HOST.to_string());
        let port: u16 = config
            .get_param("EMBEDDED_PORT")
            .unwrap_or(EMBEDDED_DEFAULT_PORT);
        let ctx_size: u32 = config
            .get_param("EMBEDDED_CTX_SIZE")
            .unwrap_or(EMBEDDED_DEFAULT_CTX_SIZE);
        let gpu_layers: u32 = config
            .get_param("EMBEDDED_GPU_LAYERS")
            .unwrap_or(EMBEDDED_DEFAULT_GPU_LAYERS);
        let batch_size: u32 = config
            .get_param("EMBEDDED_BATCH_SIZE")
            .unwrap_or(EMBEDDED_DEFAULT_BATCH_SIZE);
        let threads: u32 = config
            .get_param("EMBEDDED_THREADS")
            .unwrap_or(EMBEDDED_DEFAULT_THREADS);
        let timeout: Duration = Duration::from_secs(
            config
                .get_param("EMBEDDED_TIMEOUT")
                .unwrap_or(EMBEDDED_TIMEOUT),
        );

        let base_url = format!("http://{}:{}", host, port);
        let url = Url::parse(&base_url).map_err(|e| anyhow::anyhow!("Invalid base URL: {}", e))?;

        // No authentication needed for local server
        let auth = AuthMethod::Custom(Box::new(NoAuth));
        let api_client = ApiClient::with_timeout(url.to_string(), auth, timeout)?;

        let server_process = Arc::new(Mutex::new(ServerProcess::new(port)));

        let provider = Self {
            api_client,
            server_process,
            model,
            model_path,
            host,
            port,
            ctx_size,
            gpu_layers,
            batch_size,
            threads,
            supports_streaming: true,
        };

        // Start the server process
        provider.ensure_server_running().await?;

        Ok(provider)
    }

    async fn ensure_server_running(&self) -> Result<()> {
        let mut process = self.server_process.lock().await;

        if !process.is_running() {
            process.start(
                &self.model_path,
                &self.host,
                self.ctx_size,
                self.gpu_layers,
                self.batch_size,
                self.threads,
            )?;

            // Wait for server to be ready
            drop(process); // Release lock while waiting
            self.wait_for_server_ready().await?;
        }

        Ok(())
    }

    async fn wait_for_server_ready(&self) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(EMBEDDED_STARTUP_TIMEOUT);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("Timeout waiting for llama-server to start"));
            }

            // Try to connect to the health endpoint
            match self.api_client.response_get("health").await {
                Ok(_) => {
                    tracing::info!("llama-server is ready on port {}", self.port);
                    return Ok(());
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        self.ensure_server_running()
            .await
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to start server: {}", e)))?;

        let response = self
            .api_client
            .response_post("v1/chat/completions", payload)
            .await?;
        handle_response_openai_compat(response).await
    }
}

// No authentication provider
struct NoAuth;

#[async_trait]
impl super::api_client::AuthProvider for NoAuth {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        Ok(("X-No-Auth".to_string(), "true".to_string()))
    }
}

#[async_trait]
impl Provider for EmbeddedProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "embedded",
            "Embedded",
            "Local GGUF models via llama-server (looks in ~/.models by default)",
            "embedded",
            vec!["embedded"],
            EMBEDDED_DOC_URL,
            vec![
                ConfigKey::new("EMBEDDED_MODEL_PATH", false, false, None),
                ConfigKey::new("EMBEDDED_HOST", false, false, Some(EMBEDDED_HOST)),
                ConfigKey::new(
                    "EMBEDDED_PORT",
                    false,
                    false,
                    Some(&EMBEDDED_DEFAULT_PORT.to_string()),
                ),
                ConfigKey::new(
                    "EMBEDDED_CTX_SIZE",
                    false,
                    false,
                    Some(&EMBEDDED_DEFAULT_CTX_SIZE.to_string()),
                ),
                ConfigKey::new(
                    "EMBEDDED_GPU_LAYERS",
                    false,
                    false,
                    Some(&EMBEDDED_DEFAULT_GPU_LAYERS.to_string()),
                ),
                ConfigKey::new(
                    "EMBEDDED_BATCH_SIZE",
                    false,
                    false,
                    Some(&EMBEDDED_DEFAULT_BATCH_SIZE.to_string()),
                ),
                ConfigKey::new(
                    "EMBEDDED_THREADS",
                    false,
                    false,
                    Some(&EMBEDDED_DEFAULT_THREADS.to_string()),
                ),
                ConfigKey::new(
                    "EMBEDDED_TIMEOUT",
                    false,
                    false,
                    Some(&EMBEDDED_TIMEOUT.to_string()),
                ),
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

        let message = response_to_message(&response)?;

        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });

        let response_model = get_model(&response);
        super::utils::emit_debug_trace(model_config, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(response_model, usage)))
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
        self.ensure_server_running()
            .await
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to start server: {}", e)))?;

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
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);
            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                super::utils::emit_debug_trace(&model_config, &payload, &message, &usage.as_ref().map(|f| f.usage).unwrap_or_default());
                yield (message, usage);
            }
        }))
    }
}

impl Drop for EmbeddedProvider {
    fn drop(&mut self) {
        if let Ok(mut process) = self.server_process.try_lock() {
            process.stop();
        }
    }
}
