use crate::config::paths::Paths;
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::{
    MessageStream, Provider, ProviderDef, ProviderMetadata, ProviderUsage, Usage,
};
use crate::providers::errors::ProviderError;
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use candle_core::{Device, Tensor};
use candle_transformers::models::{quantized_llama, quantized_phi, quantized_phi3};
use futures::future::BoxFuture;
use rmcp::model::{CallToolRequestParams, Role, Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::Mutex;
use utoipa::ToSchema;
use uuid::Uuid;

const PROVIDER_NAME: &str = "local";
const DEFAULT_MODEL: &str = "llama-3.2-1b";

pub const LOCAL_LLM_MODEL_CONFIG_KEY: &str = "LOCAL_LLM_MODEL";

// Load tiny model system prompt with environment context
fn load_tiny_model_prompt() -> String {
    use std::env;

    let os = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let working_directory = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let context = json!({
        "os": os,
        "working_directory": working_directory,
        "shell": shell,
    });

    crate::prompt_template::render_template("tiny_model_system.md", &context).unwrap_or_else(|e| {
        // Fallback if template fails to load
        eprintln!("WARNING: Failed to load tiny_model_system.md: {:?}", e);
        "You are Goose, an AI assistant. You can execute shell commands by starting lines with $."
            .to_string()
    })
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    Tiny,
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatTemplate {
    Llama3,
    ChatML,
    Mistral,
}

impl Default for ChatTemplate {
    fn default() -> Self {
        ChatTemplate::Llama3
    }
}

impl ChatTemplate {
    /// Get EOS token strings to strip from output
    fn eos_strings(&self) -> &[&str] {
        match self {
            ChatTemplate::Llama3 => &["<|eot_id|>", "<|end_of_text|>"],
            ChatTemplate::ChatML => &["<|im_end|>"],
            ChatTemplate::Mistral => &["</s>"],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocalLlmModel {
    /// Model identifier (e.g., "llama-3.2-1b")
    pub id: &'static str,
    /// Display name
    pub name: &'static str,
    /// Model file size in MB
    pub size_mb: u32,
    /// Maximum context window in tokens
    pub context_limit: usize,
    /// Download URL for the model GGUF file
    pub url: &'static str,
    /// Download URL for the tokenizer JSON
    pub tokenizer_url: &'static str,
    /// Description and use case
    pub description: &'static str,
    /// Model tier/category
    pub tier: ModelTier,
    /// Chat template format
    #[serde(skip)]
    pub chat_template: ChatTemplate,
}

const LOCAL_LLM_MODELS: &[LocalLlmModel] = &[
    LocalLlmModel {
        id: "llama-3.2-1b",
        name: "Llama 3.2 1B Instruct",
        size_mb: 700,
        context_limit: 4096,
        url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
        tokenizer_url: "https://huggingface.co/NousResearch/Hermes-2-Pro-Llama-3-8B/resolve/main/tokenizer.json",
        description: "Fastest, CPU-optimized for quick responses",
        tier: ModelTier::Tiny,
        chat_template: ChatTemplate::Llama3,
    },
    LocalLlmModel {
        id: "llama-3.2-3b",
        name: "Llama 3.2 3B Instruct",
        size_mb: 2000,
        context_limit: 8192,
        url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        tokenizer_url: "https://huggingface.co/NousResearch/Hermes-2-Pro-Llama-3-8B/resolve/main/tokenizer.json",
        description: "Good balance of speed and quality for laptops",
        tier: ModelTier::Small,
        chat_template: ChatTemplate::Llama3,
    },
    LocalLlmModel {
        id: "hermes-2-pro-7b",
        name: "Hermes 2 Pro Llama-3 7B",
        size_mb: 4500,
        context_limit: 8192,
        url: "https://huggingface.co/NousResearch/Hermes-2-Pro-Llama-3-8B-GGUF/resolve/main/Hermes-2-Pro-Llama-3-8B-Q4_K_M.gguf",
        tokenizer_url: "https://huggingface.co/NousResearch/Hermes-2-Pro-Llama-3-8B/resolve/main/tokenizer.json",
        description: "High quality for desktops with GPU",
        tier: ModelTier::Medium,
        chat_template: ChatTemplate::ChatML,
    },
    LocalLlmModel {
        id: "mistral-small-22b",
        name: "Mistral Small 22B Instruct",
        size_mb: 13000,
        context_limit: 32768,
        url: "https://huggingface.co/bartowski/Mistral-Small-Instruct-2409-GGUF/resolve/main/Mistral-Small-Instruct-2409-Q4_K_M.gguf",
        tokenizer_url: "https://huggingface.co/mistralai/Mistral-Small-Instruct-2409/resolve/main/tokenizer.json",
        description: "Highest quality with long context support",
        tier: ModelTier::Large,
        chat_template: ChatTemplate::Mistral,
    },
];

impl LocalLlmModel {
    pub fn local_path(&self) -> PathBuf {
        Paths::in_data_dir("models").join(format!("{}.gguf", self.id))
    }

    pub fn tokenizer_path(&self) -> PathBuf {
        Paths::in_data_dir("models").join(format!("{}_tokenizer.json", self.id))
    }

    pub fn is_downloaded(&self) -> bool {
        self.local_path().exists() && self.tokenizer_path().exists()
    }
}

pub fn available_local_models() -> &'static [LocalLlmModel] {
    LOCAL_LLM_MODELS
}

pub fn get_local_model(id: &str) -> Option<&'static LocalLlmModel> {
    LOCAL_LLM_MODELS.iter().find(|m| m.id == id)
}

pub fn recommend_local_model() -> &'static str {
    let has_gpu = Device::new_cuda(0).is_ok() || Device::new_metal(0).is_ok();
    let mem_mb = sys_info::mem_info().map(|m| m.avail / 1024).unwrap_or(0);

    if has_gpu && mem_mb >= 16_000 {
        "hermes-2-pro-7b" // Medium tier - GPU with lots of memory
    } else if mem_mb >= 4_000 {
        "llama-3.2-3b" // Small tier - decent memory
    } else {
        "llama-3.2-1b" // Tiny tier - low memory
    }
}

enum ModelWeights {
    Llama(quantized_llama::ModelWeights),
    Phi(quantized_phi::ModelWeights),
    Phi3(quantized_phi3::ModelWeights),
}

impl ModelWeights {
    fn forward(&mut self, input: &Tensor, pos: usize) -> candle_core::Result<Tensor> {
        match self {
            ModelWeights::Llama(m) => m.forward(input, pos),
            ModelWeights::Phi(m) => m.forward(input, pos),
            ModelWeights::Phi3(m) => m.forward(input, pos),
        }
    }
}

struct LoadedModel {
    model: ModelWeights,
    tokenizer: Tokenizer,
    device: Device,
    eos_token_ids: Vec<u32>,
}

/// Streaming parser for emulator commands
/// Accumulates chunks and emits complete text or commands
struct StreamingEmulatorParser {
    buffer: String,
    in_command: bool,
    command_start_pos: usize,
}

impl StreamingEmulatorParser {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            in_command: false,
            command_start_pos: 0,
        }
    }

    /// Process a chunk and return any complete items (text or commands)
    /// Returns (optional_text, optional_command)
    fn process_chunk(&mut self, chunk: &str) -> Vec<(Option<String>, Option<String>)> {
        self.buffer.push_str(chunk);
        let mut results = Vec::new();

        loop {
            if self.in_command {
                // Look for newline to end the command
                if let Some(newline_pos) = self.buffer[self.command_start_pos..].find('\n') {
                    let absolute_pos = self.command_start_pos + newline_pos;
                    // Extract command from "$ command"
                    let command_line = &self.buffer[self.command_start_pos..absolute_pos];
                    if let Some(command) = command_line.strip_prefix('$') {
                        let command = command.trim();
                        if !command.is_empty() {
                            results.push((None, Some(command.to_string())));
                        }
                    }
                    // Remove processed part from buffer
                    self.buffer = self.buffer[absolute_pos + 1..].to_string();
                    self.in_command = false;
                    self.command_start_pos = 0;
                } else {
                    // Command not complete yet, wait for more chunks
                    break;
                }
            } else {
                // Look for command start: "\n$" or "$" at beginning
                if let Some(pos) = self.buffer.find("\n$") {
                    // Emit text before the command
                    let text = self.buffer[..pos + 1].to_string(); // Include the \n
                    if !text.trim().is_empty() {
                        results.push((Some(text), None));
                    }
                    // Remove text from buffer, start command parsing
                    self.buffer = self.buffer[pos + 1..].to_string(); // Buffer now starts with "$"
                    self.in_command = true;
                    self.command_start_pos = 0;
                } else if self.buffer.starts_with('$') && self.buffer.len() == chunk.len() {
                    // Command at very start of response (first chunk)
                    self.in_command = true;
                    self.command_start_pos = 0;
                } else {
                    // No command found, but keep last few chars in case of split pattern
                    // E.g., chunk ends with "\n" and next starts with "$"
                    if self.buffer.chars().count() > 2 && !self.buffer.ends_with('\n') {
                        // Emit all but last 2 characters as safe text (use char boundaries)
                        let mut chars = self.buffer.chars();
                        let keep_count = 2;
                        let emit_count = self.buffer.chars().count() - keep_count;

                        let emit_text: String = chars.by_ref().take(emit_count).collect();
                        let keep_text: String = chars.collect();

                        if !emit_text.is_empty() {
                            results.push((Some(emit_text), None));
                        }
                        self.buffer = keep_text;
                    }
                    break;
                }
            }
        }

        results
    }

    /// Flush any remaining buffer content, handling incomplete commands
    fn flush(&mut self) -> Vec<(Option<String>, Option<String>)> {
        let mut results = Vec::new();

        if !self.buffer.is_empty() {
            if self.in_command {
                // We're in the middle of parsing a command - complete it
                let command_line = self.buffer.trim();
                if let Some(command) = command_line.strip_prefix('$') {
                    let command = command.trim();
                    if !command.is_empty() {
                        results.push((None, Some(command.to_string())));
                    }
                } else if !command_line.is_empty() {
                    // Malformed command, just emit as text
                    results.push((Some(self.buffer.clone()), None));
                }
            } else {
                // Just regular text remaining
                results.push((Some(self.buffer.clone()), None));
            }
            self.buffer.clear();
            self.in_command = false;
        }

        results
    }
}

pub struct LocalInferenceProvider {
    model: Arc<Mutex<Option<LoadedModel>>>,
    model_config: ModelConfig,
    name: String,
}

impl LocalInferenceProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        Ok(Self {
            model: Arc::new(Mutex::new(None)),
            model_config: model,
            name: PROVIDER_NAME.to_string(),
        })
    }

    async fn load_model(model_id: &str) -> Result<LoadedModel, ProviderError> {
        // Get model definition
        let model_info = get_local_model(model_id)
            .ok_or_else(|| ProviderError::ExecutionError(format!("Unknown model: {}", model_id)))?;

        let model_path = model_info.local_path();
        let tokenizer_path = model_info.tokenizer_path();

        if !model_path.exists() {
            return Err(ProviderError::ExecutionError(format!(
                "Model not downloaded: {}. Please download it from Settings > Local Inference.",
                model_info.name
            )));
        }

        tracing::info!("Loading {} from: {}", model_info.name, model_path.display());

        // Device selection (from whisper.rs pattern)
        let device = if let Ok(device) = Device::new_metal(0) {
            tracing::info!("Using Metal device");
            device
        } else {
            tracing::info!("Using CPU device");
            Device::Cpu
        };

        // Load GGUF file
        let mut file = std::fs::File::open(&model_path).map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to open model file: {}", e))
        })?;

        // Read GGUF content
        let content = candle_core::quantized::gguf_file::Content::read(&mut file).map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to read GGUF file: {}", e))
        })?;

        // Detect model architecture from ID
        let model_id_lower = model_id.to_lowercase();
        let is_phi = model_id_lower.contains("phi");

        // Load model weights based on architecture
        // Try multiple architectures if name contains "phi"
        let (model, eos_token_id) = if is_phi {
            // Try Phi (Phi-2) first
            match quantized_phi::ModelWeights::from_gguf(content, &mut file, &device) {
                Ok(weights) => {
                    tracing::info!("Loaded with Phi architecture");
                    (ModelWeights::Phi(weights), 50256) // Phi-2 EOS token
                }
                Err(e1) => {
                    tracing::info!("Phi architecture failed ({}), trying Phi-3", e1);
                    // Reopen file for second attempt
                    let mut file = std::fs::File::open(&model_path).map_err(|e| {
                        ProviderError::ExecutionError(format!("Failed to reopen model file: {}", e))
                    })?;
                    let content = candle_core::quantized::gguf_file::Content::read(&mut file)
                        .map_err(|e| {
                            ProviderError::ExecutionError(format!(
                                "Failed to re-read GGUF file: {}",
                                e
                            ))
                        })?;

                    match quantized_phi3::ModelWeights::from_gguf(
                        false, content, &mut file, &device,
                    ) {
                        Ok(weights) => {
                            tracing::info!("Loaded with Phi-3 architecture");
                            (ModelWeights::Phi3(weights), 32000) // Phi-3 EOS token
                        }
                        Err(e2) => {
                            tracing::warn!(
                                "Phi-3 architecture failed ({}), falling back to Llama",
                                e2
                            );
                            // Try Llama as last resort
                            let mut file = std::fs::File::open(&model_path).map_err(|e| {
                                ProviderError::ExecutionError(format!(
                                    "Failed to reopen model file: {}",
                                    e
                                ))
                            })?;
                            let content =
                                candle_core::quantized::gguf_file::Content::read(&mut file)
                                    .map_err(|e| {
                                        ProviderError::ExecutionError(format!(
                                            "Failed to re-read GGUF file: {}",
                                            e
                                        ))
                                    })?;

                            let weights = quantized_llama::ModelWeights::from_gguf(
                                content, &mut file, &device,
                            )
                            .map_err(|e| {
                                ProviderError::ExecutionError(format!(
                                    "Failed to load as Phi ({}), Phi-3 ({}), or Llama ({})",
                                    e1, e2, e
                                ))
                            })?;
                            tracing::info!(
                                "Loaded Phi model with Llama architecture (may not work correctly)"
                            );
                            (ModelWeights::Llama(weights), 50256) // Use Phi EOS token
                        }
                    }
                }
            }
        } else {
            tracing::info!("Using Llama architecture");
            let weights = quantized_llama::ModelWeights::from_gguf(content, &mut file, &device)
                .map_err(|e| {
                    ProviderError::ExecutionError(format!(
                        "Failed to load Llama model weights: {}",
                        e
                    ))
                })?;
            (ModelWeights::Llama(weights), 128001) // Llama 3 EOS token
        };

        // Load tokenizer
        let tokenizer = if tokenizer_path.exists() {
            tracing::info!("Loading tokenizer from: {}", tokenizer_path.display());
            Tokenizer::from_file(&tokenizer_path).map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to load tokenizer: {}", e))
            })?
        } else {
            return Err(ProviderError::ExecutionError(format!(
                "Tokenizer not found at {}. Please download the model again.",
                tokenizer_path.display()
            )));
        };

        // Build list of EOS token IDs: start with architecture default, then add template-specific ones
        let mut eos_token_ids = vec![eos_token_id];

        // Look up token IDs for the chat template's EOS strings from tokenizer
        for eos_str in model_info.chat_template.eos_strings() {
            if let Some(token_id) = tokenizer.token_to_id(eos_str) {
                if !eos_token_ids.contains(&token_id) {
                    eos_token_ids.push(token_id);
                }
            } else {
                tracing::warn!("EOS string '{}' not found in tokenizer vocabulary", eos_str);
            }
        }

        tracing::info!("Model loaded successfully");

        Ok(LoadedModel {
            model,
            tokenizer,
            device,
            eos_token_ids,
        })
    }

    fn build_prompt(
        &self,
        system: &str,
        messages: &[Message],
        template: ChatTemplate,
        tools: &[Tool],
    ) -> String {
        match template {
            ChatTemplate::Llama3 => Self::format_llama3(system, messages, tools),
            ChatTemplate::ChatML => Self::format_chatml(system, messages, tools),
            ChatTemplate::Mistral => Self::format_mistral(system, messages, tools),
        }
    }

    /// Format message content for emulator, including text and tool responses
    fn format_message_content_for_emulator(msg: &Message) -> String {
        let mut parts = Vec::new();

        for content in &msg.content {
            match content {
                MessageContent::Text(text) => {
                    parts.push(text.text.clone());
                }
                MessageContent::ToolResponse(response) => {
                    // Include tool results in the prompt so model sees the output
                    match &response.tool_result {
                        Ok(result) => {
                            for content_item in &result.content {
                                if let Some(text_content) = content_item.as_text() {
                                    parts.push(text_content.text.to_string());
                                }
                                // Skip images and resources for now
                            }
                        }
                        Err(e) => {
                            parts.push(format!("Error: {}", e));
                        }
                    }
                }
                _ => {
                    // Skip tool requests, images, etc.
                }
            }
        }

        parts.join("\n")
    }

    fn format_llama3(system: &str, messages: &[Message], tools: &[Tool]) -> String {
        let mut prompt = String::from("<|begin_of_text|>");

        // Add system message
        if !system.is_empty() || !tools.is_empty() {
            prompt.push_str("<|start_header_id|>system<|end_header_id|>\n\n");
            prompt.push_str(system);

            // Add tools if present
            if !tools.is_empty() {
                if !system.is_empty() {
                    prompt.push_str("\n\n");
                }
                prompt.push_str("# Tools\n\nYou have access to the following tools:\n\n");
                for tool in tools {
                    let desc = tool
                        .description
                        .as_ref()
                        .map(|d| d.as_ref())
                        .unwrap_or("No description");
                    prompt.push_str(&format!("- {}: {}\n", tool.name, desc));
                }
            }

            prompt.push_str("<|eot_id|>");
        }

        // Add conversation messages
        for msg in messages {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };

            let content = Self::format_message_content_for_emulator(msg);
            if !content.trim().is_empty() {
                prompt.push_str(&format!("<|start_header_id|>{}<|end_header_id|>\n\n", role));
                prompt.push_str(&content);
                prompt.push_str("<|eot_id|>");
            }
        }

        // Add assistant prefix to prompt completion
        prompt.push_str("<|start_header_id|>assistant<|end_header_id|>\n\n");
        prompt
    }

    fn format_chatml(system: &str, messages: &[Message], _tools: &[Tool]) -> String {
        let mut prompt = String::new();

        // Add system message
        if !system.is_empty() {
            prompt.push_str("<|im_start|>system\n");
            prompt.push_str(system);
            prompt.push_str("<|im_end|>\n");
        }

        // Add conversation messages
        for msg in messages {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };

            let content = Self::format_message_content_for_emulator(msg);
            if !content.trim().is_empty() {
                prompt.push_str(&format!("<|im_start|>{}\n", role));
                prompt.push_str(&content);
                prompt.push_str("<|im_end|>\n");
            }
        }

        // Add assistant prefix
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }

    fn format_mistral(system: &str, messages: &[Message], _tools: &[Tool]) -> String {
        let mut prompt = String::new();

        // Mistral doesn't have a separate system role, prepend to first user message
        let system_prefix = if !system.is_empty() {
            format!("{}\n\n", system)
        } else {
            String::new()
        };

        // Add conversation messages
        let mut first_user = true;
        for msg in messages {
            let content = Self::format_message_content_for_emulator(msg);
            if content.trim().is_empty() {
                continue;
            }

            match msg.role {
                Role::User => {
                    prompt.push_str("[INST] ");
                    if first_user {
                        prompt.push_str(&system_prefix);
                        first_user = false;
                    }
                    prompt.push_str(&content);
                    prompt.push_str(" [/INST]");
                }
                Role::Assistant => {
                    prompt.push(' ');
                    prompt.push_str(&content);
                    prompt.push_str("</s>");
                }
            }
        }

        // If no messages, still include system in first user turn
        if first_user && !system.is_empty() {
            prompt.push_str("[INST] ");
            prompt.push_str(&system_prefix);
            prompt.push_str("[/INST]");
        }

        prompt
    }
}

impl ProviderDef for LocalInferenceProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        ProviderMetadata::new(
            PROVIDER_NAME,
            "Local Inference",
            "Local inference using quantized GGUF models (Candle)",
            DEFAULT_MODEL,
            vec![
                "llama-3.2-1b",
                "llama-3.2-3b",
                "hermes-2-pro-7b",
                "mistral-small-22b",
            ],
            "https://github.com/huggingface/candle",
            vec![], // No API keys required - models managed through UI
        )
    }

    fn from_env(model: ModelConfig) -> BoxFuture<'static, Result<Self::Provider>>
    where
        Self: Sized,
    {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for LocalInferenceProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model_config.clone()
    }

    async fn generate_session_name(
        &self,
        _session_id: &str,
        _messages: &crate::conversation::Conversation,
    ) -> Result<String, ProviderError> {
        // Disable session naming for performance
        Ok("Local conversation".to_string())
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        // Return all models - UI will show "(not downloaded)" for ones that aren't available
        let all_models: Vec<String> = available_local_models()
            .iter()
            .map(|m| m.id.to_string())
            .collect();

        Ok(all_models)
    }

    async fn complete_with_model(
        &self,
        session_id: Option<&str>,
        _model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        use futures::StreamExt;

        // Just call stream and accumulate results
        let mut stream = self
            .stream(session_id.unwrap_or(""), system, messages, tools)
            .await?;

        let mut accumulated_message = Message::assistant();
        let mut final_usage = None;

        while let Some(result) = stream.next().await {
            let (message_opt, usage_opt) = result?;

            if let Some(msg) = message_opt {
                // Accumulate message content
                accumulated_message.id = msg.id.or(accumulated_message.id);
                for content in msg.content {
                    accumulated_message.content.push(content);
                }
            }

            if let Some(usage) = usage_opt {
                final_usage = Some(usage);
            }
        }

        let usage = final_usage.ok_or_else(|| {
            ProviderError::ExecutionError("Stream ended without usage information".to_string())
        })?;

        Ok((accumulated_message, usage))
    }

    async fn stream(
        &self,
        _session_id: &str,
        _system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        // Get model metadata to determine chat template
        let model_config = &self.model_config;
        let model_info = get_local_model(&model_config.model_name).ok_or_else(|| {
            ProviderError::ExecutionError(format!("Model not found: {}", model_config.model_name))
        })?;
        let template = model_info.chat_template;

        let tiny_prompt = load_tiny_model_prompt();

        let prompt = self.build_prompt(&tiny_prompt, &messages, template, tools);

        let mut model_lock = self.model.lock().await;
        if model_lock.is_none() {
            *model_lock = Some(Self::load_model(&model_config.model_name).await?);
        }

        // Clone Arc to move into the stream
        let model_arc = self.model.clone();
        let model_name = model_config.model_name.clone();

        Ok(Box::pin(try_stream! {
            // Generate a consistent message ID for all chunks
            let message_id = Uuid::new_v4().to_string();

            // Create streaming parser for emulator commands
            let mut parser = StreamingEmulatorParser::new();

            // Get mutable access to model
            let mut model_lock = model_arc.lock().await;
            let loaded = model_lock.as_mut().ok_or_else(|| {
                ProviderError::ExecutionError("Model not loaded".to_string())
            })?;

            // Encode prompt
            let prompt_tokens = loaded
                .tokenizer
                .encode(prompt.as_str(), false)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to encode prompt: {}", e)))?
                .get_ids()
                .to_vec();

            // PREFILL: Process entire prompt at once for speed
            let input = Tensor::new(prompt_tokens.as_slice(), &loaded.device)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to create input tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to unsqueeze input tensor: {}", e)))?;

            let logits = loaded.model.forward(&input, 0).map_err(|e| {
                ProviderError::ExecutionError(format!("Prefill forward pass failed: {}", e))
            })?;

            // Quantized model returns [batch_size, vocab_size] directly for the last position
            // Just squeeze to get [vocab_size] and sample
            let logits = logits
                .squeeze(0)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to squeeze batch dim: {}", e)))?;

            let mut next_token = logits
                .argmax(0)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to sample token: {}", e)))?
                .to_scalar::<u32>()
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to convert token: {}", e)))?;

            let decoded = loaded
                .tokenizer
                .decode(&[next_token], false)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to decode token: {}", e)))?;

            // Process first token through parser (skip if EOS)
            let mut tool_call_emitted = false;
            if !loaded.eos_token_ids.contains(&next_token) {
                let parse_results = parser.process_chunk(&decoded);
                for (text, command) in parse_results {
                    if let Some(text) = text {
                        let mut message = Message::assistant().with_text(&text);
                        message.id = Some(message_id.clone());
                        yield (Some(message), None);
                    }
                    if let Some(command) = command {
                        // Create tool request
                        let tool_id = Uuid::new_v4().to_string();
                        let mut args = serde_json::Map::new();
                        args.insert("command".to_string(), json!(command));

                        let tool_call = CallToolRequestParams {
                        meta: None,
                        task: None,
                        name: Cow::Borrowed("developer__shell"),
                        arguments: Some(args),
                    };

                    let mut message = Message::assistant();
                    message.content.push(MessageContent::tool_request(tool_id, Ok(tool_call)));
                    message.id = Some(message_id.clone());
                    yield (Some(message), None);

                    // Stop after first tool call
                    tool_call_emitted = true;
                }
                }
            }

            // GENERATION LOOP: Generate remaining tokens (only if no tool call yet)
            // Use model's context limit, cap output at 2K tokens to leave room for prompt
            let max_output = model_info.context_limit.saturating_sub(prompt_tokens.len()).min(2048);
            let mut output_token_count: i32 = 1; // Count the first token from prefill
            if !tool_call_emitted {
                for index in 0..max_output.saturating_sub(1) {
                // Check for EOS tokens
                if loaded.eos_token_ids.contains(&next_token) {
                    break;
                }

                // Single token input for generation
                let input = Tensor::new(&[next_token], &loaded.device)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to create tensor: {}", e)))?
                    .unsqueeze(0)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to unsqueeze tensor: {}", e)))?;

                // Position is prompt_len + already_generated_tokens
                // We already generated 1 token from prefill, so add 1
                let pos = prompt_tokens.len() + index + 1;
                let logits = loaded
                    .model
                    .forward(&input, pos)
                    .map_err(|e| ProviderError::ExecutionError(format!("Generation forward pass failed at pos {}: {}", pos, e)))?;

                let logits = logits.squeeze(0)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to squeeze logits: {}", e)))?;

                next_token = logits.argmax(0)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to sample token: {}", e)))?
                    .to_scalar::<u32>()
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to convert token: {}", e)))?;

                // Check for EOS before decoding/yielding
                if loaded.eos_token_ids.contains(&next_token) {
                    break;
                }

                // Count the generated token
                output_token_count += 1;

                // Decode token
                let mut decoded = loaded
                    .tokenizer
                    .decode(&[next_token], false)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to decode token: {}", e)))?;

                // Strip EOS tokens from this chunk
                for eos_str in template.eos_strings() {
                    decoded = decoded.replace(eos_str, "");
                }

                if !decoded.is_empty() {
                    // Process through parser
                    let parse_results = parser.process_chunk(&decoded);
                    for (text, command) in parse_results {
                        if let Some(text) = text {
                            let mut message = Message::assistant().with_text(&text);
                            message.id = Some(message_id.clone());
                            yield (Some(message), None);
                        }
                        if let Some(command) = command {
                            // Create tool request
                            let tool_id = Uuid::new_v4().to_string();
                            let mut args = serde_json::Map::new();
                            args.insert("command".to_string(), json!(command));

                            let tool_call = CallToolRequestParams {
                                meta: None,
                                task: None,
                                name: Cow::Borrowed("developer__shell"),
                                arguments: Some(args),
                            };

                            let mut message = Message::assistant();
                            message.content.push(MessageContent::tool_request(tool_id, Ok(tool_call)));
                            message.id = Some(message_id.clone());
                            yield (Some(message), None);

                            // Stop generation after first tool call
                            tool_call_emitted = true;
                        }
                    }
                }

                // Break out of generation loop after tool call
                if tool_call_emitted {
                    break;
                }
                }
            }

            // Flush any remaining parser buffer (handles incomplete commands at end of stream)
            let flush_results = parser.flush();
            for (text, command) in flush_results {
                if let Some(text) = text {
                    let mut message = Message::assistant().with_text(&text);
                    message.id = Some(message_id.clone());
                    yield (Some(message), None);
                }
                if let Some(command) = command {
                    // Create tool request for the final command
                    let tool_id = Uuid::new_v4().to_string();
                    let mut args = serde_json::Map::new();
                    args.insert("command".to_string(), json!(command));

                    let tool_call = CallToolRequestParams {
                        meta: None,
                        task: None,
                        name: Cow::Borrowed("developer__shell"),
                        arguments: Some(args),
                    };

                    let mut message = Message::assistant();
                    message.content.push(MessageContent::tool_request(tool_id, Ok(tool_call)));
                    message.id = Some(message_id.clone());
                    yield (Some(message), None);
                }
            }

            // Final yield with usage
            let input_tokens = prompt_tokens.len() as i32;
            let total_tokens = input_tokens + output_token_count;
            let usage = Usage::new(Some(input_tokens), Some(output_token_count), Some(total_tokens));
            let provider_usage = ProviderUsage::new(model_name.clone(), usage);
            yield (None, Some(provider_usage));
        }))
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
