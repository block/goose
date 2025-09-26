use anyhow::Result;
use async_trait::async_trait;
use llama_cpp::{
    standard_sampler::{SamplerStage, StandardSampler},
    LlamaModel, LlamaParams, LlamaSession, SessionParams,
};
use rmcp::model::{Role, Tool};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use super::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::emit_debug_trace;
use crate::conversation::message::{Message, MessageContent};
use crate::impl_provider_default;
use crate::model::ModelConfig;

pub const EMBEDDED_DEFAULT_MODEL: &str = "qwen2.5-7b-instruct";
pub const EMBEDDED_KNOWN_MODELS: &[&str] = &[
    "qwen2.5-7b-instruct",
    "llama-7b",
    "codellama-7b",
    "mistral-7b",
];



const DEFAULT_MODEL_URL: &str =
    "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q3_k_m.gguf";
const DEFAULT_MODEL_SIZE_MB: u64 = 3631;

pub struct EmbeddedProvider {
    session: Arc<Mutex<LlamaSession>>,
    model: ModelConfig,
    model_type: String,
    max_tokens: u32,
    temperature: f32,
    context_length: u32,
    enable_tools: bool,
}

// JSON tool call executor - like g3
struct ToolExecutor;

impl ToolExecutor {
    /// Parse and execute JSON tool calls from the response
    async fn execute_tool_calls(text: &str) -> String {
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
                match serde_json::from_str::<serde_json::Value>(json_str) {
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
    
    /// Find the end of a JSON object
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
    async fn execute_tool_call(json: &serde_json::Value) -> Option<String> {
        let tool_name = json.get("tool")?.as_str()?;
        let args = json.get("args")?;
        
        match tool_name {
            "shell" => {
                if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                    match Command::new("sh").arg("-c").arg(command).output() {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            
                            if output.status.success() {
                                if !stdout.is_empty() {
                                    Some(format!("✓ Command executed successfully:\n{}", stdout))
                                } else {
                                    Some("✓ Command executed successfully".to_string())
                                }
                            } else {
                                Some(format!("✗ Command failed:\n{}", 
                                    if !stderr.is_empty() { stderr.to_string() } else { stdout.to_string() }))
                            }
                        }
                        Err(e) => Some(format!("✗ Failed to execute command: {}", e)),
                    }
                } else {
                    Some("✗ Missing 'command' parameter for shell tool".to_string())
                }
            }
            "final_output" => {
                // For final_output, we don't execute anything, just acknowledge
                None
            }
            _ => {
                Some(format!("✗ Unknown tool: {}", tool_name))
            }
        }
    }
}

impl_provider_default!(EmbeddedProvider);

impl EmbeddedProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        
        // Get configuration parameters
        let model_path: String = config
            .get_param("EMBEDDED_MODEL_PATH")
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.cache/goose/models/qwen2.5-7b-instruct-q3_k_m.gguf", home)
            });
        
        let model_type: String = config
            .get_param("EMBEDDED_MODEL_TYPE")
            .unwrap_or_else(|_| "qwen".to_string());
        
        let context_length: u32 = config
            .get_param("EMBEDDED_CONTEXT_LENGTH")
            .unwrap_or(32768);  // Match g3's default
        
        let max_tokens: u32 = config
            .get_param("EMBEDDED_MAX_TOKENS")
            .unwrap_or(2048);
        
        let temperature: f32 = config
            .get_param("EMBEDDED_TEMPERATURE")
            .unwrap_or(0.1);
        
        let gpu_layers: u32 = config
            .get_param("EMBEDDED_GPU_LAYERS")
            .unwrap_or(32);  // Use GPU by default like g3!
        
        let threads: u32 = config
            .get_param("EMBEDDED_THREADS")
            .unwrap_or(8);  // Match g3's thread count
        
        // Check if tools should be enabled (similar to claude-cli's permission mode)
        let goose_mode: String = config
            .get_param("GOOSE_MODE")
            .unwrap_or_else(|_| "manual".to_string());
        let enable_tools = goose_mode == "auto";
        
        // Initialize the model
        Self::initialize_model(
            model,
            model_path,
            model_type,
            context_length,
            max_tokens,
            temperature,
            gpu_layers,
            threads,
            enable_tools,
        )
    }
    
    fn initialize_model(
        model: ModelConfig,
        model_path: String,
        model_type: String,
        context_length: u32,
        max_tokens: u32,
        temperature: f32,
        gpu_layers: u32,
        threads: u32,
        enable_tools: bool,
    ) -> Result<Self> {
        info!("Loading embedded model from: {}", model_path);
        
        // Expand tilde in path
        let expanded_path = shellexpand::tilde(&model_path);
        let model_path_buf = PathBuf::from(expanded_path.as_ref());
        
        // If model doesn't exist and it's the default Qwen model, offer to download it
        if !model_path_buf.exists() {
            if model_path.contains("qwen2.5-7b-instruct") {
                info!("Model file not found. Attempting to download Qwen 2.5 7B model...");
                Self::download_default_model(&model_path_buf)?;
            } else {
                return Err(anyhow::anyhow!("Model file not found: {}", model_path_buf.display()));
            }
        }
        
        let model_path = model_path_buf.as_path();
        
        // Set up model parameters
        let mut params = LlamaParams::default();
        
        // Use Metal on macOS
        #[cfg(target_os = "macos")]
        {
            params.n_gpu_layers = gpu_layers;
            info!("Using Metal with {} GPU layers", gpu_layers);
        }
        
        // Use CUDA on Linux if available
        #[cfg(not(target_os = "macos"))]
        {
            params.n_gpu_layers = gpu_layers;
            if gpu_layers > 0 {
                info!("Using {} GPU layers", gpu_layers);
            }
        }
        
        info!("Using context length: {}", context_length);
        
        // Load the model
        info!("Loading model...");
        let llama_model = LlamaModel::load_from_file(model_path, params)
            .map_err(|e| anyhow::anyhow!("Failed to load model: {}", e))?;
        
        // Create session with parameters
        let mut session_params = SessionParams::default();
        session_params.n_ctx = context_length;
        session_params.n_threads = threads;
        
        let session = llama_model
            .create_session(session_params)
            .map_err(|e| anyhow::anyhow!("Failed to create session: {}", e))?;
        
        info!("Successfully loaded {} model", model_type);
        
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            model,
            model_type,
            max_tokens,
            temperature,
            context_length,
            enable_tools,
        })
    }
    
    /// Download the default Qwen model if it doesn't exist
    fn download_default_model(model_path: &Path) -> Result<()> {
        use std::fs;
        
        // Create the parent directory if it doesn't exist
        if let Some(parent) = model_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        info!("Downloading Qwen 2.5 7B model (Q3_K_M quantization, ~3.5GB)...");
        info!("This is a one-time download that may take several minutes.");
        info!("Downloading to: {}", model_path.display());
        
        // Use curl with progress bar for download
        let output = Command::new("curl")
            .args(&[
                "-L",  // Follow redirects
                "-#",  // Show progress bar
                "-f",  // Fail on HTTP errors
                "-o", model_path.to_str().unwrap(),
                DEFAULT_MODEL_URL,
            ])
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            if stderr.contains("command not found") || stderr.contains("not found") {
                error!("curl is not installed. Please install curl or manually download the model.");
                error!("Manual download: {}", DEFAULT_MODEL_URL);
                error!("Save to: {}", model_path.display());
                anyhow::bail!("curl not found - please install curl or download the model manually");
            }
            
            anyhow::bail!("Failed to download model: {}", stderr);
        }
        
        // Verify the file was created and has reasonable size
        let metadata = fs::metadata(model_path)?;
        let size_mb = metadata.len() / (1024 * 1024);
        
        if size_mb < DEFAULT_MODEL_SIZE_MB - 100 {
            fs::remove_file(model_path).ok();
            anyhow::bail!(
                "Downloaded file appears incomplete ({}MB vs expected ~{}MB)",
                size_mb, DEFAULT_MODEL_SIZE_MB
            );
        }
        
        info!("Successfully downloaded Qwen 2.5 7B model ({}MB)", size_mb);
        Ok(())
    }
    
    /// Format messages for the model based on model type
    fn format_messages(&self, messages: &[Message]) -> String {
        let model_name_lower = self.model_type.to_lowercase();
        
        if model_name_lower.contains("qwen") {
            // Qwen format: <|im_start|>role\ncontent<|im_end|>
            let mut formatted = String::new();
            
            for message in messages {
                let role = match message.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };
                
                let content = message.as_concat_text();
                formatted.push_str(&format!(
                    "<|im_start|>{}\n{}<|im_end|>\n",
                    role, content
                ));
            }
            
            // Add the start of assistant response
            formatted.push_str("<|im_start|>assistant\n");
            formatted
        } else {
            // Generic/Llama format
            let mut formatted = String::new();
            
            for message in messages {
                match message.role {
                    Role::User => {
                        formatted.push_str(&format!("[INST] {} [/INST] ", message.as_concat_text()));
                    }
                    Role::Assistant => {
                        formatted.push_str(&format!("{} </s><s>", message.as_concat_text()));
                    }
                }
            }
            formatted
        }
    }
    
    /// Get stop sequences based on model type
    fn get_stop_sequences(&self) -> Vec<&'static str> {
        let model_name_lower = self.model_type.to_lowercase();
        
        if model_name_lower.contains("qwen") {
            vec!["<|im_end|>", "<|endoftext|>", "</s>", "<|im_start|>"]
        } else {
            vec!["</s>", "[/INST]", "<</SYS>>", "[INST]", "### Human:", "### Assistant:"]
        }
    }
    
    /// Generate completion from the model
    async fn generate_completion(
        &self,
        prompt: &str,
        max_tokens: u32,
        temperature: f32,
    ) -> Result<String, ProviderError> {
        let session = self.session.clone();
        let prompt = prompt.to_string();
        let stop_sequences = self.get_stop_sequences();
        
        // Run generation in a blocking task
        let result = tokio::task::spawn_blocking(move || -> Result<String, anyhow::Error> {
            let mut session = session.blocking_lock();
            
            // Set context
            session
                .set_context(&prompt)
                .map_err(|e| anyhow::anyhow!("Failed to set context: {}", e))?;
            
            // Create sampler
            let stages = vec![
                SamplerStage::Temperature(temperature),
                SamplerStage::TopK(40),
                SamplerStage::TopP(0.9),
            ];
            let sampler = StandardSampler::new_softmax(stages, 1);
            
            // Start completion
            let mut completion_handle = session
                .start_completing_with(sampler, max_tokens as usize)
                .map_err(|e| anyhow::anyhow!("Failed to start completion: {}", e))?;
            
            let mut generated_text = String::new();
            let mut token_count = 0;
            
            // Generate tokens
            while let Some(token) = completion_handle.next_token() {
                let token_string = session.model().token_to_piece(token);
                generated_text.push_str(&token_string);
                token_count += 1;
                
                if token_count >= max_tokens as usize {
                    break;
                }
                
                // Check for stop sequences
                let mut hit_stop = false;
                for stop_seq in &stop_sequences {
                    if generated_text.contains(stop_seq) {
                        hit_stop = true;
                        break;
                    }
                }
                
                if hit_stop {
                    break;
                }
            }
            
            // Clean up stop sequences from the end
            for stop_seq in &stop_sequences {
                if let Some(pos) = generated_text.find(stop_seq) {
                    generated_text.truncate(pos);
                    break;
                }
            }
            
            Ok(generated_text.trim().to_string())
        })
        .await
        .map_err(|e| ProviderError::ExecutionError(format!("Task join error: {}", e)))?
        .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
        
        Ok(result)
    }
    

}

#[async_trait]
impl Provider for EmbeddedProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "embedded",
            "Embedded",
            "Run local language models with optional tool execution",
            EMBEDDED_DEFAULT_MODEL,
            EMBEDDED_KNOWN_MODELS.to_vec(),
            "",
            vec![],
        )
    }
    
    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }
    
    #[tracing::instrument(
        skip(self, model_config, system, messages),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str, // Ignored - we use g3's system prompt for embedded
        messages: &[Message],
        _tools: &[Tool], // Ignored - embedded provider doesn't use tool definitions
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Use g3's approach: simple system prompt with JSON tool format instructions
        let mut full_messages = Vec::new();
        
        // Use g3's system prompt for embedded providers (with JSON tool format)
        let g3_system_prompt = if self.enable_tools {
            "You are Goose, a general-purpose AI agent. Your goal is to analyze and solve problems by writing code.

# Tool Call Format

When you need to execute a tool, write ONLY the JSON tool call on a new line:

{\"tool\": \"tool_name\", \"args\": {\"param\": \"value\"}}

The tool will execute immediately and you'll receive the result (success or error) to continue with.

# Available Tools

- **shell**: Execute shell commands
  - Format: {\"tool\": \"shell\", \"args\": {\"command\": \"your_command_here\"}}
  - Example: {\"tool\": \"shell\", \"args\": {\"command\": \"ls ~/Downloads\"}}

- **final_output**: Signal task completion with a detailed summary of work done
  - Format: {\"tool\": \"final_output\", \"args\": {\"summary\": \"what_was_accomplished\"}}

# Instructions

1. Analyze the request and break down into smaller tasks if appropriate
2. Execute ONE tool at a time
3. STOP when the original request was satisfied
4. Call the final_output tool when done

# Response Guidelines

- Use Markdown formatting for all responses except tool calls.
- Whenever taking actions, use the pronoun 'I'"
        } else {
            // Without tools, just be a simple assistant
            "You are Goose, a general-purpose AI assistant. Provide helpful responses using markdown formatting."
        };
        
        // Add the system prompt as the first user message (works better for embedded models)
        full_messages.push(Message::user().with_text(g3_system_prompt));
        
        // Add conversation messages directly
        full_messages.extend_from_slice(messages);
        
        // Format for the model
        let prompt = self.format_messages(&full_messages);
        
        // Generate completion
        let mut generated_text = self
            .generate_completion(&prompt, self.max_tokens, self.temperature)
            .await?;
        
        // If tools are enabled, parse and execute JSON tool calls in the response
        // This is built-in functionality like g3, not exposed as tool definitions
        if self.enable_tools {
            generated_text = ToolExecutor::execute_tool_calls(&generated_text).await;
        }
        
        // Create response message
        let message = Message::new(
            Role::Assistant,
            chrono::Utc::now().timestamp(),
            vec![MessageContent::text(generated_text.clone())],
        );
        
        // Estimate token usage
        let prompt_tokens = (prompt.len() / 4) as i32;
        let completion_tokens = (generated_text.len() / 4) as i32;
        
        let usage = Usage {
            input_tokens: Some(prompt_tokens),
            output_tokens: Some(completion_tokens),
            total_tokens: Some(prompt_tokens + completion_tokens),
        };
        
        // Debug tracing
        let payload = json!({
            "model": model_config.model_name,
            "system_length": system.len(),
            "messages": messages.len(),
            "tools_enabled": self.enable_tools,
        });
        
        let response = json!({
            "text_length": generated_text.len(),
            "usage": usage,
        });
        
        emit_debug_trace(model_config, &payload, &response, &usage);
        
        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), usage),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_embedded_model_config() {
        let provider = EmbeddedProvider::default();
        let config = provider.get_model_config();
        
        assert_eq!(config.model_name, EMBEDDED_DEFAULT_MODEL);
        assert!(config.context_limit() > 0);
    }
}
