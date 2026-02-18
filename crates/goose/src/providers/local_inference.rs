pub mod hf_models;
pub mod local_model_registry;

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
fn compact_tools_json(tools: &[Tool]) -> Option<String> {
    let compact: Vec<Value> = tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description.as_ref().map(|d| d.as_ref()).unwrap_or(""),
                }
            })
        })
        .collect();
    serde_json::to_string(&compact).ok()
}

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

/// Split generated text into (content, tool_calls_json).
/// Looks for the last top-level JSON object containing `"tool_calls"`.
/// Returns the text before it as content, and the JSON string if found.
#[allow(clippy::string_slice)]
fn split_content_and_tool_calls(text: &str) -> (String, Option<String>) {
    let trimmed = text.trim_end();
    if !trimmed.ends_with('}') {
        return (text.to_string(), None);
    }

    // Scan backwards for the matching '{' of the final '}'.
    // We only match on ASCII braces so `start` is always a char boundary.
    let bytes = trimmed.as_bytes();
    let mut depth = 0i32;
    let mut json_start = None;
    for i in (0..bytes.len()).rev() {
        match bytes[i] {
            b'}' => depth += 1,
            b'{' => {
                depth -= 1;
                if depth == 0 {
                    json_start = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    let Some(start) = json_start else {
        return (text.to_string(), None);
    };

    let json_str = &trimmed[start..];
    let parsed: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return (text.to_string(), None),
    };

    if parsed
        .get("tool_calls")
        .and_then(|v| v.as_array())
        .is_none()
    {
        return (text.to_string(), None);
    }

    let content = trimmed[..start].trim_end().to_string();
    (content, Some(json_str.to_string()))
}

/// Return the byte length of text that is safe to stream.
/// Everything before the last unmatched top-level `{` is safe — the `{` could
/// be the start of a tool-call JSON block still being generated.
/// If all braces are balanced the entire text is safe.
fn safe_stream_end(text: &str) -> usize {
    // Hold back from the start of any incomplete <tool_call> tag.
    // If we find an unmatched opening, nothing from that point should be streamed.
    let xml_hold = text.find("<tool_call>").unwrap_or(text.len());

    let bytes = text.as_bytes();
    let mut safe_end = bytes.len();
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'{' => {
                if depth == 0 {
                    safe_end = i;
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    safe_end = i + 1;
                }
            }
            _ => {
                if depth == 0 {
                    safe_end = i + 1;
                }
            }
        }
    }

    // Also hold back a partial `<tool_call` prefix at the end of the text.
    // The tag is 11 chars; if the last N chars are a prefix of `<tool_call>`, hold them.
    let tag = b"<tool_call>";
    let tail_hold = {
        let mut hold = safe_end;
        let check_len = tag.len().min(bytes.len());
        for start in (safe_end.saturating_sub(check_len))..safe_end {
            let tail = &bytes[start..safe_end];
            if tag.starts_with(tail) {
                hold = start;
                break;
            }
        }
        hold
    };

    safe_end.min(xml_hold).min(tail_hold)
}

/// Extract tool call messages from a JSON object containing "tool_calls".
/// Handles both the model's native format (name/arguments at top level)
/// and the OpenAI format (function.name/function.arguments).
fn extract_tool_call_messages(tool_calls_json: &str, message_id: &str) -> Vec<Message> {
    let parsed: Value = match serde_json::from_str(tool_calls_json) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let Some(tool_calls) = parsed.get("tool_calls").and_then(|v| v.as_array()) else {
        return vec![];
    };

    let mut messages = Vec::new();
    for tc in tool_calls {
        // Try OpenAI format first: {"function": {"name": ..., "arguments": ...}, "id": ...}
        // Then model's native format: {"name": ..., "arguments": {...}, "id": ...}
        let (name, arguments) = if let Some(func) = tc.get("function") {
            let n = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args_str = func
                .get("arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            let args: Option<serde_json::Map<String, Value>> = serde_json::from_str(args_str).ok();
            (n.to_string(), args)
        } else {
            let n = tc.get("name").and_then(|v| v.as_str()).unwrap_or("");
            // Arguments may be an object directly (model format) or a string (OAI format)
            let args = if let Some(obj) = tc.get("arguments").and_then(|v| v.as_object()) {
                Some(obj.clone())
            } else if let Some(s) = tc.get("arguments").and_then(|v| v.as_str()) {
                serde_json::from_str(s).ok()
            } else {
                None
            };
            (n.to_string(), args)
        };

        if name.is_empty() {
            continue;
        }

        let id = tc
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let tool_call = CallToolRequestParams {
            meta: None,
            task: None,
            name: Cow::Owned(name),
            arguments,
        };

        let mut msg = Message::assistant();
        msg.content
            .push(MessageContent::tool_request(id, Ok(tool_call)));
        msg.id = Some(message_id.to_string());
        messages.push(msg);
    }

    messages
}

/// Parse XML-style tool calls used by models like qwen3-coder.
/// Format:
/// ```text
/// <tool_call>
/// <function=tool_name>
/// <parameter=param1>value1</parameter>
/// <parameter=param2>value2</parameter>
/// </function>
/// </tool_call>
/// ```
/// Returns (content_before_tool_calls, vec_of_tool_calls) or None if no XML tool calls found.
#[allow(clippy::type_complexity)]
fn split_content_and_xml_tool_calls(
    text: &str,
) -> Option<(String, Vec<(String, serde_json::Map<String, Value>)>)> {
    let (content, first_block_and_rest) = text.split_once("<tool_call>")?;
    let content = content.trim_end().to_string();
    let mut tool_calls = Vec::new();

    // Process the first block, then keep splitting on subsequent <tool_call> tags
    let mut remaining = first_block_and_rest;
    loop {
        // Split off the block up to </tool_call> (or take the rest if unclosed)
        let (block, after_close) = remaining
            .split_once("</tool_call>")
            .unwrap_or((remaining, ""));

        if let Some(tool_call) = parse_single_xml_tool_call(block) {
            tool_calls.push(tool_call);
        }

        // Try to find the next <tool_call> in what remains
        match after_close.split_once("<tool_call>") {
            Some((_between, next_remaining)) => remaining = next_remaining,
            None => break,
        }
    }

    if tool_calls.is_empty() {
        None
    } else {
        Some((content, tool_calls))
    }
}

fn parse_single_xml_tool_call(block: &str) -> Option<(String, serde_json::Map<String, Value>)> {
    // Try <function=NAME><parameter=K>V</parameter>...</function> format first
    if let Some(result) = parse_xml_function_format(block) {
        return Some(result);
    }
    // Try GLM-style: TOOL_NAME<arg_key>K</arg_key><arg_value>V</arg_value>...
    parse_xml_arg_key_value_format(block)
}

fn parse_xml_function_format(block: &str) -> Option<(String, serde_json::Map<String, Value>)> {
    let (_, after_func_eq) = block.split_once("<function=")?;
    let (func_name, func_body) = after_func_eq.split_once('>')?;
    let func_name = func_name.trim().to_string();

    let mut args = serde_json::Map::new();
    let mut rest = func_body;

    while let Some((_, after_param_eq)) = rest.split_once("<parameter=") {
        let Some((param_name, after_name_close)) = after_param_eq.split_once('>') else {
            break;
        };
        let param_name = param_name.trim().to_string();

        let (value, after_value) = after_name_close
            .split_once("</parameter>")
            .unwrap_or((after_name_close, ""));
        let value = value.trim();

        let json_value =
            serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()));
        args.insert(param_name, json_value);

        rest = after_value;
    }

    Some((func_name, args))
}

/// Parse GLM-style tool calls: `NAME<arg_key>K</arg_key><arg_value>V</arg_value>...`
/// Also handles zero-argument calls like just `NAME`.
fn parse_xml_arg_key_value_format(block: &str) -> Option<(String, serde_json::Map<String, Value>)> {
    let func_name_end = block.find("<arg_key>").unwrap_or(block.len());
    // Safe: find returns a byte offset at the start of an ASCII '<' character,
    // and block.len() is always a valid boundary.
    #[allow(clippy::string_slice)]
    let func_name = block[..func_name_end].trim().to_string();
    if func_name.is_empty() {
        return None;
    }

    let mut args = serde_json::Map::new();
    #[allow(clippy::string_slice)]
    let mut rest = &block[func_name_end..];

    while let Some((_, after_key_open)) = rest.split_once("<arg_key>") {
        let Some((key, after_key_close)) = after_key_open.split_once("</arg_key>") else {
            break;
        };
        let key = key.trim().to_string();

        let Some((_, after_val_open)) = after_key_close.split_once("<arg_value>") else {
            break;
        };
        let (value, after_val_close) = after_val_open
            .split_once("</arg_value>")
            .unwrap_or((after_val_open, ""));
        let value = value.trim();

        let json_value =
            serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()));
        args.insert(key, json_value);

        rest = after_val_close;
    }

    Some((func_name, args))
}

fn extract_xml_tool_call_messages(
    tool_calls: Vec<(String, serde_json::Map<String, Value>)>,
    message_id: &str,
) -> Vec<Message> {
    tool_calls
        .into_iter()
        .map(|(name, args)| {
            let tool_call = CallToolRequestParams {
                meta: None,
                task: None,
                name: Cow::Owned(name),
                arguments: if args.is_empty() { None } else { Some(args) },
            };
            let mut msg = Message::assistant();
            msg.content.push(MessageContent::tool_request(
                Uuid::new_v4().to_string(),
                Ok(tool_call),
            ));
            msg.id = Some(message_id.to_string());
            msg
        })
        .collect()
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

            if use_emulator {
                // === Emulator path (Tiny/Small models) ===
                let prompt =
                    match loaded
                        .model
                        .apply_chat_template(&loaded.template, &chat_messages, true)
                    {
                        Ok(p) => p,
                        Err(e) => {
                            send_err!(ProviderError::ExecutionError(format!(
                                "Failed to apply chat template: {}",
                                e
                            )));
                        }
                    };

                let tokens = match loaded.model.str_to_token(&prompt, AddBos::Never) {
                    Ok(t) => t,
                    Err(e) => {
                        send_err!(ProviderError::ExecutionError(format!(
                            "Failed to tokenize prompt: {}",
                            e
                        )));
                    }
                };

                let (prompt_token_count, effective_ctx) = match validate_and_compute_context(
                    loaded,
                    &runtime,
                    tokens.len(),
                    context_limit,
                    &settings,
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        send_err!(e);
                    }
                };
                let mut ctx = match create_and_prefill_context(
                    loaded,
                    &runtime,
                    &tokens,
                    effective_ctx,
                    &settings,
                ) {
                    Ok(c) => c,
                    Err(e) => {
                        send_err!(e);
                    }
                };

                let mut emulator_parser = StreamingEmulatorParser::new(code_mode_enabled);
                let mut tool_call_emitted = false;
                let mut send_failed = false;

                let output_token_count = match generation_loop(
                    &loaded.model,
                    &mut ctx,
                    &settings,
                    prompt_token_count,
                    effective_ctx,
                    |piece| {
                        let actions = emulator_parser.process_chunk(piece);
                        for action in actions {
                            match send_emulator_action(&action, &message_id, &tx) {
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
                ) {
                    Ok(n) => n,
                    Err(e) => {
                        send_err!(e);
                    }
                };

                if !send_failed {
                    for action in emulator_parser.flush() {
                        match send_emulator_action(&action, &message_id, &tx) {
                            Ok(_) => {}
                            Err(_) => break,
                        }
                    }
                }

                let provider_usage = finalize_usage(
                    &mut log,
                    model_name,
                    "emulator",
                    prompt_token_count,
                    output_token_count,
                    None,
                );
                let _ = tx.blocking_send(Ok((None, Some(provider_usage))));
            } else {
                // === Native tool-calling path (Medium/Large models) ===
                let min_generation_headroom = 512;
                let n_ctx_train = loaded.model.n_ctx_train() as usize;
                let memory_max_ctx = estimate_max_context_for_memory(&loaded.model, &runtime);
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
                            &chat_messages,
                            tools,
                            None,
                            true,
                        )
                    }
                };

                let template_result = match apply_template(full_tools_json.as_deref()) {
                    Ok(r) => {
                        let token_count = loaded
                            .model
                            .str_to_token(&r.prompt, AddBos::Never)
                            .map(|t| t.len())
                            .unwrap_or(0);
                        if token_count > token_budget {
                            apply_template(compact_tools.as_deref()).unwrap_or(r)
                        } else {
                            r
                        }
                    }
                    Err(_) => match apply_template(compact_tools.as_deref()) {
                        Ok(r) => r,
                        Err(e) => {
                            send_err!(ProviderError::ExecutionError(format!(
                                "Failed to apply chat template: {}",
                                e
                            )));
                        }
                    },
                };

                let _ = log.write(
                    &serde_json::json!({"applied_prompt": &template_result.prompt}),
                    None,
                );

                let tokens = match loaded
                    .model
                    .str_to_token(&template_result.prompt, AddBos::Never)
                {
                    Ok(t) => t,
                    Err(e) => {
                        send_err!(ProviderError::ExecutionError(format!(
                            "Failed to tokenize prompt: {}",
                            e
                        )));
                    }
                };

                let (prompt_token_count, effective_ctx) = match validate_and_compute_context(
                    loaded,
                    &runtime,
                    tokens.len(),
                    context_limit,
                    &settings,
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        send_err!(e);
                    }
                };
                let mut ctx = match create_and_prefill_context(
                    loaded,
                    &runtime,
                    &tokens,
                    effective_ctx,
                    &settings,
                ) {
                    Ok(c) => c,
                    Err(e) => {
                        send_err!(e);
                    }
                };

                let mut generated_text = String::new();
                let mut streamed_len: usize = 0;

                let output_token_count = match generation_loop(
                    &loaded.model,
                    &mut ctx,
                    &settings,
                    prompt_token_count,
                    effective_ctx,
                    |piece| {
                        generated_text.push_str(piece);

                        let has_xml_tc =
                            split_content_and_xml_tool_calls(&generated_text).is_some();
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
                                msg.id = Some(message_id.clone());
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
                ) {
                    Ok(n) => n,
                    Err(e) => {
                        send_err!(e);
                    }
                };

                // Try XML tool call format first (e.g. qwen3-coder), then JSON
                let (content, tool_call_msgs) = if let Some((xml_content, xml_calls)) =
                    split_content_and_xml_tool_calls(&generated_text)
                {
                    let msgs = extract_xml_tool_call_messages(xml_calls, &message_id);
                    (xml_content, msgs)
                } else {
                    let (json_content, tool_calls_json) =
                        split_content_and_tool_calls(&generated_text);
                    let msgs = tool_calls_json
                        .map(|tc| extract_tool_call_messages(&tc, &message_id))
                        .unwrap_or_default();
                    (json_content, msgs)
                };

                if content.len() > streamed_len {
                    #[allow(clippy::string_slice)]
                    let remaining = &content[streamed_len..];
                    if !remaining.is_empty() {
                        let mut msg = Message::assistant().with_text(remaining);
                        msg.id = Some(message_id.clone());
                        let _ = tx.blocking_send(Ok((Some(msg), None)));
                    }
                }

                if !tool_call_msgs.is_empty() {
                    for msg in tool_call_msgs {
                        let _ = tx.blocking_send(Ok((Some(msg), None)));
                    }
                } else if content.is_empty() && !generated_text.is_empty() {
                    let mut msg = Message::assistant().with_text(&generated_text);
                    msg.id = Some(message_id.clone());
                    let _ = tx.blocking_send(Ok((Some(msg), None)));
                }

                let provider_usage = finalize_usage(
                    &mut log,
                    model_name,
                    "native",
                    prompt_token_count,
                    output_token_count,
                    Some(("generated_text", &generated_text)),
                );
                let _ = tx.blocking_send(Ok((None, Some(provider_usage))));
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
    fn test_parse_xml_tool_call_single() {
        let text = "I'll search for that.\n\n<tool_call>\n<function=search__files>\n<parameter=pattern>local.*inference</parameter>\n</function>\n</tool_call>";
        let result = split_content_and_xml_tool_calls(text);
        assert!(result.is_some());
        let (content, calls) = result.unwrap();
        assert_eq!(content, "I'll search for that.");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "search__files");
        assert_eq!(calls[0].1.get("pattern").unwrap(), "local.*inference");
    }

    #[test]
    fn test_parse_xml_tool_call_multiple_params() {
        let text = "<tool_call>\n<function=developer__shell>\n<parameter=command>ls -la</parameter>\n<parameter=timeout>30</parameter>\n</function>\n</tool_call>";
        let result = split_content_and_xml_tool_calls(text);
        assert!(result.is_some());
        let (content, calls) = result.unwrap();
        assert!(content.is_empty());
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, SHELL_TOOL);
        assert_eq!(calls[0].1.get("command").unwrap(), "ls -la");
        // 30 should be parsed as a number
        assert_eq!(calls[0].1.get("timeout").unwrap(), &json!(30));
    }

    #[test]
    fn test_parse_xml_tool_call_no_tool_call() {
        let text = "Just some regular text with no tool calls.";
        assert!(split_content_and_xml_tool_calls(text).is_none());
    }

    #[test]
    fn test_parse_xml_tool_call_multiple_calls() {
        let text = "Doing two things:\n<tool_call>\n<function=foo__bar>\n<parameter=x>1</parameter>\n</function>\n</tool_call>\n<tool_call>\n<function=baz__qux>\n<parameter=y>hello</parameter>\n</function>\n</tool_call>";
        let result = split_content_and_xml_tool_calls(text);
        assert!(result.is_some());
        let (content, calls) = result.unwrap();
        assert_eq!(content, "Doing two things:");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "foo__bar");
        assert_eq!(calls[1].0, "baz__qux");
    }

    #[test]
    fn test_parse_xml_tool_call_multiline_value() {
        let text = "<tool_call>\n<function=developer__write_file>\n<parameter=path>test.py</parameter>\n<parameter=content>def hello():\n    print(\"world\")</parameter>\n</function>\n</tool_call>";
        let result = split_content_and_xml_tool_calls(text);
        assert!(result.is_some());
        let (_content, calls) = result.unwrap();
        assert_eq!(calls[0].0, "developer__write_file");
        assert_eq!(
            calls[0].1.get("content").unwrap(),
            "def hello():\n    print(\"world\")"
        );
    }

    #[test]
    fn test_safe_stream_end_holds_back_tool_call_tag() {
        let text = "Some text before <tool_call>\n<function=foo>";
        let safe = safe_stream_end(text);
        assert!(safe <= text.find("<tool_call>").unwrap());
    }

    #[test]
    fn test_safe_stream_end_holds_back_partial_tag() {
        let text = "Some text <tool_ca";
        let safe = safe_stream_end(text);
        // Should hold back the partial tag
        assert!(safe <= text.find('<').unwrap());
    }

    #[test]
    fn test_parse_glm_style_tool_call() {
        let text = "<tool_call>developer__shell<arg_key>command</arg_key><arg_value>ls -la</arg_value></tool_call>";
        let result = split_content_and_xml_tool_calls(text);
        assert!(result.is_some());
        let (content, calls) = result.unwrap();
        assert!(content.is_empty());
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, SHELL_TOOL);
        assert_eq!(calls[0].1.get("command").unwrap(), "ls -la");
    }

    #[test]
    fn test_parse_glm_style_tool_call_no_args() {
        let text = "Some text\n<tool_call>load</tool_call>";
        let result = split_content_and_xml_tool_calls(text);
        assert!(result.is_some());
        let (content, calls) = result.unwrap();
        assert_eq!(content, "Some text");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "load");
        assert!(calls[0].1.is_empty());
    }

    #[test]
    fn test_parse_glm_style_tool_call_multiple_args() {
        let text = "Let me check.\n<tool_call>execute<arg_key>code</arg_key><arg_value>async function run() { return 1; }</arg_value><arg_key>tool_graph</arg_key><arg_value>[{\"tool\": \"shell\"}]</arg_value></tool_call>";
        let result = split_content_and_xml_tool_calls(text);
        assert!(result.is_some());
        let (content, calls) = result.unwrap();
        assert_eq!(content, "Let me check.");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "execute");
        assert_eq!(
            calls[0].1.get("code").unwrap(),
            "async function run() { return 1; }"
        );
        // tool_graph should be parsed as JSON array
        assert!(calls[0].1.get("tool_graph").unwrap().is_array());
    }

    #[test]
    fn test_extract_xml_tool_call_messages() {
        let calls = vec![(
            SHELL_TOOL.to_string(),
            serde_json::Map::from_iter(vec![("command".to_string(), json!("ls"))]),
        )];
        let msgs = extract_xml_tool_call_messages(calls, "test-id");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].id, Some("test-id".to_string()));
        match &msgs[0].content[0] {
            MessageContent::ToolRequest(req) => {
                let call = req.tool_call.as_ref().unwrap();
                assert_eq!(&*call.name, SHELL_TOOL);
                assert_eq!(
                    call.arguments.as_ref().unwrap().get("command").unwrap(),
                    "ls"
                );
            }
            _ => panic!("Expected ToolRequest"),
        }
    }

    // --- effective_context_size tests ---

    #[test]
    fn test_effective_context_size_basic() {
        // prompt(100) + headroom(512) = 612, well within limits
        assert_eq!(effective_context_size(100, 4096, 4096, None), 612);
    }

    #[test]
    fn test_effective_context_size_capped_by_limit() {
        assert_eq!(effective_context_size(100, 1024, 8192, None), 612);
    }

    #[test]
    fn test_effective_context_size_capped_by_memory() {
        // memory_max_ctx(800) < context_limit(4096), but needed(612) < 800
        assert_eq!(effective_context_size(100, 4096, 4096, Some(800)), 612);
    }

    #[test]
    fn test_effective_context_size_memory_smaller_than_needed() {
        // needed = 600+512 = 1112, but memory cap is 700 → capped to 700
        assert_eq!(effective_context_size(600, 4096, 4096, Some(700)), 700);
    }

    #[test]
    fn test_effective_context_size_zero_limit_uses_train() {
        assert_eq!(effective_context_size(100, 0, 2048, None), 612);
    }

    #[test]
    fn test_effective_context_size_prompt_exceeds_all_limits() {
        // needed = 5000+512 = 5512 > limit(4096) → capped to 4096
        assert_eq!(effective_context_size(5000, 4096, 4096, None), 4096);
    }

    // --- split_content_and_tool_calls tests ---

    #[test]
    fn test_split_content_and_tool_calls_with_tool() {
        let text = "Here is the result.\n{\"tool_calls\": [{\"function\": {\"name\": \"shell\", \"arguments\": \"{}\"}, \"id\": \"abc\"}]}";
        let (content, tc) = split_content_and_tool_calls(text);
        assert_eq!(content, "Here is the result.");
        assert!(tc.is_some());
        let parsed: Value = serde_json::from_str(&tc.unwrap()).unwrap();
        assert_eq!(parsed["tool_calls"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_split_content_and_tool_calls_no_tool() {
        let text = "Just regular text, no JSON.";
        let (content, tc) = split_content_and_tool_calls(text);
        assert_eq!(content, text);
        assert!(tc.is_none());
    }

    #[test]
    fn test_split_content_and_tool_calls_json_without_tool_calls_key() {
        let text = "{\"key\": \"value\"}";
        let (content, tc) = split_content_and_tool_calls(text);
        assert_eq!(content, text);
        assert!(tc.is_none());
    }

    // --- extract_tool_call_messages tests ---

    #[test]
    fn test_extract_tool_call_messages_openai_format() {
        let json = r#"{"tool_calls": [{"function": {"name": "developer__shell", "arguments": "{\"command\": \"ls\"}"}, "id": "call-1"}]}"#;
        let msgs = extract_tool_call_messages(json, "msg-1");
        assert_eq!(msgs.len(), 1);
        match &msgs[0].content[0] {
            MessageContent::ToolRequest(req) => {
                let call = req.tool_call.as_ref().unwrap();
                assert_eq!(&*call.name, SHELL_TOOL);
                assert_eq!(
                    call.arguments.as_ref().unwrap().get("command").unwrap(),
                    "ls"
                );
            }
            _ => panic!("Expected ToolRequest"),
        }
    }

    #[test]
    fn test_extract_tool_call_messages_native_format() {
        let json = r#"{"tool_calls": [{"name": "developer__shell", "arguments": {"command": "ls"}, "id": "call-2"}]}"#;
        let msgs = extract_tool_call_messages(json, "msg-2");
        assert_eq!(msgs.len(), 1);
        match &msgs[0].content[0] {
            MessageContent::ToolRequest(req) => {
                let call = req.tool_call.as_ref().unwrap();
                assert_eq!(&*call.name, SHELL_TOOL);
            }
            _ => panic!("Expected ToolRequest"),
        }
    }

    #[test]
    fn test_extract_tool_call_messages_invalid_json() {
        assert!(extract_tool_call_messages("not json", "msg-3").is_empty());
    }

    #[test]
    fn test_extract_tool_call_messages_empty_name_skipped() {
        let json = r#"{"tool_calls": [{"name": "", "arguments": {}, "id": "x"}]}"#;
        assert!(extract_tool_call_messages(json, "msg-4").is_empty());
    }

    // --- compact_tools_json tests ---

    #[test]
    fn test_compact_tools_json_produces_minimal_output() {
        use rmcp::model::Tool;
        use rmcp::object;

        let tools = vec![Tool::new(
            "developer__shell".to_string(),
            "Run shell commands".to_string(),
            object!({"type": "object", "properties": {"command": {"type": "string"}}}),
        )];
        let result = compact_tools_json(&tools);
        assert!(result.is_some());
        let parsed: Vec<Value> = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed.len(), 1);
        let func = &parsed[0]["function"];
        assert_eq!(func["name"], "developer__shell");
        assert_eq!(func["description"], "Run shell commands");
        // Should not contain full parameter schemas
        assert!(func.get("parameters").is_none());
    }

    #[test]
    fn test_compact_tools_json_empty() {
        let result = compact_tools_json(&[]);
        assert!(result.is_some());
        let parsed: Vec<Value> = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(parsed.is_empty());
    }

    // --- safe_stream_end additional tests ---

    #[test]
    fn test_safe_stream_end_balanced_braces() {
        let text = "Result: {\"key\": \"value\"} done";
        assert_eq!(safe_stream_end(text), text.len());
    }

    #[test]
    fn test_safe_stream_end_unbalanced_open_brace() {
        let text = "Some text {\"tool_calls\": [";
        assert_eq!(safe_stream_end(text), "Some text ".len());
    }

    #[test]
    fn test_safe_stream_end_empty() {
        assert_eq!(safe_stream_end(""), 0);
    }

    #[test]
    fn test_safe_stream_end_no_braces() {
        let text = "plain text here";
        assert_eq!(safe_stream_end(text), text.len());
    }
}
