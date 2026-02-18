mod emulator;
pub mod hf_models;
mod inference_context;
pub mod local_model_registry;
mod native_path;
mod tool_parsing;

use emulator::{build_emulator_tool_description, load_tiny_model_prompt, run_emulator_path};
use inference_context::LoadedModel;
use native_path::run_native_tool_path;
use tool_parsing::compact_tools_json;

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
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::{list_llama_ggml_backend_devices, LlamaBackendDeviceType};
use rmcp::model::{RawContent, Role, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
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

type StreamSender =
    tokio::sync::mpsc::Sender<Result<(Option<Message>, Option<ProviderUsage>), ProviderError>>;

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

        if use_emulator && !tools.is_empty() {
            let tool_desc = build_emulator_tool_description(tools, code_mode_enabled);
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
