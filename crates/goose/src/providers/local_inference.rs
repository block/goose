pub mod hf_models;
pub mod local_model_registry;
mod tool_parsing;

use tool_parsing::{
    compact_tools_json, extract_tool_call_messages, extract_xml_tool_call_messages,
    safe_stream_end, split_content_and_tool_calls, split_content_and_xml_tool_calls,
};

use crate::config::paths::Paths;
use crate::config::ExtensionConfig;
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::{
    MessageStream, Provider, ProviderDef, ProviderMetadata, ProviderUsage, Usage,
};
use crate::providers::errors::ProviderError;
use crate::providers::formats::openai::format_tools;
use crate::providers::utils::RequestLog;
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::future::BoxFuture;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::openai::OpenAIChatTemplateParams;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::{list_llama_ggml_backend_devices, LlamaBackendDeviceType};
use rmcp::model::{CallToolRequestParams, RawContent, Role, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex, Weak};
use tokio::sync::Mutex;
use utoipa::ToSchema;
use uuid::Uuid;

const SHELL_TOOL: &str = "developer__shell";
const CODE_EXECUTION_TOOL: &str = "code_execution__execute";

type ModelSlot = Arc<Mutex<Option<LoadedModel>>>;

/// Owns the llama backend and all cached models. Field order matters:
/// `models` is declared before `backend` so Rust drops all loaded models
/// (and their Metal/GPU resources) before the backend calls
/// `llama_backend_free()`, avoiding the ggml-metal assertion on shutdown.
pub struct InferenceRuntime {
    models: StdMutex<HashMap<String, ModelSlot>>,
    backend: LlamaBackend,
}

/// Global weak reference used to share a single `InferenceRuntime` across
/// all providers and server routes. Only a `Weak` is stored — strong `Arc`s
/// live in providers and `AppState`. When all strong refs drop (normal
/// shutdown), the runtime is deallocated and the backend freed. The `Weak`
/// left behind is inert during `__cxa_finalize`, so no ggml statics race.
static RUNTIME: StdMutex<Weak<InferenceRuntime>> = StdMutex::new(Weak::new());

impl InferenceRuntime {
    pub fn get_or_init() -> Arc<Self> {
        let mut guard = RUNTIME.lock().expect("runtime lock poisoned");
        if let Some(runtime) = guard.upgrade() {
            return runtime;
        }
        let backend = match LlamaBackend::init() {
            Ok(b) => b,
            Err(llama_cpp_2::LlamaCppError::BackendAlreadyInitialized) => {
                panic!("LlamaBackend already initialized but Weak was dead — should be impossible")
            }
            Err(e) => panic!("Failed to init llama backend: {}", e),
        };
        let runtime = Arc::new(Self {
            models: StdMutex::new(HashMap::new()),
            backend,
        });
        *guard = Arc::downgrade(&runtime);
        runtime
    }

    pub fn backend(&self) -> &LlamaBackend {
        &self.backend
    }

    fn get_or_create_model_slot(&self, model_id: &str) -> ModelSlot {
        let mut map = self.models.lock().expect("model cache lock poisoned");
        map.entry(model_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    fn other_model_slots(&self, keep_model_id: &str) -> Vec<ModelSlot> {
        let map = self.models.lock().expect("model cache lock poisoned");
        map.iter()
            .filter(|(id, _)| id.as_str() != keep_model_id)
            .map(|(_, slot)| slot.clone())
            .collect()
    }
}

const PROVIDER_NAME: &str = "local";
const DEFAULT_MODEL: &str = "llama-3.2-1b";

pub const LOCAL_LLM_MODEL_CONFIG_KEY: &str = "LOCAL_LLM_MODEL";

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocalLlmModel {
    pub id: &'static str,
    pub name: &'static str,
    pub size_mb: u32,
    pub context_limit: usize,
    pub url: &'static str,
    pub description: &'static str,
    pub tier: ModelTier,
}

const LOCAL_LLM_MODELS: &[LocalLlmModel] = &[
    LocalLlmModel {
        id: "llama-3.2-1b",
        name: "Llama 3.2 1B Instruct",
        size_mb: 700,
        context_limit: 4096,
        url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
        description: "Fastest, CPU-optimized for quick responses",
        tier: ModelTier::Tiny,
    },
    LocalLlmModel {
        id: "llama-3.2-3b",
        name: "Llama 3.2 3B Instruct",
        size_mb: 2000,
        context_limit: 8192,
        url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        description: "Good balance of speed and quality for laptops",
        tier: ModelTier::Small,
    },
    LocalLlmModel {
        id: "hermes-2-pro-7b",
        name: "Hermes 2 Pro Llama-3 7B",
        size_mb: 4500,
        context_limit: 8192,
        url: "https://huggingface.co/NousResearch/Hermes-2-Pro-Llama-3-8B-GGUF/resolve/main/Hermes-2-Pro-Llama-3-8B-Q4_K_M.gguf",
        description: "High quality for desktops with GPU",
        tier: ModelTier::Medium,
    },
    LocalLlmModel {
        id: "mistral-small-22b",
        name: "Mistral Small 22B Instruct",
        size_mb: 13000,
        context_limit: 32768,
        url: "https://huggingface.co/bartowski/Mistral-Small-Instruct-2409-GGUF/resolve/main/Mistral-Small-Instruct-2409-Q4_K_M.gguf",
        description: "Highest quality with long context support",
        tier: ModelTier::Large,
    },
];

impl LocalLlmModel {
    pub fn local_path(&self) -> PathBuf {
        Paths::in_data_dir("models").join(format!("{}.gguf", self.id))
    }

    pub fn is_downloaded(&self) -> bool {
        self.local_path().exists()
    }
}

pub fn available_local_models() -> &'static [LocalLlmModel] {
    LOCAL_LLM_MODELS
}

pub fn get_local_model(id: &str) -> Option<&'static LocalLlmModel> {
    LOCAL_LLM_MODELS.iter().find(|m| m.id == id)
}

/// Resolve model path, context limit, and settings for any model ID — checks registry first,
/// then falls back to the hardcoded featured list.
pub fn resolve_model_path(
    model_id: &str,
) -> Option<(
    PathBuf,
    usize,
    crate::providers::local_inference::local_model_registry::ModelSettings,
)> {
    use crate::providers::local_inference::local_model_registry::get_registry;

    // Check registry first (covers both HF-downloaded and migrated legacy models)
    if let Ok(registry) = get_registry().lock() {
        if let Some(entry) = registry.get_model(model_id) {
            let ctx = entry.settings.context_size.unwrap_or(0) as usize;
            return Some((entry.local_path.clone(), ctx, entry.settings.clone()));
        }
    }

    // Fall back to hardcoded featured list
    get_local_model(model_id).map(|m| {
        (
            m.local_path(),
            m.context_limit,
            crate::providers::local_inference::local_model_registry::ModelSettings::default(),
        )
    })
}

pub fn available_inference_memory_bytes(runtime: &InferenceRuntime) -> u64 {
    let _ = &runtime.backend;
    let devices = list_llama_ggml_backend_devices();

    let accel_memory = devices
        .iter()
        .filter(|d| {
            matches!(
                d.device_type,
                LlamaBackendDeviceType::Gpu
                    | LlamaBackendDeviceType::IntegratedGpu
                    | LlamaBackendDeviceType::Accelerator
            )
        })
        .map(|d| d.memory_free as u64)
        .max()
        .unwrap_or(0);

    if accel_memory > 0 {
        accel_memory
    } else {
        devices
            .iter()
            .filter(|d| d.device_type == LlamaBackendDeviceType::Cpu)
            .map(|d| d.memory_free as u64)
            .max()
            .unwrap_or(0)
    }
}

pub fn recommend_local_model(runtime: &InferenceRuntime) -> &'static str {
    let effective_memory_mb = available_inference_memory_bytes(runtime) / (1024 * 1024);

    if effective_memory_mb >= 16_000 {
        "mistral-small-22b"
    } else if effective_memory_mb >= 6_000 {
        "hermes-2-pro-7b"
    } else if effective_memory_mb >= 3_000 {
        "llama-3.2-3b"
    } else {
        "llama-3.2-1b"
    }
}

struct LoadedModel {
    model: LlamaModel,
    template: LlamaChatTemplate,
}

enum EmulatorAction {
    Text(String),
    ShellCommand(String),
    ExecuteCode(String),
}

enum ParserState {
    Normal,
    InCommand,
    InExecuteBlock,
}

struct StreamingEmulatorParser {
    buffer: String,
    state: ParserState,
    code_mode_enabled: bool,
}

impl StreamingEmulatorParser {
    fn new(code_mode_enabled: bool) -> Self {
        Self {
            buffer: String::new(),
            state: ParserState::Normal,
            code_mode_enabled,
        }
    }

    fn process_chunk(&mut self, chunk: &str) -> Vec<EmulatorAction> {
        self.buffer.push_str(chunk);
        let mut results = Vec::new();

        loop {
            match self.state {
                ParserState::InCommand => {
                    if let Some((command_line, rest)) = self.buffer.split_once('\n') {
                        if let Some(command) = command_line.strip_prefix('$') {
                            let command = command.trim();
                            if !command.is_empty() {
                                results.push(EmulatorAction::ShellCommand(command.to_string()));
                            }
                        }
                        self.buffer = rest.to_string();
                        self.state = ParserState::Normal;
                    } else {
                        break;
                    }
                }
                ParserState::InExecuteBlock => {
                    // Look for closing ``` to end the execute block
                    if let Some(end_idx) = self.buffer.find("\n```") {
                        #[allow(clippy::string_slice)]
                        let code = self.buffer[..end_idx].to_string();
                        // Skip past the closing ``` and any trailing newline
                        #[allow(clippy::string_slice)]
                        let rest = &self.buffer[end_idx + 4..];
                        let rest = rest.strip_prefix('\n').unwrap_or(rest);
                        self.buffer = rest.to_string();
                        self.state = ParserState::Normal;
                        if !code.trim().is_empty() {
                            results.push(EmulatorAction::ExecuteCode(code));
                        }
                    } else {
                        // Still accumulating code — wait for closing fence
                        break;
                    }
                }
                ParserState::Normal => {
                    // Check for ```execute block (code mode)
                    if self.code_mode_enabled {
                        if let Some((before, after)) = self.buffer.split_once("```execute\n") {
                            if !before.trim().is_empty() {
                                results.push(EmulatorAction::Text(before.to_string()));
                            }
                            self.buffer = after.to_string();
                            self.state = ParserState::InExecuteBlock;
                            continue;
                        }
                        // Also handle without newline after tag (accumulating)
                        if self.buffer.ends_with("```execute") {
                            let before = self.buffer.trim_end_matches("```execute");
                            if !before.trim().is_empty() {
                                results.push(EmulatorAction::Text(before.to_string()));
                            }
                            self.buffer.clear();
                            self.state = ParserState::InExecuteBlock;
                            continue;
                        }
                    }

                    // Check for $ command
                    if let Some((before_dollar, from_dollar)) = self.buffer.split_once("\n$") {
                        let text = format!("{}\n", before_dollar);
                        if !text.trim().is_empty() {
                            results.push(EmulatorAction::Text(text));
                        }
                        self.buffer = format!("${}", from_dollar);
                        self.state = ParserState::InCommand;
                    } else if self.buffer.starts_with('$') && self.buffer.len() == chunk.len() {
                        self.state = ParserState::InCommand;
                    } else {
                        // Hold back a small tail in case it's the start of
                        // a ``` fence or a \n$ command prefix.
                        let hold_back = if self.code_mode_enabled { 12 } else { 2 };
                        let char_count = self.buffer.chars().count();
                        if char_count > hold_back && !self.buffer.ends_with('\n') {
                            let mut chars = self.buffer.chars();
                            let emit_count = char_count - hold_back;
                            let emit_text: String = chars.by_ref().take(emit_count).collect();
                            let keep_text: String = chars.collect();
                            if !emit_text.is_empty() {
                                results.push(EmulatorAction::Text(emit_text));
                            }
                            self.buffer = keep_text;
                        }
                        break;
                    }
                }
            }
        }

        results
    }

    fn flush(&mut self) -> Vec<EmulatorAction> {
        let mut results = Vec::new();

        if !self.buffer.is_empty() {
            match self.state {
                ParserState::InCommand => {
                    let command_line = self.buffer.trim();
                    if let Some(command) = command_line.strip_prefix('$') {
                        let command = command.trim();
                        if !command.is_empty() {
                            results.push(EmulatorAction::ShellCommand(command.to_string()));
                        }
                    } else if !command_line.is_empty() {
                        results.push(EmulatorAction::Text(self.buffer.clone()));
                    }
                }
                ParserState::InExecuteBlock => {
                    // Unterminated code block — execute what we have
                    let code = self.buffer.trim();
                    if !code.is_empty() {
                        results.push(EmulatorAction::ExecuteCode(code.to_string()));
                    }
                }
                ParserState::Normal => {
                    results.push(EmulatorAction::Text(self.buffer.clone()));
                }
            }
            self.buffer.clear();
            self.state = ParserState::Normal;
        }

        results
    }
}

fn build_openai_messages_json(system: &str, messages: &[Message]) -> String {
    let mut arr: Vec<Value> = vec![json!({"role": "system", "content": system})];

    for msg in messages {
        let role_str = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        // Collect text parts, tool calls (assistant), and tool results (user)
        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();
        let mut tool_results = Vec::new();

        for content in &msg.content {
            match content {
                MessageContent::Text(t) => {
                    if !t.text.trim().is_empty() {
                        text_parts.push(t.text.clone());
                    }
                }
                MessageContent::ToolRequest(req) => {
                    if let Ok(call) = &req.tool_call {
                        let args_str = call
                            .arguments
                            .as_ref()
                            .and_then(|a| serde_json::to_string(a).ok())
                            .unwrap_or_else(|| "{}".to_string());
                        tool_calls.push(json!({
                            "id": req.id,
                            "type": "function",
                            "function": {
                                "name": call.name,
                                "arguments": args_str,
                            }
                        }));
                    }
                }
                MessageContent::ToolResponse(resp) => {
                    let result_text = match &resp.tool_result {
                        Ok(result) => result
                            .content
                            .iter()
                            .filter_map(|c| match c.raw {
                                RawContent::Text(ref t) => Some(t.text.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                        Err(e) => format!("Error: {e}"),
                    };
                    tool_results.push((resp.id.clone(), result_text));
                }
                _ => {}
            }
        }

        // Emit assistant message: may have text content + tool_calls
        if role_str == "assistant" {
            if !tool_calls.is_empty() {
                let mut assistant_msg = json!({
                    "role": "assistant",
                    "tool_calls": tool_calls,
                });
                let text = text_parts.join("\n");
                if !text.is_empty() {
                    assistant_msg["content"] = Value::String(text);
                }
                arr.push(assistant_msg);
            } else {
                let text = text_parts.join("\n");
                if !text.is_empty() {
                    arr.push(json!({"role": "assistant", "content": text}));
                }
            }
        } else {
            // User messages: emit tool results as separate "tool" role messages,
            // and any text as a regular user message.
            let text = text_parts.join("\n");
            if !text.is_empty() {
                arr.push(json!({"role": "user", "content": text}));
            }
            for (tool_call_id, result_text) in tool_results {
                arr.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": result_text,
                }));
            }
        }
    }

    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

fn extract_text_content(msg: &Message) -> String {
    let mut parts = Vec::new();

    for content in &msg.content {
        match content {
            MessageContent::Text(text) => {
                parts.push(text.text.clone());
            }
            MessageContent::ToolRequest(req) => {
                if let Ok(call) = &req.tool_call {
                    if let Some(cmd) = call
                        .arguments
                        .as_ref()
                        .and_then(|a| a.get("command"))
                        .and_then(|v| v.as_str())
                    {
                        parts.push(format!("$ {}", cmd));
                    } else if let Some(code) = call
                        .arguments
                        .as_ref()
                        .and_then(|a| a.get("code"))
                        .and_then(|v| v.as_str())
                    {
                        parts.push(format!("```execute\n{}\n```", code));
                    }
                }
            }
            MessageContent::ToolResponse(response) => match &response.tool_result {
                Ok(result) => {
                    let mut output_parts = Vec::new();
                    for content_item in &result.content {
                        if let Some(text_content) = content_item.as_text() {
                            output_parts.push(text_content.text.to_string());
                        }
                    }
                    if !output_parts.is_empty() {
                        parts.push(format!("Command output:\n{}", output_parts.join("\n")));
                    }
                }
                Err(e) => {
                    parts.push(format!("Command error: {}", e));
                }
            },
            _ => {}
        }
    }

    parts.join("\n")
}

/// Build a compact JSON string of tools with only name and description
/// (no parameter schemas) to reduce token count for small context windows.
/// Estimate the maximum context length that can fit in available accelerator/CPU
/// memory based on the model's KV cache requirements.
///
/// Returns `None` if the model architecture values are unavailable.
fn estimate_max_context_for_memory(
    model: &LlamaModel,
    runtime: &InferenceRuntime,
) -> Option<usize> {
    let available = available_inference_memory_bytes(runtime);
    if available == 0 {
        return None;
    }

    // Reserve memory for computation scratch buffers (attention, etc.) and other overhead.
    // The compute buffer can be 40-50% of the KV cache size for large models, so we
    // conservatively use only half the available memory for the KV cache.
    let usable = (available as f64 * 0.5) as u64;

    let n_layer = model.n_layer() as u64;
    let n_head_kv = model.n_head_kv() as u64;
    let n_head = model.n_head() as u64;
    let n_embd = model.n_embd() as u64;

    if n_head == 0 || n_layer == 0 || n_head_kv == 0 || n_embd == 0 {
        return None;
    }

    // For MLA (Multi-head Latent Attention) models like DeepSeek/GLM, the actual KV cache
    // dimensions differ from n_head_kv * head_dim. Read the true dimensions from GGUF metadata.
    let arch = model
        .meta_val_str("general.architecture")
        .unwrap_or_default();
    let head_dim = n_embd / n_head;
    let k_per_head = model
        .meta_val_str(&format!("{arch}.attention.key_length"))
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(head_dim);
    let v_per_head = model
        .meta_val_str(&format!("{arch}.attention.value_length"))
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(head_dim);

    // Total KV dimensions across all KV heads, times n_layer, times 2 bytes (f16) per element
    let bytes_per_token = (k_per_head + v_per_head) * n_head_kv * n_layer * 2;

    if bytes_per_token == 0 {
        return None;
    }

    Some((usable / bytes_per_token) as usize)
}

fn effective_context_size(
    prompt_token_count: usize,
    context_limit: usize,
    n_ctx_train: usize,
    memory_max_ctx: Option<usize>,
) -> usize {
    let limit = if context_limit > 0 {
        context_limit
    } else {
        n_ctx_train
    };

    // Cap by estimated memory capacity when available.
    let limit = match memory_max_ctx {
        Some(mem_max) if mem_max < limit => {
            tracing::info!(
                "Capping context from {} to {} based on available memory",
                limit,
                mem_max,
            );
            mem_max
        }
        _ => limit,
    };

    let min_generation_headroom = 512;
    let needed = prompt_token_count + min_generation_headroom;
    if needed > limit {
        tracing::warn!(
            "Prompt ({} tokens) + headroom exceeds context limit ({}), capping to limit",
            prompt_token_count,
            limit,
        );
    }
    needed.min(limit)
}

fn build_context_params(
    ctx_size: u32,
    settings: &crate::providers::local_inference::local_model_registry::ModelSettings,
) -> LlamaContextParams {
    let mut params = LlamaContextParams::default().with_n_ctx(NonZeroU32::new(ctx_size));

    if let Some(n_batch) = settings.n_batch {
        params = params.with_n_batch(n_batch);
    }
    if let Some(n_threads) = settings.n_threads {
        params = params.with_n_threads(n_threads);
        params = params.with_n_threads_batch(n_threads);
    }
    if let Some(flash_attn) = settings.flash_attention {
        // llama_flash_attn_type: 0 = disabled, 1 = enabled
        let policy = if flash_attn { 1 } else { 0 };
        params = params.with_flash_attention_policy(policy);
    }

    params
}

fn build_sampler(
    settings: &crate::providers::local_inference::local_model_registry::ModelSettings,
) -> LlamaSampler {
    use crate::providers::local_inference::local_model_registry::SamplingConfig;

    let has_penalties = settings.repeat_penalty != 1.0
        || settings.frequency_penalty != 0.0
        || settings.presence_penalty != 0.0;

    let mut samplers: Vec<LlamaSampler> = Vec::new();

    if has_penalties {
        samplers.push(LlamaSampler::penalties(
            settings.repeat_last_n,
            settings.repeat_penalty,
            settings.frequency_penalty,
            settings.presence_penalty,
        ));
    }

    match &settings.sampling {
        SamplingConfig::Greedy => {
            samplers.push(LlamaSampler::greedy());
        }
        SamplingConfig::Temperature {
            temperature,
            top_k,
            top_p,
            min_p,
            seed,
        } => {
            samplers.push(LlamaSampler::top_k(*top_k));
            samplers.push(LlamaSampler::top_p(*top_p, 1));
            samplers.push(LlamaSampler::min_p(*min_p, 1));
            samplers.push(LlamaSampler::temp(*temperature));
            samplers.push(LlamaSampler::dist(seed.unwrap_or(0)));
        }
        SamplingConfig::MirostatV2 { tau, eta, seed } => {
            samplers.push(LlamaSampler::mirostat_v2(seed.unwrap_or(0), *tau, *eta));
        }
    }

    if samplers.len() == 1 {
        samplers.pop().unwrap()
    } else {
        LlamaSampler::chain_simple(samplers)
    }
}

/// Validate prompt tokens against memory limits and compute the effective
/// context size. Returns `(prompt_token_count, effective_ctx)`.
fn validate_and_compute_context(
    loaded: &LoadedModel,
    runtime: &InferenceRuntime,
    prompt_token_count: usize,
    context_limit: usize,
    settings: &crate::providers::local_inference::local_model_registry::ModelSettings,
) -> Result<(usize, usize), ProviderError> {
    let n_ctx_train = loaded.model.n_ctx_train() as usize;
    let memory_max_ctx = estimate_max_context_for_memory(&loaded.model, runtime);
    let effective_ctx = if let Some(ctx_size) = settings.context_size {
        ctx_size as usize
    } else {
        effective_context_size(
            prompt_token_count,
            context_limit,
            n_ctx_train,
            memory_max_ctx,
        )
    };
    if let Some(mem_max) = memory_max_ctx {
        if prompt_token_count > mem_max {
            return Err(ProviderError::ContextLengthExceeded(format!(
                "Prompt ({} tokens) exceeds estimated memory capacity ({} tokens). \
                 Try a smaller model or reduce conversation length.",
                prompt_token_count, mem_max,
            )));
        }
    }
    Ok((prompt_token_count, effective_ctx))
}

/// Create a llama context and prefill (decode) all prompt tokens.
fn create_and_prefill_context<'model>(
    loaded: &'model LoadedModel,
    runtime: &InferenceRuntime,
    tokens: &[llama_cpp_2::token::LlamaToken],
    effective_ctx: usize,
    settings: &crate::providers::local_inference::local_model_registry::ModelSettings,
) -> Result<llama_cpp_2::context::LlamaContext<'model>, ProviderError> {
    let ctx_params = build_context_params(effective_ctx as u32, settings);
    let mut ctx = loaded
        .model
        .new_context(runtime.backend(), ctx_params)
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to create context: {}", e)))?;

    let n_batch = ctx.n_batch() as usize;
    for chunk in tokens.chunks(n_batch) {
        let mut batch = LlamaBatch::get_one(chunk)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to create batch: {}", e)))?;
        ctx.decode(&mut batch)
            .map_err(|e| ProviderError::ExecutionError(format!("Prefill decode failed: {}", e)))?;
    }

    Ok(ctx)
}

/// Action to take after processing a generated token piece.
enum TokenAction {
    Continue,
    Stop,
}

/// Run the autoregressive generation loop. Calls `on_piece` for each non-empty
/// token piece. The callback returns `TokenAction::Stop` to break early.
/// Returns the total number of generated tokens.
fn generation_loop(
    model: &LlamaModel,
    ctx: &mut llama_cpp_2::context::LlamaContext<'_>,
    settings: &crate::providers::local_inference::local_model_registry::ModelSettings,
    prompt_token_count: usize,
    effective_ctx: usize,
    mut on_piece: impl FnMut(&str) -> Result<TokenAction, ProviderError>,
) -> Result<i32, ProviderError> {
    let mut sampler = build_sampler(settings);
    let max_output = if let Some(max) = settings.max_output_tokens {
        effective_ctx.saturating_sub(prompt_token_count).min(max)
    } else {
        effective_ctx.saturating_sub(prompt_token_count)
    };
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut output_token_count: i32 = 0;

    for _ in 0..max_output {
        let token = sampler.sample(ctx, -1);
        sampler.accept(token);

        if model.is_eog_token(token) {
            break;
        }

        output_token_count += 1;

        let piece = model
            .token_to_piece(token, &mut decoder, true, None)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to decode token: {}", e)))?;

        if !piece.is_empty() && matches!(on_piece(&piece)?, TokenAction::Stop) {
            break;
        }

        let next_tokens = [token];
        let mut next_batch = LlamaBatch::get_one(&next_tokens)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to create batch: {}", e)))?;
        ctx.decode(&mut next_batch)
            .map_err(|e| ProviderError::ExecutionError(format!("Decode failed: {}", e)))?;
    }

    Ok(output_token_count)
}

/// Build a `ProviderUsage` and write the request log entry.
fn finalize_usage(
    log: &mut RequestLog,
    model_name: String,
    path_label: &str,
    prompt_token_count: usize,
    output_token_count: i32,
    extra_log_fields: Option<(&str, &str)>,
) -> ProviderUsage {
    let input_tokens = prompt_token_count as i32;
    let total_tokens = input_tokens + output_token_count;
    let usage = Usage::new(
        Some(input_tokens),
        Some(output_token_count),
        Some(total_tokens),
    );
    let mut log_json = serde_json::json!({
        "path": path_label,
        "prompt_tokens": input_tokens,
        "output_tokens": output_token_count,
    });
    if let Some((key, value)) = extra_log_fields {
        log_json[key] = serde_json::json!(value);
    }
    let _ = log.write(&log_json, Some(&usage));
    ProviderUsage::new(model_name, usage)
}

/// Convert an `EmulatorAction` into a `Message` and send it through the
/// channel. Returns `Ok(true)` if it was a tool call, `Ok(false)` for text,
/// or `Err(())` if the channel is closed.
type StreamSender =
    tokio::sync::mpsc::Sender<Result<(Option<Message>, Option<ProviderUsage>), ProviderError>>;

fn send_emulator_action(
    action: &EmulatorAction,
    message_id: &str,
    tx: &StreamSender,
) -> Result<bool, ()> {
    match action {
        EmulatorAction::Text(text) => {
            let mut message = Message::assistant().with_text(text);
            message.id = Some(message_id.to_string());
            tx.blocking_send(Ok((Some(message), None)))
                .map_err(|_| ())?;
            Ok(false)
        }
        EmulatorAction::ShellCommand(command) => {
            let tool_id = Uuid::new_v4().to_string();
            let mut args = serde_json::Map::new();
            args.insert("command".to_string(), json!(command));
            let tool_call = CallToolRequestParams {
                meta: None,
                task: None,
                name: Cow::Owned(SHELL_TOOL.to_string()),
                arguments: Some(args),
            };
            let mut message = Message::assistant();
            message
                .content
                .push(MessageContent::tool_request(tool_id, Ok(tool_call)));
            message.id = Some(message_id.to_string());
            tx.blocking_send(Ok((Some(message), None)))
                .map_err(|_| ())?;
            Ok(true)
        }
        EmulatorAction::ExecuteCode(code) => {
            let tool_id = Uuid::new_v4().to_string();
            let wrapped = if code.contains("async function run()") {
                code.clone()
            } else {
                format!("async function run() {{\n{}\n}}", code)
            };
            let mut args = serde_json::Map::new();
            args.insert("code".to_string(), json!(wrapped));
            let tool_call = CallToolRequestParams {
                meta: None,
                task: None,
                name: Cow::Owned(CODE_EXECUTION_TOOL.to_string()),
                arguments: Some(args),
            };
            let mut message = Message::assistant();
            message
                .content
                .push(MessageContent::tool_request(tool_id, Ok(tool_call)));
            message.id = Some(message_id.to_string());
            tx.blocking_send(Ok((Some(message), None)))
                .map_err(|_| ())?;
            Ok(true)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_emulator_path(
    loaded: &LoadedModel,
    runtime: &InferenceRuntime,
    chat_messages: &[LlamaChatMessage],
    settings: &crate::providers::local_inference::local_model_registry::ModelSettings,
    context_limit: usize,
    code_mode_enabled: bool,
    model_name: String,
    message_id: &str,
    tx: &StreamSender,
    log: &mut RequestLog,
) -> Result<(), ProviderError> {
    let prompt = loaded
        .model
        .apply_chat_template(&loaded.template, chat_messages, true)
        .map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to apply chat template: {}", e))
        })?;

    let tokens = loaded
        .model
        .str_to_token(&prompt, AddBos::Never)
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to tokenize prompt: {}", e)))?;

    let (prompt_token_count, effective_ctx) =
        validate_and_compute_context(loaded, runtime, tokens.len(), context_limit, settings)?;
    let mut ctx = create_and_prefill_context(loaded, runtime, &tokens, effective_ctx, settings)?;

    let mut emulator_parser = StreamingEmulatorParser::new(code_mode_enabled);
    let mut tool_call_emitted = false;
    let mut send_failed = false;

    let output_token_count = generation_loop(
        &loaded.model,
        &mut ctx,
        settings,
        prompt_token_count,
        effective_ctx,
        |piece| {
            let actions = emulator_parser.process_chunk(piece);
            for action in actions {
                match send_emulator_action(&action, message_id, tx) {
                    Ok(is_tool) => {
                        if is_tool {
                            tool_call_emitted = true;
                        }
                    }
                    Err(_) => {
                        send_failed = true;
                        return Ok(TokenAction::Stop);
                    }
                }
            }
            if tool_call_emitted {
                Ok(TokenAction::Stop)
            } else {
                Ok(TokenAction::Continue)
            }
        },
    )?;

    if !send_failed {
        for action in emulator_parser.flush() {
            if send_emulator_action(&action, message_id, tx).is_err() {
                break;
            }
        }
    }

    let provider_usage = finalize_usage(
        log,
        model_name,
        "emulator",
        prompt_token_count,
        output_token_count,
        None,
    );
    let _ = tx.blocking_send(Ok((None, Some(provider_usage))));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_native_tool_path(
    loaded: &LoadedModel,
    runtime: &InferenceRuntime,
    chat_messages: &[LlamaChatMessage],
    oai_messages_json: &Option<String>,
    full_tools_json: Option<&str>,
    compact_tools: Option<&str>,
    settings: &crate::providers::local_inference::local_model_registry::ModelSettings,
    context_limit: usize,
    model_name: String,
    message_id: &str,
    tx: &StreamSender,
    log: &mut RequestLog,
) -> Result<(), ProviderError> {
    let min_generation_headroom = 512;
    let n_ctx_train = loaded.model.n_ctx_train() as usize;
    let memory_max_ctx = estimate_max_context_for_memory(&loaded.model, runtime);
    let context_cap = if let Some(ctx_size) = settings.context_size {
        ctx_size as usize
    } else {
        let base = if context_limit > 0 {
            context_limit
        } else {
            n_ctx_train
        };
        match memory_max_ctx {
            Some(mem_max) if mem_max < base => mem_max,
            _ => base,
        }
    };
    let token_budget = context_cap.saturating_sub(min_generation_headroom);

    let apply_template = |tools: Option<&str>| {
        if let Some(ref messages_json) = oai_messages_json {
            let params = OpenAIChatTemplateParams {
                messages_json: messages_json.as_str(),
                tools_json: tools,
                tool_choice: None,
                json_schema: None,
                grammar: None,
                reasoning_format: None,
                chat_template_kwargs: None,
                add_generation_prompt: true,
                use_jinja: true,
                parallel_tool_calls: false,
                enable_thinking: false,
                add_bos: false,
                add_eos: false,
                parse_tool_calls: true,
            };
            loaded
                .model
                .apply_chat_template_oaicompat(&loaded.template, &params)
        } else {
            loaded.model.apply_chat_template_with_tools_oaicompat(
                &loaded.template,
                chat_messages,
                tools,
                None,
                true,
            )
        }
    };

    let template_result = match apply_template(full_tools_json) {
        Ok(r) => {
            let token_count = loaded
                .model
                .str_to_token(&r.prompt, AddBos::Never)
                .map(|t| t.len())
                .unwrap_or(0);
            if token_count > token_budget {
                apply_template(compact_tools).unwrap_or(r)
            } else {
                r
            }
        }
        Err(_) => apply_template(compact_tools).map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to apply chat template: {}", e))
        })?,
    };

    let _ = log.write(
        &serde_json::json!({"applied_prompt": &template_result.prompt}),
        None,
    );

    let tokens = loaded
        .model
        .str_to_token(&template_result.prompt, AddBos::Never)
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to tokenize prompt: {}", e)))?;

    let (prompt_token_count, effective_ctx) =
        validate_and_compute_context(loaded, runtime, tokens.len(), context_limit, settings)?;
    let mut ctx = create_and_prefill_context(loaded, runtime, &tokens, effective_ctx, settings)?;

    let mut generated_text = String::new();
    let mut streamed_len: usize = 0;

    let output_token_count = generation_loop(
        &loaded.model,
        &mut ctx,
        settings,
        prompt_token_count,
        effective_ctx,
        |piece| {
            generated_text.push_str(piece);

            let has_xml_tc = split_content_and_xml_tool_calls(&generated_text).is_some();
            let (content, tc) = split_content_and_tool_calls(&generated_text);
            let stream_up_to = if tc.is_some() {
                content.len()
            } else if has_xml_tc {
                split_content_and_xml_tool_calls(&generated_text)
                    .map(|(c, _)| c.len())
                    .unwrap_or(0)
            } else {
                safe_stream_end(&generated_text)
            };
            if stream_up_to > streamed_len {
                #[allow(clippy::string_slice)]
                let new_text = &generated_text[streamed_len..stream_up_to];
                if !new_text.is_empty() {
                    let mut msg = Message::assistant().with_text(new_text);
                    msg.id = Some(message_id.to_string());
                    if tx.blocking_send(Ok((Some(msg), None))).is_err() {
                        return Ok(TokenAction::Stop);
                    }
                }
                streamed_len = stream_up_to;
            }

            let should_stop = template_result
                .additional_stops
                .iter()
                .any(|stop| generated_text.ends_with(stop));
            if should_stop {
                Ok(TokenAction::Stop)
            } else {
                Ok(TokenAction::Continue)
            }
        },
    )?;

    let (content, tool_call_msgs) =
        if let Some((xml_content, xml_calls)) = split_content_and_xml_tool_calls(&generated_text) {
            let msgs = extract_xml_tool_call_messages(xml_calls, message_id);
            (xml_content, msgs)
        } else {
            let (json_content, tool_calls_json) = split_content_and_tool_calls(&generated_text);
            let msgs = tool_calls_json
                .map(|tc| extract_tool_call_messages(&tc, message_id))
                .unwrap_or_default();
            (json_content, msgs)
        };

    if content.len() > streamed_len {
        #[allow(clippy::string_slice)]
        let remaining = &content[streamed_len..];
        if !remaining.is_empty() {
            let mut msg = Message::assistant().with_text(remaining);
            msg.id = Some(message_id.to_string());
            let _ = tx.blocking_send(Ok((Some(msg), None)));
        }
    }

    if !tool_call_msgs.is_empty() {
        for msg in tool_call_msgs {
            let _ = tx.blocking_send(Ok((Some(msg), None)));
        }
    } else if content.is_empty() && !generated_text.is_empty() {
        let mut msg = Message::assistant().with_text(&generated_text);
        msg.id = Some(message_id.to_string());
        let _ = tx.blocking_send(Ok((Some(msg), None)));
    }

    let provider_usage = finalize_usage(
        log,
        model_name,
        "native",
        prompt_token_count,
        output_token_count,
        Some(("generated_text", &generated_text)),
    );
    let _ = tx.blocking_send(Ok((None, Some(provider_usage))));
    Ok(())
}

pub struct LocalInferenceProvider {
    runtime: Arc<InferenceRuntime>,
    model: ModelSlot,
    model_config: ModelConfig,
    name: String,
}

impl LocalInferenceProvider {
    pub async fn from_env(model: ModelConfig, _extensions: Vec<ExtensionConfig>) -> Result<Self> {
        let runtime = InferenceRuntime::get_or_init();
        let model_slot = runtime.get_or_create_model_slot(&model.model_name);
        Ok(Self {
            runtime,
            model: model_slot,
            model_config: model,
            name: PROVIDER_NAME.to_string(),
        })
    }

    fn load_model_sync(
        runtime: &InferenceRuntime,
        model_id: &str,
        settings: &crate::providers::local_inference::local_model_registry::ModelSettings,
    ) -> Result<LoadedModel, ProviderError> {
        let (model_path, _context_limit, _) = resolve_model_path(model_id)
            .ok_or_else(|| ProviderError::ExecutionError(format!("Unknown model: {}", model_id)))?;

        if !model_path.exists() {
            return Err(ProviderError::ExecutionError(format!(
                "Model not downloaded: {}. Please download it from Settings > Local Inference.",
                model_id
            )));
        }

        tracing::info!("Loading {} from: {}", model_id, model_path.display());

        let backend = runtime.backend();

        let mut params = LlamaModelParams::default();
        if let Some(n_gpu_layers) = settings.n_gpu_layers {
            params = params.with_n_gpu_layers(n_gpu_layers);
        }
        if settings.use_mlock {
            params = params.with_use_mlock(true);
        }
        let model = LlamaModel::load_from_file(backend, &model_path, &params)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to load model: {}", e)))?;

        let template = match model.chat_template(None) {
            Ok(t) => t,
            Err(_) => {
                tracing::warn!("Model has no embedded chat template, falling back to chatml");
                LlamaChatTemplate::new("chatml").map_err(|e| {
                    ProviderError::ExecutionError(format!(
                        "Failed to create fallback chat template: {}",
                        e
                    ))
                })?
            }
        };

        tracing::info!("Model loaded successfully");

        Ok(LoadedModel { model, template })
    }
}

impl ProviderDef for LocalInferenceProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        use crate::providers::local_inference::local_model_registry::get_registry;

        let mut known_models: Vec<&str> = vec![
            "llama-3.2-1b",
            "llama-3.2-3b",
            "hermes-2-pro-7b",
            "mistral-small-22b",
        ];

        // Add any registry models not already in the featured list
        let mut dynamic_models = Vec::new();
        if let Ok(registry) = get_registry().lock() {
            for entry in registry.list_models() {
                if !known_models.contains(&entry.id.as_str()) {
                    dynamic_models.push(entry.id.clone());
                }
            }
        }
        let dynamic_refs: Vec<&str> = dynamic_models.iter().map(|s| s.as_str()).collect();
        known_models.extend(dynamic_refs);

        ProviderMetadata::new(
            PROVIDER_NAME,
            "Local Inference",
            "Local inference using quantized GGUF models (llama.cpp)",
            DEFAULT_MODEL,
            known_models,
            "https://github.com/utilityai/llama-cpp-rs",
            vec![],
        )
    }

    fn from_env(
        model: ModelConfig,
        extensions: Vec<ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>>
    where
        Self: Sized,
    {
        Box::pin(Self::from_env(model, extensions))
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

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        use crate::providers::local_inference::local_model_registry::get_registry;

        let mut all_models: Vec<String> = available_local_models()
            .iter()
            .map(|m| m.id.to_string())
            .collect();

        if let Ok(registry) = get_registry().lock() {
            for entry in registry.list_models() {
                if !all_models.contains(&entry.id) {
                    all_models.push(entry.id.clone());
                }
            }
        }

        Ok(all_models)
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        _session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let (_model_path, model_context_limit, model_settings) =
            resolve_model_path(&model_config.model_name).ok_or_else(|| {
                ProviderError::ExecutionError(format!(
                    "Model not found: {}",
                    model_config.model_name
                ))
            })?;

        // Ensure model is loaded — unload any other models first to free memory.
        {
            let mut model_lock = self.model.lock().await;
            if model_lock.is_none() {
                for slot in self.runtime.other_model_slots(&model_config.model_name) {
                    let mut other = slot.lock().await;
                    if other.is_some() {
                        tracing::info!("Unloading previous model to free memory");
                        *other = None;
                    }
                }

                let model_id = model_config.model_name.clone();
                let settings_for_load = model_settings.clone();
                let runtime_for_load = self.runtime.clone();
                let loaded = tokio::task::spawn_blocking(move || {
                    Self::load_model_sync(&runtime_for_load, &model_id, &settings_for_load)
                })
                .await
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Model load task failed: {}", e))
                })??;
                *model_lock = Some(loaded);
            }
        }

        // Models that support native OpenAI-compatible tool-call JSON use the
        // native path (template-based tool calling with JSON output). All other
        // models use the emulator which parses `$ command` and ```execute blocks.
        let use_emulator = !model_settings.native_tool_calling;
        let system_prompt = if use_emulator {
            load_tiny_model_prompt()
        } else {
            system.to_string()
        };

        // Build chat messages for the template
        let mut chat_messages =
            vec![
                LlamaChatMessage::new("system".to_string(), system_prompt.clone()).map_err(
                    |e| {
                        ProviderError::ExecutionError(format!(
                            "Failed to create system message: {}",
                            e
                        ))
                    },
                )?,
            ];

        // Check if Code Mode extension is available
        let code_mode_enabled = tools.iter().any(|t| t.name == CODE_EXECUTION_TOOL);

        // Append tool descriptions to system prompt
        if use_emulator && !tools.is_empty() {
            let mut tool_desc = String::new();

            if code_mode_enabled {
                // Build Code Mode instructions with available functions as
                // Namespace.functionName() — matching how Code Mode exposes them.
                tool_desc.push_str("\n\n# Running Code\n\n");
                tool_desc.push_str(
                    "You can call tools by writing code in a ```execute block. \
                     The code runs immediately — do not explain it, just run it.\n\n",
                );
                tool_desc.push_str("Example — counting files in /tmp:\n\n");
                tool_desc.push_str("```execute\nasync function run() {\n");
                tool_desc.push_str(
                    "  const result = await Developer.shell({ command: \"ls -1 /tmp | wc -l\" });\n",
                );
                tool_desc.push_str("  return result;\n}\n```\n\n");
                tool_desc.push_str("Rules:\n");
                tool_desc.push_str("- Code MUST define async function run() and return a result\n");
                tool_desc.push_str("- All function calls are async — use await\n");
                tool_desc
                    .push_str("- Use ```execute for tool calls, $ for simple shell one-liners\n\n");
                tool_desc.push_str("Available functions:\n\n");

                for tool in tools {
                    if tool.name.starts_with("code_execution__") {
                        continue;
                    }
                    let parts: Vec<&str> = tool.name.splitn(2, "__").collect();
                    if parts.len() == 2 {
                        let namespace = {
                            let mut c = parts[0].chars();
                            match c.next() {
                                None => String::new(),
                                Some(first) => first.to_uppercase().chain(c).collect::<String>(),
                            }
                        };
                        // Convert snake_case to camelCase
                        let camel_name: String = parts[1]
                            .split('_')
                            .enumerate()
                            .map(|(i, part)| {
                                if i == 0 {
                                    part.to_string()
                                } else {
                                    let mut c = part.chars();
                                    match c.next() {
                                        None => String::new(),
                                        Some(first) => first.to_uppercase().chain(c).collect(),
                                    }
                                }
                            })
                            .collect();
                        let desc = tool.description.as_ref().map(|d| d.as_ref()).unwrap_or("");
                        tool_desc.push_str(&format!("- {namespace}.{camel_name}(): {desc}\n"));
                    }
                }
            } else {
                tool_desc.push_str("\n\n# Tools\n\nYou have access to the following tools:\n\n");
                for tool in tools {
                    let desc = tool
                        .description
                        .as_ref()
                        .map(|d| d.as_ref())
                        .unwrap_or("No description");
                    tool_desc.push_str(&format!("- {}: {}\n", tool.name, desc));
                }
            }

            chat_messages = vec![LlamaChatMessage::new(
                "system".to_string(),
                format!("{}{}", system_prompt, tool_desc),
            )
            .map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to create system message: {}", e))
            })?];
        }

        for msg in messages {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };
            let content = extract_text_content(msg);
            if !content.trim().is_empty() {
                chat_messages.push(LlamaChatMessage::new(role.to_string(), content).map_err(
                    |e| ProviderError::ExecutionError(format!("Failed to create message: {}", e)),
                )?);
            }
        }

        let (full_tools_json, compact_tools) = if !use_emulator && !tools.is_empty() {
            let full = format_tools(tools)
                .ok()
                .and_then(|spec| serde_json::to_string(&spec).ok());
            let compact = compact_tools_json(tools);
            (full, compact)
        } else {
            (None, None)
        };

        let oai_messages_json = if model_settings.use_jinja {
            Some(build_openai_messages_json(&system_prompt, messages))
        } else {
            None
        };

        let model_arc = self.model.clone();
        let runtime = self.runtime.clone();
        let model_name = model_config.model_name.clone();
        let context_limit = model_context_limit;
        let settings = model_settings;

        let log_payload = serde_json::json!({
            "system": &system_prompt,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": match m.role { Role::User => "user", Role::Assistant => "assistant" },
                    "content": extract_text_content(m),
                })
            }).collect::<Vec<_>>(),
            "tools": tools.iter().map(|t| &t.name).collect::<Vec<_>>(),
            "settings": {
                "use_jinja": settings.use_jinja,
                "native_tool_calling": settings.native_tool_calling,
                "context_size": settings.context_size,
                "sampling": settings.sampling,
            },
        });

        let mut log = RequestLog::start(&self.model_config, &log_payload).map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to start request log: {e}"))
        })?;

        // Channel for streaming tokens from blocking thread to async stream
        let (tx, mut rx) = tokio::sync::mpsc::channel::<
            Result<(Option<Message>, Option<ProviderUsage>), ProviderError>,
        >(32);

        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();

            // Macro to log errors before sending them through the channel
            macro_rules! send_err {
                ($err:expr) => {{
                    let err = $err;
                    let msg = match &err {
                        ProviderError::ExecutionError(s) => s.as_str(),
                        ProviderError::ContextLengthExceeded(s) => s.as_str(),
                        _ => "unknown error",
                    };
                    let _ = log.error(msg);
                    let _ = tx.blocking_send(Err(err));
                    return;
                }};
            }

            let model_guard = rt.block_on(model_arc.lock());
            let loaded = match model_guard.as_ref() {
                Some(l) => l,
                None => {
                    send_err!(ProviderError::ExecutionError(
                        "Model not loaded".to_string()
                    ));
                }
            };

            let message_id = Uuid::new_v4().to_string();

            let result = if use_emulator {
                run_emulator_path(
                    loaded,
                    &runtime,
                    &chat_messages,
                    &settings,
                    context_limit,
                    code_mode_enabled,
                    model_name,
                    &message_id,
                    &tx,
                    &mut log,
                )
            } else {
                run_native_tool_path(
                    loaded,
                    &runtime,
                    &chat_messages,
                    &oai_messages_json,
                    full_tools_json.as_deref(),
                    compact_tools.as_deref(),
                    &settings,
                    context_limit,
                    model_name,
                    &message_id,
                    &tx,
                    &mut log,
                )
            };

            if let Err(err) = result {
                let msg = match &err {
                    ProviderError::ExecutionError(s) => s.as_str(),
                    ProviderError::ContextLengthExceeded(s) => s.as_str(),
                    _ => "unknown error",
                };
                let _ = log.error(msg);
                let _ = tx.blocking_send(Err(err));
            }
        });

        Ok(Box::pin(try_stream! {
            while let Some(result) = rx.recv().await {
                let item = result?;
                yield item;
            }

        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_context_size_basic() {
        assert_eq!(effective_context_size(100, 4096, 4096, None), 612);
    }

    #[test]
    fn test_effective_context_size_capped_by_limit() {
        assert_eq!(effective_context_size(100, 1024, 8192, None), 612);
    }

    #[test]
    fn test_effective_context_size_capped_by_memory() {
        assert_eq!(effective_context_size(100, 4096, 4096, Some(800)), 612);
    }

    #[test]
    fn test_effective_context_size_memory_smaller_than_needed() {
        assert_eq!(effective_context_size(600, 4096, 4096, Some(700)), 700);
    }

    #[test]
    fn test_effective_context_size_zero_limit_uses_train() {
        assert_eq!(effective_context_size(100, 0, 2048, None), 612);
    }

    #[test]
    fn test_effective_context_size_prompt_exceeds_all_limits() {
        assert_eq!(effective_context_size(5000, 4096, 4096, None), 4096);
    }
}
