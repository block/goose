use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{get_model, handle_response_openai_compat, handle_status_openai_compat};
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::formats::openai::{
    create_request, get_usage, response_to_message, response_to_streaming_message,
};
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use rmcp::model::{Role, Tool};
use serde_json::{json, Value};
use std::io;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tokio::pin;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;
use url::Url;

pub const EMBEDDED_HOST: &str = "127.0.0.1";
pub const EMBEDDED_DEFAULT_PORT: u16 = 8080;
pub const EMBEDDED_DEFAULT_CTX_SIZE: u32 = 8192;
pub const EMBEDDED_TIMEOUT: u64 = 600; // seconds
pub const EMBEDDED_STARTUP_TIMEOUT: u64 = 30; // seconds to wait for server to start
pub const EMBEDDED_DOC_URL: &str =
    "https://github.com/ggerganov/llama.cpp/blob/master/examples/server/README.md";

/// Maximum size for tool output (4KB) to prevent context overflow with small models
const MAX_TOOL_OUTPUT_SIZE: usize = 4096;

/// System prompt for tool emulation mode (when model doesn't support native tool calling)
const EMULATION_SYSTEM_PROMPT: &str = r#"You are Goose, a general-purpose AI agent. Your goal is to analyze and solve problems by writing code.

# Tool Call Format

When you need to execute a tool, write ONLY the JSON tool call on a new line:

{"tool": "tool_name", "args": {"param": "value"}}

The tool will execute immediately and you'll receive the result (success or error) to continue with.

# Available Tools

- **shell**: Execute shell commands
  - Format: {"tool": "shell", "args": {"command": "your_command_here"}}
  - Example: {"tool": "shell", "args": {"command": "ls ~/Downloads"}}

- **final_output**: Signal task completion with a detailed summary of work done
  - Format: {"tool": "final_output", "args": {"summary": "what_was_accomplished"}}

# Instructions

1. Analyze the request and break down into smaller tasks if appropriate
2. Execute ONE tool at a time
3. STOP when the original request was satisfied
4. Call the final_output tool when done

# Response Guidelines

- Use Markdown formatting for all responses except tool calls.
- Whenever taking actions, use the pronoun 'I'
"#;

/// Platform-specific defaults
#[derive(Debug, Clone)]
struct PlatformDefaults {
    gpu_layers: u32,
    batch_size: u32,
    threads: u32,
}

/// Detect platform and return intelligent defaults
fn detect_platform_defaults() -> PlatformDefaults {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_count = sys.cpus().len() as u32;
    let total_memory_gb = sys.total_memory() / (1024 * 1024 * 1024);

    // Detect platform
    let os = std::env::consts::OS;

    tracing::debug!(
        "Platform detection: OS={}, CPUs={}, Memory={}GB",
        os,
        cpu_count,
        total_memory_gb
    );

    match os {
        "macos" => {
            // Check if Apple Silicon (M1/M2/M3) by checking for arm64
            let is_apple_silicon = std::env::consts::ARCH == "aarch64";

            if is_apple_silicon {
                // Apple Silicon with unified memory - can use more GPU layers
                // Estimate based on available memory
                let gpu_layers = if total_memory_gb >= 64 {
                    80 // High memory system
                } else if total_memory_gb >= 32 {
                    60 // Medium memory system
                } else {
                    40 // Lower memory system
                };

                tracing::info!(
                    "Detected Apple Silicon with {}GB RAM, using {} GPU layers",
                    total_memory_gb,
                    gpu_layers
                );

                PlatformDefaults {
                    gpu_layers,
                    batch_size: 512,
                    threads: cpu_count,
                }
            } else {
                // Intel Mac - likely CPU only
                tracing::info!("Detected Intel Mac, using CPU-only mode");
                PlatformDefaults {
                    gpu_layers: 0,
                    batch_size: 256,
                    threads: cpu_count,
                }
            }
        }
        "linux" => {
            // Try to detect NVIDIA GPU by checking if nvidia-smi exists
            let has_nvidia = std::process::Command::new("nvidia-smi").output().is_ok();

            if has_nvidia {
                // Assume modern NVIDIA GPU with 8-24GB VRAM
                // Conservative estimate: 1 layer ≈ 200-300MB for a 7B model
                let gpu_layers = 60; // Good for 12-16GB VRAM

                tracing::info!(
                    "Detected Linux with NVIDIA GPU, using {} GPU layers",
                    gpu_layers
                );

                PlatformDefaults {
                    gpu_layers,
                    batch_size: 512,
                    threads: cpu_count,
                }
            } else {
                // CPU-only Linux
                tracing::info!("Detected Linux without NVIDIA GPU, using CPU-only mode");
                PlatformDefaults {
                    gpu_layers: 0,
                    batch_size: 256,
                    threads: cpu_count,
                }
            }
        }
        "windows" => {
            // Windows - check for NVIDIA
            let has_nvidia = std::process::Command::new("nvidia-smi.exe")
                .output()
                .is_ok();

            if has_nvidia {
                tracing::info!("Detected Windows with NVIDIA GPU, using 60 GPU layers");
                PlatformDefaults {
                    gpu_layers: 60,
                    batch_size: 512, // Slightly more conservative on Windows
                    threads: cpu_count,
                }
            } else {
                tracing::info!("Detected Windows without NVIDIA GPU, using CPU-only mode");
                PlatformDefaults {
                    gpu_layers: 0,
                    batch_size: 256,
                    threads: cpu_count,
                }
            }
        }
        _ => {
            // Unknown platform - conservative defaults
            tracing::warn!("Unknown platform, using conservative CPU-only defaults");
            PlatformDefaults {
                gpu_layers: 0,
                batch_size: 256,
                threads: cpu_count.min(8),
            }
        }
    }
}

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
    /// Whether the model supports native tool calling (None = not yet detected)
    #[serde(skip)]
    tool_calling_support: Arc<Mutex<Option<bool>>>,
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

        // Detect platform-specific defaults
        let platform_defaults = detect_platform_defaults();

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
            .unwrap_or(platform_defaults.gpu_layers);
        let batch_size: u32 = config
            .get_param("EMBEDDED_BATCH_SIZE")
            .unwrap_or(platform_defaults.batch_size);
        let threads: u32 = config
            .get_param("EMBEDDED_THREADS")
            .unwrap_or(platform_defaults.threads);
        let timeout: Duration = Duration::from_secs(
            config
                .get_param("EMBEDDED_TIMEOUT")
                .unwrap_or(EMBEDDED_TIMEOUT),
        );

        tracing::info!(
            "Embedded provider configuration: gpu_layers={}, batch_size={}, threads={}, ctx_size={}",
            gpu_layers,
            batch_size,
            threads,
            ctx_size
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
            tool_calling_support: Arc::new(Mutex::new(None)),
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

        tracing::info!(
            "Waiting for llama-server to load model on port {}...",
            self.port
        );

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for llama-server to start and load model"
                ));
            }

            // Try to make a small test request to verify the model is loaded
            // The health endpoint might return OK before the model is ready
            let test_payload = json!({
                "model": "embedded",
                "messages": [{"role": "user", "content": "test"}],
                "max_tokens": 1,
                "temperature": 0.0
            });

            match self
                .api_client
                .response_post("v1/chat/completions", &test_payload)
                .await
            {
                Ok(response) => {
                    // Check if we got a valid response (not a 503 loading error)
                    match handle_response_openai_compat(response).await {
                        Ok(_) => {
                            tracing::info!(
                                "llama-server is ready and model loaded on port {}",
                                self.port
                            );
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("{:?}", e);
                            if error_msg.contains("503") || error_msg.contains("Loading model") {
                                tracing::debug!("Model still loading, waiting...");
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            } else {
                                // Some other error - might indicate server issues
                                tracing::debug!(
                                    "Server error during readiness check: {}, retrying...",
                                    e
                                );
                                tokio::time::sleep(Duration::from_millis(500)).await;
                            }
                        }
                    }
                }
                Err(_) => {
                    // Server not responding yet
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

    /// Detect if the model supports native tool calling
    async fn detect_tool_support(&self) -> bool {
        // Create a simple test tool
        let test_tool = Tool::new(
            "test".to_string(),
            "test tool".to_string(),
            serde_json::Map::new(),
        );

        let test_message = Message::user().with_text("test");

        let test_payload = match create_request(
            &self.model,
            "test",
            &[test_message],
            &[test_tool],
            &super::utils::ImageFormat::OpenAi,
        ) {
            Ok(payload) => payload,
            Err(_) => return false,
        };

        match self.post(&test_payload).await {
            Ok(response) => {
                // Check if response has tool_calls in the expected OpenAI format
                response
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("tool_calls"))
                    .is_some()
            }
            Err(_) => false,
        }
    }

    /// Get or detect tool calling support
    async fn get_tool_support(&self) -> bool {
        // Check if forced emulation is enabled FIRST (before cache)
        let config = crate::config::Config::global();
        let force_emulation = config
            .get_param::<bool>("EMBEDDED_FORCE_TOOL_EMULATION")
            .unwrap_or(false);
        
        eprintln!("🔍 get_tool_support: EMBEDDED_FORCE_TOOL_EMULATION = {}", force_emulation);
        
        if force_emulation {
            eprintln!("✅ FORCING EMULATION MODE");
            tracing::info!("Tool emulation forced via EMBEDDED_FORCE_TOOL_EMULATION");
            return false;
        }

        let mut support = self.tool_calling_support.lock().await;

        eprintln!("🔍 get_tool_support: cached value = {:?}", *support);

        if let Some(cached) = *support {
            eprintln!("🔍 get_tool_support: returning cached = {}", cached);
            return cached;
        }

        // Detect support
        tracing::debug!("Detecting tool calling support...");
        let detected = self.detect_tool_support().await;

        if detected {
            tracing::info!("Model supports native tool calling");
            println!("REMOVE ME: model supports native tool calling");
        } else {
            tracing::info!("Model does not support native tool calling, will use emulation mode");
            println!("REMOVE ME: model does not support native tool calling");
        }

        *support = Some(detected);
        detected
    }
}

/// Tool executor for emulation mode (when model doesn't support native tool calling)
struct ToolExecutor;

impl ToolExecutor {
    /// Parse and execute JSON tool calls from the response text
    async fn execute_tool_calls(text: &str) -> String {
        println!("REMOVE ME, doing a emulated tool call...");
        let mut result = String::new();
        let mut remaining = text;

        // Look for JSON tool calls in the format: {"tool": "name", "args": {...}}
        while let Some(start_idx) = remaining.find(r#"{"tool":"#) {
            // Add everything before the JSON
            result.push_str(&remaining[..start_idx]);

            // Find the end of the JSON object
            let json_start = &remaining[start_idx..];
            if let Some(end_idx) = Self::find_json_end(json_start) {
                let json_str = &json_start[..=end_idx];

                // Try to parse and execute the tool call
                match serde_json::from_str::<Value>(json_str) {
                    Ok(json) => {
                        // Add the original JSON to result
                        result.push_str(json_str);

                        // Execute the tool call
                        if let Some(tool_result) = Self::execute_tool_call(&json).await {
                            result.push_str(&format!("\n\n{}\n", tool_result));
                        }
                    }
                    Err(_) => {
                        // Not valid JSON, just add it as-is
                        result.push_str(&json_start[..=end_idx]);
                    }
                }

                remaining = &json_start[end_idx + 1..];
            } else {
                // Couldn't find end of JSON, add the rest as-is
                result.push_str(remaining);
                break;
            }
        }

        // Add any remaining text
        result.push_str(remaining);

        if result.is_empty() {
            text.to_string()
        } else {
            result
        }
    }

    /// Find the end of a JSON object in text
    fn find_json_end(text: &str) -> Option<usize> {
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, ch) in text.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }

        None
    }

    /// Execute a single tool call
    async fn execute_tool_call(json: &Value) -> Option<String> {
        let tool_name = json.get("tool")?.as_str()?;
        let args = json.get("args")?;

        match tool_name {
            "shell" => {
                if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                    tracing::info!("Executing shell command: {}", command);

                    match Command::new("sh").arg("-c").arg(command).output() {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);

                            // Truncate if output is too large
                            let truncate_if_large = |s: &str| -> String {
                                if s.len() > MAX_TOOL_OUTPUT_SIZE {
                                    format!(
                                        "{}... [truncated, {} bytes total]",
                                        &s[..MAX_TOOL_OUTPUT_SIZE],
                                        s.len()
                                    )
                                } else {
                                    s.to_string()
                                }
                            };

                            let stdout_display = truncate_if_large(&stdout);
                            let stderr_display = truncate_if_large(&stderr);

                            if output.status.success() {
                                Some(format!(
                                    "Command executed successfully:\n```\n{}\n```",
                                    stdout_display
                                ))
                            } else {
                                Some(format!(
                                    "Command failed with exit code {}:\nstdout:\n```\n{}\n```\nstderr:\n```\n{}\n```",
                                    output.status.code().unwrap_or(-1),
                                    stdout_display,
                                    stderr_display
                                ))
                            }
                        }
                        Err(e) => Some(format!("Failed to execute command: {}", e)),
                    }
                } else {
                    Some("Error: shell tool requires 'command' argument".to_string())
                }
            }
            "final_output" => {
                // final_output is a no-op, just signals completion
                tracing::info!("Task completed");
                None
            }
            _ => Some(format!("Error: Unknown tool '{}'", tool_name)),
        }
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
            "Local GGUF models via llama-server (auto-detects platform, looks in ~/.models)",
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
                ConfigKey::new("EMBEDDED_GPU_LAYERS", false, false, Some("auto-detected")),
                ConfigKey::new("EMBEDDED_BATCH_SIZE", false, false, Some("auto-detected")),
                ConfigKey::new("EMBEDDED_THREADS", false, false, Some("auto-detected")),
                ConfigKey::new(
                    "EMBEDDED_TIMEOUT",
                    false,
                    false,
                    Some(&EMBEDDED_TIMEOUT.to_string()),
                ),
                ConfigKey::new("EMBEDDED_FORCE_TOOL_EMULATION", false, false, Some("false")),
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

        eprintln!("🔍 DIAGNOSTIC: tools.len()={}, filtered_tools.len()={}, goose_mode={}", 
                  tools.len(), filtered_tools.len(), goose_mode);

        // Check if we should use emulation mode
        let use_emulation = !filtered_tools.is_empty() && !self.get_tool_support().await;

        eprintln!("🔍 DIAGNOSTIC: use_emulation={}", use_emulation);

        if use_emulation {
            eprintln!("✅ USING EMULATION MODE");
            tracing::info!("Using tool emulation mode");

            // Use emulation prompt as system parameter (not user message) to avoid Jinja issues
            let emulation_payload = create_request(
                &self.model,
                EMULATION_SYSTEM_PROMPT, // Emulation prompt as system
                messages,
                &[], // No tools parameter
                &super::utils::ImageFormat::OpenAi,
            )?;

            let response = self
                .with_retry(|| async {
                    let payload_clone = emulation_payload.clone();
                    self.post(&payload_clone).await
                })
                .await?;

            let message = response_to_message(&response)?;

            // Extract text from message and execute tool calls
            let text = message.as_concat_text();
            let augmented_text = ToolExecutor::execute_tool_calls(&text).await;

            // Create new message with augmented text
            let augmented_message = Message::new(
                Role::Assistant,
                chrono::Utc::now().timestamp(),
                vec![MessageContent::text(augmented_text)],
            );

            let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
                tracing::debug!("Failed to get usage data");
                Usage::default()
            });

            let response_model = get_model(&response);
            super::utils::emit_debug_trace(model_config, &emulation_payload, &response, &usage);
            Ok((augmented_message, ProviderUsage::new(response_model, usage)))
        } else {
            eprintln!("❌ USING NATIVE TOOL CALLING (not emulation)");
            // Use native tool calling (current path)
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
        eprintln!("🔍 STREAM: tools.len()={}", tools.len());
        
        self.ensure_server_running()
            .await
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to start server: {}", e)))?;

        let config = crate::config::Config::global();
        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());
        let filtered_tools = if goose_mode == "chat" { &[] } else { tools };
        
        // Check if we should use emulation mode
        let use_emulation = !filtered_tools.is_empty() && !self.get_tool_support().await;
        
        eprintln!("🔍 STREAM: use_emulation={}", use_emulation);
        
        // Determine what to send based on emulation mode
        let (final_system, final_messages, final_tools) = if use_emulation {
            eprintln!("✅ STREAM: USING EMULATION MODE");
            // Use emulation prompt as system parameter to avoid Jinja template issues
            (EMULATION_SYSTEM_PROMPT, messages.to_vec(), vec![])
        } else {
            eprintln!("❌ STREAM: USING NATIVE TOOL CALLING");
            (system, messages.to_vec(), filtered_tools.to_vec())
        };

        let mut payload = create_request(
            &self.model,
            final_system,
            &final_messages,
            &final_tools,
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
            
            let mut collected_text = String::new();
            let mut last_usage = None;
            
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                
                if use_emulation {
                    // In emulation mode, collect text for tool execution later
                    if let Some(ref msg) = message {
                        super::utils::emit_debug_trace(&model_config, &payload, msg, &usage.as_ref().map(|f| f.usage).unwrap_or_default());
                        collected_text.push_str(&msg.as_concat_text());
                    }
                    last_usage = usage;
                } else {
                    // In native mode, yield directly
                    if let Some(ref msg) = message {
                        super::utils::emit_debug_trace(&model_config, &payload, msg, &usage.as_ref().map(|f| f.usage).unwrap_or_default());
                    }
                    yield (message, usage);
                }
            }
            
            // If in emulation mode, execute tools and yield augmented message
            if use_emulation && !collected_text.is_empty() {
                eprintln!("🔧 EXECUTING TOOL CALLS IN EMULATION MODE");
                let augmented_text = ToolExecutor::execute_tool_calls(&collected_text).await;
                let augmented_message = Message::new(
                    Role::Assistant,
                    chrono::Utc::now().timestamp(),
                    vec![MessageContent::text(augmented_text)],
                );
                yield (Some(augmented_message), last_usage);
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
