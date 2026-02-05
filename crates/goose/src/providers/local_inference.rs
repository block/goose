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
use rmcp::model::Role;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::Mutex;
use utoipa::ToSchema;
use uuid::Uuid;

const PROVIDER_NAME: &str = "local";
const DEFAULT_MODEL: &str = "llama-3.2-1b";

pub const LOCAL_LLM_MODEL_CONFIG_KEY: &str = "LOCAL_LLM_MODEL";

const LOCAL_SYSTEM_PROMPT: &str = "You are Goose, an AI assistant running locally on the user's machine using a quantized language model. \

IMPORTANT: You do not have access to tools, file system operations, web browsing, or code execution. You can only provide text responses and guidance.

If the user asks you to:
- Run commands or execute code
- Read or write files
- Browse the web or search for information
- Use any external tools

Politely inform them that local models don't support these features yet, and suggest they switch to a cloud provider (like Anthropic, OpenAI, or Google) in the model settings for full Goose functionality.";

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
    eos_token_id: u32,
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
        let model = get_local_model(model_id)
            .ok_or_else(|| ProviderError::ExecutionError(format!("Unknown model: {}", model_id)))?;

        let model_path = model.local_path();
        let tokenizer_path = model.tokenizer_path();

        if !model_path.exists() {
            return Err(ProviderError::ExecutionError(format!(
                "Model not downloaded: {}. Please download it from Settings > Local Inference.",
                model.name
            )));
        }

        tracing::info!("Loading {} from: {}", model.name, model_path.display());

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

        tracing::info!("Model loaded successfully");

        Ok(LoadedModel {
            model,
            tokenizer,
            device,
            eos_token_id,
        })
    }

    async fn generate(
        &self,
        loaded: &mut LoadedModel,
        prompt: &str,
        max_tokens: usize,
        template: ChatTemplate,
    ) -> Result<String, ProviderError> {
        // Encode prompt
        let prompt_tokens = loaded
            .tokenizer
            .encode(prompt, false)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to encode prompt: {}", e)))?
            .get_ids()
            .to_vec();

        // PREFILL: Process prompt tokens one-by-one for stability
        let mut next_token = 0u32;
        for (pos, &token) in prompt_tokens.iter().enumerate() {
            let input = Tensor::new(&[token], &loaded.device)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to create tensor at pos {}: {}", pos, e)))?
                .unsqueeze(0)
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Failed to unsqueeze tensor at pos {}: {}", pos, e))
                })?;

            let logits = loaded.model.forward(&input, pos).map_err(|e| {
                ProviderError::ExecutionError(format!("Prefill forward pass failed at pos {}: {}", pos, e))
            })?;

            let logits = logits.squeeze(0).map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to squeeze logits at pos {}: {}", pos, e))
            })?;

            next_token = logits
                .argmax(0)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to sample token at pos {}: {}", pos, e)))?
                .to_scalar::<u32>()
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Failed to convert token at pos {}: {}", pos, e))
                })?;
        }

        let mut generated_text = loaded
            .tokenizer
            .decode(&[next_token], false)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to decode token: {}", e)))?;

        // GENERATION LOOP: Now generate remaining tokens using KV-cache
        for index in 0..max_tokens.saturating_sub(1) {
            // Check for EOS tokens (both variants for Llama 3/3.1/3.2)
            if next_token == loaded.eos_token_id || next_token == 128009 {
                break;
            }

            // Single token input for generation
            let input = Tensor::new(&[next_token], &loaded.device)
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Failed to create tensor: {}", e))
                })?
                .unsqueeze(0)
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Failed to unsqueeze tensor: {}", e))
                })?;

            // Forward pass with correct position
            // After prefill of N tokens at position 0, first generated token is at position N
            // We already generated that token, so loop generates tokens at positions N+1, N+2, ...
            let pos = prompt_tokens.len() + index + 1;
            let logits = loaded.model.forward(&input, pos).map_err(|e| {
                ProviderError::ExecutionError(format!(
                    "Generation forward pass failed at pos {}: {}",
                    pos, e
                ))
            })?;

            // Squeeze to get [vocab_size]
            let logits = logits.squeeze(0).map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to squeeze logits: {}", e))
            })?;

            // Sample next token
            next_token = logits
                .argmax(0)
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Failed to sample token: {}", e))
                })?
                .to_scalar::<u32>()
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Failed to convert token: {}", e))
                })?;

            // Decode and append
            let decoded = loaded.tokenizer.decode(&[next_token], false).map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to decode token: {}", e))
            })?;

            generated_text.push_str(&decoded);
        }

        // Strip EOS tokens from output
        let mut clean_text = generated_text;
        for eos_str in template.eos_strings() {
            clean_text = clean_text.replace(eos_str, "");
        }

        Ok(clean_text)
    }

    fn build_prompt(&self, system: &str, messages: &[Message], template: ChatTemplate, tools: &[Tool]) -> String {
        match template {
            ChatTemplate::Llama3 => Self::format_llama3(system, messages, tools),
            ChatTemplate::ChatML => Self::format_chatml(system, messages, tools),
            ChatTemplate::Mistral => Self::format_mistral(system, messages, tools),
        }
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
                    let desc = tool.description.as_ref().map(|d| d.as_ref()).unwrap_or("No description");
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

            prompt.push_str(&format!("<|start_header_id|>{}<|end_header_id|>\n\n", role));
            prompt.push_str(&msg.as_concat_text());
            prompt.push_str("<|eot_id|>");
        }

        // Add assistant prefix to prompt completion
        prompt.push_str("<|start_header_id|>assistant<|end_header_id|>\n\n");
        prompt
    }

    fn format_chatml(system: &str, messages: &[Message], tools: &[Tool]) -> String {
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

            prompt.push_str(&format!("<|im_start|>{}\n", role));
            prompt.push_str(&msg.as_concat_text());
            prompt.push_str("<|im_end|>\n");
        }

        // Add assistant prefix
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }

    fn format_mistral(system: &str, messages: &[Message], tools: &[Tool]) -> String {
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
            match msg.role {
                Role::User => {
                    prompt.push_str("[INST] ");
                    if first_user {
                        prompt.push_str(&system_prefix);
                        first_user = false;
                    }
                    prompt.push_str(&msg.as_concat_text());
                    prompt.push_str(" [/INST]");
                }
                Role::Assistant => {
                    prompt.push(' ');
                    prompt.push_str(&msg.as_concat_text());
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
        // Skip expensive inference for session naming
        Ok("Local conversation".to_string())
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // Return all models - UI will show "(not downloaded)" for ones that aren't available
        let all_models: Vec<String> = available_local_models()
            .iter()
            .map(|m| m.id.to_string())
            .collect();

        Ok(Some(all_models))
    }

    async fn complete_with_model(
        &self,
        _session_id: Option<&str>,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Get model metadata to determine chat template
        let model_info = get_local_model(&model_config.model_name).ok_or_else(|| {
            ProviderError::ExecutionError(format!("Model not found: {}", model_config.model_name))
        })?;

        // Check first character of last user message for test mode
        let mut test_mode = None;
        let mut modified_messages = messages.to_vec();
        if let Some(last_msg) = modified_messages.last_mut() {
            // Find the text content item (skip info-msg blocks)
            for (idx, content) in last_msg.content.iter().enumerate() {
                if let MessageContent::Text(text) = content {
                    // Skip info-msg blocks
                    if text.text.starts_with("<info-msg>") {
                        continue;
                    }

                    // Check first character for test mode
                    if let Some(first_char) = text.text.chars().next() {
                        if first_char == '1' || first_char == '2' || first_char == '3' {
                            test_mode = Some(first_char);
                            eprintln!("TEST MODE {}: Detected from message", first_char);
                            // Strip the first character from this content item
                            let stripped = text.text.chars().skip(1).collect::<String>();
                            last_msg.content[idx] = MessageContent::text(stripped);
                            break;
                        }
                    }
                    break; // Only check first non-info-msg text content
                }
            }
        }

        // Build prompt based on test mode
        let (system_to_use, tools_to_use) = match test_mode {
            Some('1') => {
                eprintln!("TEST MODE 1: Local system prompt, no tools");
                (LOCAL_SYSTEM_PROMPT, &[] as &[Tool])
            }
            Some('2') => {
                eprintln!("TEST MODE 2: Provided system prompt, no tools");
                (system, &[] as &[Tool])
            }
            Some('3') => {
                eprintln!("TEST MODE 3: Provided system prompt with tools");
                (system, tools)
            }
            _ => {
                // Default: use local system prompt
                (LOCAL_SYSTEM_PROMPT, &[] as &[Tool])
            }
        };

        let prompt = self.build_prompt(system_to_use, &modified_messages, model_info.chat_template, tools_to_use);

        // Load model if needed
        let mut model_lock = self.model.lock().await;
        if model_lock.is_none() {
            *model_lock = Some(Self::load_model(&model_config.model_name).await?);
        }
        let loaded = model_lock.as_mut().unwrap();

        // Generate response
        let response = self
            .generate(loaded, &prompt, 100, model_info.chat_template)
            .await?;
        tracing::info!("Generation complete: {} chars", response.len());

        // Return message
        let message = Message::assistant().with_text(&response);
        let usage = Usage::new(None, None, None); // Will estimate later

        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), usage),
        ))
    }

    async fn stream(
        &self,
        _session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        // Get model metadata to determine chat template
        let model_config = &self.model_config;
        let model_info = get_local_model(&model_config.model_name).ok_or_else(|| {
            ProviderError::ExecutionError(format!("Model not found: {}", model_config.model_name))
        })?;
        let template = model_info.chat_template;

        // Check first character of last user message for test mode
        let mut test_mode = None;
        let mut modified_messages = messages.to_vec();
        if let Some(last_msg) = modified_messages.last_mut() {
            // Find the text content item (skip info-msg blocks)
            for (idx, content) in last_msg.content.iter().enumerate() {
                if let MessageContent::Text(text) = content {
                    // Skip info-msg blocks
                    if text.text.starts_with("<info-msg>") {
                        continue;
                    }

                    // Check first character for test mode
                    if let Some(first_char) = text.text.chars().next() {
                        if first_char == '1' || first_char == '2' || first_char == '3' {
                            test_mode = Some(first_char);
                            eprintln!("TEST MODE {}: Detected from message", first_char);
                            // Strip the first character from this content item
                            let stripped = text.text.chars().skip(1).collect::<String>();
                            last_msg.content[idx] = MessageContent::text(stripped);
                            break;
                        }
                    }
                    break; // Only check first non-info-msg text content
                }
            }
        }

        // Build prompt based on test mode
        let (system_to_use, tools_to_use) = match test_mode {
            Some('1') => {
                eprintln!("TEST MODE 1: Local system prompt, no tools");
                (LOCAL_SYSTEM_PROMPT, &[] as &[Tool])
            }
            Some('2') => {
                eprintln!("TEST MODE 2: Provided system prompt, no tools");
                (system, &[] as &[Tool])
            }
            Some('3') => {
                eprintln!("TEST MODE 3: Provided system prompt with tools");
                (system, tools)
            }
            _ => {
                // Default: use local system prompt
                (LOCAL_SYSTEM_PROMPT, &[] as &[Tool])
            }
        };

        let prompt = self.build_prompt(system_to_use, &modified_messages, template, tools_to_use);

        // Debug: Save prompt to file for testing
        if let Ok(_) = std::fs::write("/tmp/goose_prompt_stream.txt", &prompt) {
            eprintln!("DEBUG: Saved prompt to /tmp/goose_prompt_stream.txt ({} bytes)", prompt.len());
        }

        // Lazy load model if needed
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

            // PREFILL: Process prompt tokens one-by-one for stability
            let mut next_token = 0u32;
            for (pos, &token) in prompt_tokens.iter().enumerate() {
                let input = Tensor::new(&[token], &loaded.device)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to create tensor at pos {}: {}", pos, e)))?
                    .unsqueeze(0)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to unsqueeze tensor at pos {}: {}", pos, e)))?;

                let logits = loaded
                    .model
                    .forward(&input, pos)
                    .map_err(|e| ProviderError::ExecutionError(format!("Prefill forward pass failed at pos {}: {}", pos, e)))?;

                let logits = logits.squeeze(0)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to squeeze logits at pos {}: {}", pos, e)))?;

                next_token = logits.argmax(0)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to sample token at pos {}: {}", pos, e)))?
                    .to_scalar::<u32>()
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to convert token at pos {}: {}", pos, e)))?;

                // Debug last few positions
                if pos >= prompt_tokens.len().saturating_sub(5) {
                    eprintln!("DEBUG: pos={}, input_token={}, next_token={}, logits_shape={:?}", pos, token, next_token, logits.shape());

                    // At the very last position, check if logits are valid
                    if pos == prompt_tokens.len() - 1 {
                        // Get top 5 token IDs and their logit values
                        if let Ok(flat_logits) = logits.to_vec1::<f32>() {
                            let mut indexed: Vec<(usize, f32)> = flat_logits.iter().copied().enumerate().collect();
                            indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                            eprintln!("DEBUG: Top 5 tokens at last position:");
                            for (i, (idx, val)) in indexed.iter().take(5).enumerate() {
                                eprintln!("  {}. token_id={}, logit={:.4}", i+1, idx, val);
                            }
                            eprintln!("DEBUG: Token 791 ('The') logit: {:.4}", flat_logits.get(791).unwrap_or(&-999.0));
                            eprintln!("DEBUG: Token 127999 (garbage) logit: {:.4}", flat_logits.get(127999).unwrap_or(&-999.0));
                        }
                    }
                }
            }

            eprintln!("DEBUG: First token after prefill: ID={}, prompt_len={}", next_token, prompt_tokens.len());

            let decoded = loaded
                .tokenizer
                .decode(&[next_token], false)
                .map_err(|e| ProviderError::ExecutionError(format!("Failed to decode token: {}", e)))?;

            eprintln!("DEBUG: First decoded token: '{}'", decoded);

            // Yield first token
            let mut message = Message::assistant().with_text(&decoded);
            message.id = Some(message_id.clone());
            yield (Some(message), None);

            // GENERATION LOOP: Generate remaining tokens
            let max_tokens: usize = 100;
            for index in 0..max_tokens.saturating_sub(1) {
                // Check for EOS tokens
                if next_token == loaded.eos_token_id || next_token == 128009 {
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

                // Decode and yield token
                let mut decoded = loaded
                    .tokenizer
                    .decode(&[next_token], false)
                    .map_err(|e| ProviderError::ExecutionError(format!("Failed to decode token: {}", e)))?;

                // Strip EOS tokens from this chunk
                for eos_str in template.eos_strings() {
                    decoded = decoded.replace(eos_str, "");
                }

                if !decoded.is_empty() {
                    let mut message = Message::assistant().with_text(&decoded);
                    message.id = Some(message_id.clone());
                    yield (Some(message), None);
                }
            }

            // Final yield with usage
            let usage = Usage::new(None, None, None);
            let provider_usage = ProviderUsage::new(model_name.clone(), usage);
            yield (None, Some(provider_usage));
        }))
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
