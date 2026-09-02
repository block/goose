pub mod config_resolver;
pub use goose_download_manager as download_manager;
pub mod huggingface_auth;
pub mod paths;
#[cfg(feature = "llamacpp")]
pub mod prompt_template;
pub mod provider_utils;

mod backend;
#[cfg(feature = "eredu")]
mod eredu;
pub mod hf_models;
#[cfg(feature = "llamacpp")]
mod llamacpp;
pub mod local_model_registry;
pub mod management;
#[cfg(feature = "llamacpp")]
pub(crate) mod multimodal;
#[cfg(feature = "llamacpp")]
pub(crate) mod thinking_output;
#[cfg(feature = "llamacpp")]
mod tool_emulation;
#[cfg(feature = "llamacpp")]
mod tool_parsing;

use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use backend::{BackendLoadedModel, LocalInferenceBackend};
#[cfg(feature = "eredu")]
use eredu::{EreduBackend, EREDU_BACKEND_ID};
use goose_provider_types::base::{MessageStream, Provider, ProviderDescriptor, ProviderMetadata};
use goose_provider_types::conversation::message::{
    Message, MessageContent, SystemNotificationType,
};
use goose_provider_types::conversation::token_usage::{ProviderUsage, Usage};
use goose_provider_types::errors::ProviderError;
#[cfg(feature = "llamacpp")]
use goose_provider_types::images::ImageFormat;
use goose_provider_types::model::ModelConfig;
use goose_provider_types::request_log::{start_log, LoggerHandleExt, RequestLogHandle};
#[cfg(feature = "llamacpp")]
use llamacpp::{LlamaCppBackend, LLAMACPP_BACKEND_ID};
use local_model_registry::{ChatTemplate, InferenceBackend};
use rmcp::model::Tool;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex, Weak};
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

type ModelSlotHandle = Arc<ModelSlot>;

struct ModelSlot {
    state: Mutex<ModelSlotState>,
    notify: Notify,
}

enum ModelSlotState {
    Empty,
    Loading,
    Loaded(Box<dyn BackendLoadedModel>),
}

impl ModelSlot {
    fn new() -> Self {
        Self {
            state: Mutex::new(ModelSlotState::Empty),
            notify: Notify::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ModelCacheKey {
    backend_id: &'static str,
    model_id: String,
    chat_template: ChatTemplate,
    draft_model: Option<String>,
}

impl ModelCacheKey {
    fn new(
        backend_id: &'static str,
        model_id: impl Into<String>,
        chat_template: ChatTemplate,
        draft_model: Option<String>,
    ) -> Self {
        Self {
            backend_id,
            model_id: model_id.into(),
            chat_template,
            draft_model,
        }
    }
}

pub struct InferenceRuntime {
    models: StdMutex<HashMap<ModelCacheKey, ModelSlotHandle>>,
    cold_load_lock: Mutex<()>,
    backends: HashMap<&'static str, Arc<dyn LocalInferenceBackend>>,
}

pub fn builtin_chat_template_names() -> Vec<String> {
    #[cfg(feature = "llamacpp")]
    let names = llamacpp::builtin_chat_template_names();
    #[cfg(not(feature = "llamacpp"))]
    let names = Vec::new();
    names
}

static RUNTIME: StdMutex<Weak<InferenceRuntime>> = StdMutex::new(Weak::new());

fn current_runtime() -> Option<Arc<InferenceRuntime>> {
    RUNTIME.lock().expect("runtime lock poisoned").upgrade()
}

impl InferenceRuntime {
    pub fn get_or_init() -> Result<Arc<Self>> {
        let mut guard = RUNTIME.lock().expect("runtime lock poisoned");
        if let Some(runtime) = guard.upgrade() {
            return Ok(runtime);
        }
        let mut backends = HashMap::new();
        #[cfg(feature = "llamacpp")]
        backends.insert(
            LLAMACPP_BACKEND_ID,
            Arc::new(LlamaCppBackend::new()?) as Arc<dyn LocalInferenceBackend>,
        );
        #[cfg(feature = "eredu")]
        backends.insert(
            EREDU_BACKEND_ID,
            Arc::new(EreduBackend::new()) as Arc<dyn LocalInferenceBackend>,
        );
        if backends.is_empty() {
            anyhow::bail!(
                "Local inference has no backend; enable the `llamacpp` or `eredu` crate feature"
            );
        }
        let runtime = Arc::new(Self {
            models: StdMutex::new(HashMap::new()),
            cold_load_lock: Mutex::new(()),
            backends,
        });
        *guard = Arc::downgrade(&runtime);
        Ok(runtime)
    }

    fn default_backend(&self) -> &dyn LocalInferenceBackend {
        #[cfg(feature = "llamacpp")]
        if let Some(backend) = self.backends.get(LLAMACPP_BACKEND_ID) {
            return backend.as_ref();
        }
        #[cfg(feature = "eredu")]
        if let Some(backend) = self.backends.get(EREDU_BACKEND_ID) {
            return backend.as_ref();
        }
        self.backends
            .values()
            .next()
            .expect("default local inference backend registered")
            .as_ref()
    }

    fn backend_for_model(
        &self,
        resolved: &ResolvedModelPaths,
    ) -> Result<Arc<dyn LocalInferenceBackend>, ProviderError> {
        let backend = select_backend(
            resolved.format,
            resolved.settings.backend_id,
            self.backends.contains_key("llamacpp"),
            self.backends.contains_key("eredu"),
        )?;
        let backend_id = backend.id();
        self.backends.get(backend_id).cloned().ok_or_else(|| {
            ProviderError::ExecutionError(format!(
                "Local inference backend '{}' unavailable",
                backend_id
            ))
        })
    }

    fn get_or_create_model_slot(&self, key: ModelCacheKey) -> ModelSlotHandle {
        let mut map = self.models.lock().expect("model cache lock poisoned");
        map.entry(key)
            .or_insert_with(|| Arc::new(ModelSlot::new()))
            .clone()
    }

    fn model_slot(&self, key: &ModelCacheKey) -> Option<ModelSlotHandle> {
        let map = self.models.lock().expect("model cache lock poisoned");
        map.get(key).cloned()
    }

    fn other_model_slots(&self, keep_key: &ModelCacheKey) -> Vec<ModelSlotHandle> {
        let map = self.models.lock().expect("model cache lock poisoned");
        map.iter()
            .filter(|(key, _)| *key != keep_key)
            .map(|(_, slot)| slot.clone())
            .collect()
    }
}

fn select_backend(
    format: ModelFormat,
    configured: Option<InferenceBackend>,
    llamacpp_available: bool,
    eredu_available: bool,
) -> Result<InferenceBackend, ProviderError> {
    let selected = configured.unwrap_or(match format {
        ModelFormat::Safetensors => InferenceBackend::Eredu,
        ModelFormat::Gguf if llamacpp_available => InferenceBackend::LlamaCpp,
        ModelFormat::Gguf => InferenceBackend::Eredu,
    });
    if format == ModelFormat::Safetensors && selected == InferenceBackend::LlamaCpp {
        return Err(ProviderError::ExecutionError(
            "llama.cpp cannot load SafeTensors models; select Eredu or Auto".to_string(),
        ));
    }
    let available = match selected {
        InferenceBackend::LlamaCpp => llamacpp_available,
        InferenceBackend::Eredu => eredu_available,
    };
    if !available {
        return Err(ProviderError::ExecutionError(format!(
            "Local inference backend '{}' unavailable",
            selected.id()
        )));
    }
    Ok(selected)
}

pub async fn is_model_loaded(model_name: &str) -> Result<bool, ProviderError> {
    let resolved = match resolve_model_path(model_name) {
        Some(resolved) => resolved,
        None => return Ok(false),
    };
    let runtime = InferenceRuntime::get_or_init().map_err(|error| {
        ProviderError::ExecutionError(format!("Failed to initialize local inference: {error}"))
    })?;
    let backend = runtime.backend_for_model(&resolved)?;
    let key = ModelCacheKey::new(
        backend.id(),
        model_name.to_string(),
        resolved.settings.chat_template,
        resolved.settings.draft_model,
    );
    let Some(slot) = runtime.model_slot(&key) else {
        return Ok(false);
    };

    let state = slot.state.lock().await;
    Ok(matches!(*state, ModelSlotState::Loaded(_)))
}

pub async fn loaded_model_ids() -> Result<HashSet<String>, ProviderError> {
    let Some(runtime) = current_runtime() else {
        return Ok(HashSet::new());
    };
    let slots = {
        let map = runtime.models.lock().expect("model cache lock poisoned");
        map.iter()
            .map(|(key, slot)| (key.model_id.clone(), slot.clone()))
            .collect::<Vec<_>>()
    };

    let mut loaded = HashSet::new();
    for (model_id, slot) in slots {
        if let Ok(state) = slot.state.try_lock() {
            if matches!(*state, ModelSlotState::Loaded(_)) {
                loaded.insert(model_id);
            }
        } else {
            loaded.insert(model_id);
        }
    }
    Ok(loaded)
}

pub async fn evict_model(model_name: &str) -> Result<bool, ProviderError> {
    let Some(runtime) = current_runtime() else {
        return Ok(false);
    };
    let slots = {
        let map = runtime.models.lock().expect("model cache lock poisoned");
        map.iter()
            .filter(|(key, _)| key.model_id == model_name)
            .map(|(_, slot)| slot.clone())
            .collect::<Vec<_>>()
    };

    let mut evicted = false;
    for slot in slots {
        let mut state = slot.state.lock().await;
        if matches!(*state, ModelSlotState::Loaded(_)) {
            *state = ModelSlotState::Empty;
            evicted = true;
            slot.notify.notify_waiters();
        }
    }
    Ok(evicted)
}

const PROVIDER_NAME: &str = "local";
const DEFAULT_MODEL: &str = "bartowski/Llama-3.2-1B-Instruct-GGUF:Q4_K_M";

pub const LOCAL_LLM_MODEL_CONFIG_KEY: &str = "LOCAL_LLM_MODEL";

#[derive(Clone)]
pub(crate) struct ResolvedModelPaths {
    pub model_path: PathBuf,
    pub context_limit: usize,
    pub settings: crate::local_model_registry::ModelSettings,
    #[cfg_attr(not(feature = "llamacpp"), allow(dead_code))]
    pub mmproj_path: Option<PathBuf>,
    pub format: ModelFormat,
    #[cfg_attr(not(feature = "eredu"), allow(dead_code))]
    pub draft_model_path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ModelFormat {
    Gguf,
    Safetensors,
}

fn resolve_model_local_path(model_id: &str) -> Option<PathBuf> {
    use crate::local_model_registry::get_registry;

    get_registry()
        .lock()
        .ok()?
        .get_model(model_id)
        .map(|entry| entry.local_path.clone())
}

pub fn local_context_limit(model_id: &str) -> Option<usize> {
    resolve_model_path(model_id)
        .map(|resolved| resolved.context_limit)
        .filter(|limit| *limit > 0)
}

/// Resolve model path, context limit, settings, and mmproj path for a model ID from the registry.
fn resolve_model_path(model_id: &str) -> Option<ResolvedModelPaths> {
    use crate::local_model_registry::{default_settings_for_model, get_registry};

    if let Ok(registry) = get_registry().lock() {
        if let Some(entry) = registry.get_model(model_id) {
            let ctx = entry.settings.context_size.unwrap_or(0) as usize;
            let mut settings = entry.settings.clone();
            let defaults = default_settings_for_model(model_id);
            settings.vision_capable = defaults.vision_capable;
            settings.mmproj_size_bytes = entry.mmproj_size_bytes;
            let mmproj_path = entry.mmproj_path.as_ref().filter(|p| p.exists()).cloned();
            let format = if entry.filename.ends_with(".gguf")
                || entry.local_path.extension().and_then(|ext| ext.to_str()) == Some("gguf")
            {
                ModelFormat::Gguf
            } else {
                ModelFormat::Safetensors
            };
            let draft_model = settings
                .draft_model
                .clone()
                .or_else(|| {
                    config_resolver::string_param("GOOSE_LOCAL_DRAFT_MODEL")
                        .ok()
                        .flatten()
                })
                .filter(|draft_model| draft_model != model_id);
            let draft_model_path = draft_model.as_deref().and_then(resolve_model_local_path);
            return Some(ResolvedModelPaths {
                model_path: entry.local_path.clone(),
                context_limit: ctx,
                settings,
                mmproj_path,
                format,
                draft_model_path,
            });
        }
    }

    None
}

pub fn available_inference_memory_bytes(runtime: &InferenceRuntime) -> u64 {
    runtime.default_backend().available_memory_bytes()
}

pub fn recommend_local_model(runtime: &InferenceRuntime) -> String {
    use local_model_registry::{get_registry, is_featured_model, FEATURED_MODELS};

    let available_memory = available_inference_memory_bytes(runtime);

    if let Ok(registry) = get_registry().lock() {
        let mut models: Vec<_> = registry
            .list_models()
            .iter()
            .filter(|m| is_featured_model(&m.id) && m.file_size() > 0)
            .collect();
        models.sort_by_key(|model| std::cmp::Reverse(model.file_size()));

        // Return largest that fits in available memory
        for model in &models {
            if available_memory >= model.file_size() {
                return model.id.clone();
            }
        }

        // If nothing fits, return smallest
        if let Some(smallest) = models.last() {
            return smallest.id.clone();
        }
    }

    // Fallback to first featured model
    FEATURED_MODELS[0].spec.to_string()
}

#[cfg(feature = "llamacpp")]
fn build_openai_messages_json(
    system: &str,
    messages: &[Message],
    media_marker: Option<&str>,
) -> String {
    use goose_provider_types::formats::openai::format_messages;

    let mut arr: Vec<Value> = vec![json!({"role": "system", "content": system})];
    arr.extend(format_messages(messages, &ImageFormat::OpenAi));
    strip_image_parts_from_messages(&mut arr);
    if let Some(marker) = media_marker {
        convert_text_media_markers(&mut arr, marker);
    }
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(feature = "llamacpp")]
fn build_openai_text_messages_json(
    system: &str,
    messages: &[Message],
    media_marker: Option<&str>,
) -> String {
    let mut arr: Vec<Value> = vec![json!({"role": "system", "content": system})];
    arr.extend(messages.iter().filter_map(|m| {
        let content = extract_text_content(m);
        if content.trim().is_empty() {
            return None;
        }
        let role = match m.role {
            rmcp::model::Role::User => "user",
            rmcp::model::Role::Assistant => "assistant",
        };
        Some(json!({"role": role, "content": content}))
    }));
    if let Some(marker) = media_marker {
        convert_text_media_markers(&mut arr, marker);
    }
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(feature = "llamacpp")]
fn convert_text_media_markers(messages: &mut [Value], marker: &str) {
    if marker.is_empty() {
        return;
    }

    for msg in messages {
        let Some(content) = msg.get_mut("content") else {
            continue;
        };

        if let Some(text) = content.as_str() {
            if let Some(parts) = split_media_marker_text(text, marker) {
                *content = json!(parts);
            }
            continue;
        }

        let Some(content_parts) = content.as_array_mut() else {
            continue;
        };
        let mut updated = Vec::new();
        let mut changed = false;
        for part in content_parts.iter() {
            if part.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                    if let Some(parts) = split_media_marker_text(text, marker) {
                        updated.extend(parts);
                        changed = true;
                        continue;
                    }
                }
            }
            updated.push(part.clone());
        }
        if changed {
            *content_parts = updated;
        }
    }
}

#[cfg(feature = "llamacpp")]
fn split_media_marker_text(text: &str, marker: &str) -> Option<Vec<Value>> {
    let mut parts = Vec::new();
    let mut rest = text;
    let mut found_marker = false;
    while let Some((before, after)) = rest.split_once(marker) {
        found_marker = true;
        let before = before.strip_suffix('\n').unwrap_or(before);
        if !before.is_empty() {
            parts.push(json!({"type": "text", "text": before}));
        }
        parts.push(json!({"type": "media_marker", "text": marker}));
        rest = after;
        rest = rest.strip_prefix('\n').unwrap_or(rest);
    }
    if !found_marker {
        return None;
    }
    if !rest.is_empty() {
        parts.push(json!({"type": "text", "text": rest}));
    }
    Some(parts)
}

/// Remove `image_url` content parts from OpenAI-format messages JSON, replacing
/// each with a text note. This prevents an FFI crash in llama.cpp which does not
/// accept `image_url` content-part types.
fn strip_image_parts_from_messages(messages: &mut [Value]) {
    let mut stripped = false;
    for msg in messages.iter_mut() {
        if let Some(content) = msg.get_mut("content").and_then(|c| c.as_array_mut()) {
            for part in content.iter_mut() {
                if part.get("type").and_then(|t| t.as_str()) == Some("image_url") {
                    *part = json!({
                        "type": "text",
                        "text": "[Image attached — image input is not supported with the currently selected model]"
                    });
                    stripped = true;
                }
            }
        }
    }
    if stripped {
        tracing::warn!("Stripped image content parts from messages — vision encoder not available for this model");
    }
}

/// Convert a message into plain text for the emulator path's chat history.
///
/// This is the emulator-path counterpart of [`format_messages`] used by the native
/// path. It reconstructs the text-based tool syntax that the emulator prompt teaches
/// the model:
///
/// - `ToolRequest` with a `"command"` argument → `$ command`
/// - `ToolRequest` with a `"code"` argument → `` ```execute_typescript\n…\n``` ``
/// - `ToolResponse` → `Command output:\n…`
///
/// Only `developer__shell` and `code_execution__execute_typescript` style tool calls are
/// recognized (by argument shape, not tool name). Tool calls from other extensions
/// (e.g. custom MCP tools made by a native-tool-calling model earlier in the
/// conversation) are silently dropped, since the emulator path has no syntax to
/// represent them.
fn extract_text_content(msg: &Message) -> String {
    let mut parts = Vec::new();

    for content in &msg.content {
        match content {
            MessageContent::Text(text) => {
                let text = text.text.to_string();
                if !text.trim().is_empty() {
                    parts.push(text);
                }
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
                        parts.push(format!("```execute_typescript\n{}\n```", code));
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
            MessageContent::Image(_) => {
                parts.push(
                    "[Image attached — image input is not supported with the currently selected model]"
                        .to_string(),
                );
            }
            _ => {}
        }
    }

    parts.join("\n")
}

/// Build a `ProviderUsage` and write the request log entry.
fn finalize_usage(
    log: &mut Option<Box<dyn RequestLogHandle>>,
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
    name: String,
}

impl LocalInferenceProvider {
    pub async fn from_env() -> Result<Self> {
        let runtime = InferenceRuntime::get_or_init()?;
        Ok(Self {
            runtime,
            name: PROVIDER_NAME.to_string(),
        })
    }
}

impl ProviderDescriptor for LocalInferenceProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        use crate::local_model_registry::{get_registry, FEATURED_MODELS};

        let mut known_models: Vec<&str> = FEATURED_MODELS.iter().map(|m| m.spec).collect();

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
}

#[async_trait]
impl Provider for LocalInferenceProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn get_context_limit(&self, model: &str, override_limit: Option<usize>) -> usize {
        goose_provider_types::context_limit::ContextLimitResolver::new(&self.name)
            .resolve(model, override_limit, || async {
                Ok(resolve_model_path(model).map(|resolved| resolved.context_limit))
            })
            .await
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        use crate::local_model_registry::get_registry;

        let mut all_models: Vec<String> = Vec::new();

        if let Ok(registry) = get_registry().lock() {
            for entry in registry.list_models() {
                all_models.push(entry.id.clone());
            }
        }

        Ok(all_models)
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let resolved = resolve_model_path(&model_config.model_name).ok_or_else(|| {
            ProviderError::ExecutionError(format!("Model not found: {}", model_config.model_name))
        })?;
        let backend = self.runtime.backend_for_model(&resolved)?;
        let model_context_limit = resolved.context_limit;

        // Allow request_params to override thinking
        let mut model_settings = resolved.settings.clone();
        if let Some(false) = model_config
            .request_param::<bool>("enable_thinking")
            .or_else(|| {
                config_resolver::bool_param("GOOSE_LOCAL_ENABLE_THINKING")
                    .ok()
                    .flatten()
            })
        {
            model_settings.enable_thinking = false;
        }

        let cache_key = ModelCacheKey::new(
            backend.id(),
            model_config.model_name.clone(),
            model_settings.chat_template.clone(),
            model_settings.draft_model.clone(),
        );
        let model_slot = self.runtime.get_or_create_model_slot(cache_key.clone());
        let runtime = self.runtime.clone();

        let cache_key = cache_key.clone();
        let model_arc = model_slot.clone();
        let backend = backend.clone();
        let model_name = model_config.model_name.clone();
        let temperature = model_config.temperature;
        let max_tokens = model_config.max_tokens;
        let context_limit = model_context_limit;
        let settings = model_settings;
        let resolved_model = resolved.clone();
        let system = system.to_string();
        let messages = messages.to_vec();
        let tools = tools.to_vec();
        let log_payload = serde_json::json!({
            "system": &system,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": match m.role { rmcp::model::Role::User => "user", rmcp::model::Role::Assistant => "assistant" },
                    "content": extract_text_content(m),
                })
            }).collect::<Vec<_>>(),
            "tools": tools.iter().map(|t| &t.name).collect::<Vec<_>>(),
            "settings": {
                "tool_calling": settings.tool_calling,
                "chat_template": settings.chat_template,
                "context_size": settings.context_size,
                "sampling": settings.sampling,
            },
        });

        let (tx, mut rx) = tokio::sync::mpsc::channel::<
            Result<(Option<Message>, Option<ProviderUsage>), ProviderError>,
        >(32);
        let mut log = start_log(model_config, &log_payload)?;

        tokio::spawn(async move {
            let mut model_load_ms = None;

            // Ensure model is loaded — unload any other models first to free memory.
            loop {
                let state = model_slot.state.lock().await;
                match &*state {
                    ModelSlotState::Loaded(_) => break,
                    ModelSlotState::Loading => {
                        let notified = model_slot.notify.notified();
                        drop(state);
                        notified.await;
                    }
                    ModelSlotState::Empty => {
                        drop(state);

                        let cold_load_guard = runtime.cold_load_lock.lock().await;
                        let mut state = model_slot.state.lock().await;
                        match &*state {
                            ModelSlotState::Loaded(_) => break,
                            ModelSlotState::Loading => {
                                let notified = model_slot.notify.notified();
                                drop(state);
                                drop(cold_load_guard);
                                notified.await;
                                continue;
                            }
                            ModelSlotState::Empty => {}
                        }
                        *state = ModelSlotState::Loading;
                        drop(state);

                        let loading_message = Message::assistant().with_system_notification(
                            SystemNotificationType::ProgressMessage,
                            format!("Loading local model {model_name}..."),
                        );
                        if tx.send(Ok((Some(loading_message), None))).await.is_err() {
                            let mut state = model_slot.state.lock().await;
                            *state = ModelSlotState::Empty;
                            model_slot.notify.notify_waiters();
                            return;
                        }

                        let other_model_slots = runtime.other_model_slots(&cache_key);
                        for slot in other_model_slots {
                            let mut other = slot.state.lock().await;
                            if matches!(*other, ModelSlotState::Loaded(_)) {
                                tracing::info!("Unloading previous model to free memory");
                                *other = ModelSlotState::Empty;
                            }
                        }

                        let model_id = model_name.clone();
                        let resolved_for_load = resolved_model.clone();
                        let settings_for_load = settings.clone();
                        let backend_for_load = backend.clone();
                        let load_started = std::time::Instant::now();
                        let loaded = match tokio::task::spawn_blocking(move || {
                            backend_for_load.load_model(
                                &model_id,
                                &resolved_for_load,
                                &settings_for_load,
                            )
                        })
                        .await
                        {
                            Ok(Ok(loaded)) => loaded,
                            Ok(Err(err)) => {
                                let mut state = model_slot.state.lock().await;
                                *state = ModelSlotState::Empty;
                                model_slot.notify.notify_waiters();
                                let _ = log.error(&err);
                                let _ = tx.send(Err(err)).await;
                                return;
                            }
                            Err(err) => {
                                let mut state = model_slot.state.lock().await;
                                *state = ModelSlotState::Empty;
                                model_slot.notify.notify_waiters();
                                let err = ProviderError::ExecutionError(err.to_string());
                                let _ = log.error(&err);
                                let _ = tx.send(Err(err)).await;
                                return;
                            }
                        };
                        let elapsed_ms =
                            u64::try_from(load_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                        model_load_ms = Some(elapsed_ms);
                        tracing::info!(
                            backend = backend.id(),
                            model = %model_name,
                            model_load_ms = elapsed_ms,
                            "Loaded local inference model"
                        );
                        let _ = log.write(
                            &json!({
                                "path": "model_load",
                                "backend": backend.id(),
                                "model": &model_name,
                                "model_load_ms": elapsed_ms,
                            }),
                            None,
                        );

                        let mut state = model_slot.state.lock().await;
                        *state = ModelSlotState::Loaded(loaded);
                        model_slot.notify.notify_waiters();
                        drop(cold_load_guard);
                        break;
                    }
                }
            }

            tokio::task::spawn_blocking(move || {
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

                let mut model_guard = model_arc.state.blocking_lock();
                let loaded = match &mut *model_guard {
                    ModelSlotState::Loaded(loaded) => loaded.as_mut(),
                    ModelSlotState::Empty | ModelSlotState::Loading => {
                        send_err!(ProviderError::ExecutionError(
                            "Model not loaded".to_string()
                        ));
                    }
                };

                let message_id = Uuid::new_v4().to_string();
                let log = Arc::new(StdMutex::new(log));

                let request = backend::LocalGenerationRequest {
                    model_name,
                    system,
                    messages,
                    tools,
                    settings,
                    temperature,
                    max_tokens,
                    context_limit,
                    model_load_ms,
                    resolved_model,
                    message_id,
                    tx: tx.clone(),
                    log: log.clone(),
                };

                let result = backend.generate(loaded, request);

                if let Err(err) = result {
                    let msg = match &err {
                        ProviderError::ExecutionError(s) => s.as_str(),
                        ProviderError::ContextLengthExceeded(s) => s.as_str(),
                        _ => "unknown error",
                    };
                    if let Ok(mut log) = log.lock() {
                        let _ = log.error(msg);
                    }
                    let _ = tx.blocking_send(Err(err));
                }
            });
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

    #[cfg(feature = "llamacpp")]
    #[test]
    fn converts_marker_in_string_content_to_media_marker_part() {
        let mut messages = vec![json!({
            "role": "user",
            "content": "look\n<__media__>\nclosely",
        })];

        convert_text_media_markers(&mut messages, "<__media__>");

        assert_eq!(
            messages[0]["content"],
            json!([
                {"type": "text", "text": "look"},
                {"type": "media_marker", "text": "<__media__>"},
                {"type": "text", "text": "closely"},
            ])
        );
    }

    #[cfg(feature = "llamacpp")]
    #[test]
    fn converts_marker_inside_text_content_parts() {
        let mut messages = vec![json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "<__media__>describe"},
                {"type": "text", "text": "next"},
                {"type": "media_marker", "text": "<__media__>"},
            ],
        })];

        convert_text_media_markers(&mut messages, "<__media__>");

        assert_eq!(
            messages[0]["content"],
            json!([
                {"type": "media_marker", "text": "<__media__>"},
                {"type": "text", "text": "describe"},
                {"type": "text", "text": "next"},
                {"type": "media_marker", "text": "<__media__>"},
            ])
        );
    }

    #[test]
    fn preserves_balanced_info_tags_in_text_content() {
        let message =
            Message::user().with_text("before <info-msg>executed payload</info-msg> after");

        assert_eq!(
            extract_text_content(&message),
            "before <info-msg>executed payload</info-msg> after"
        );
    }

    #[test]
    fn preserves_unterminated_info_tags_in_text_content() {
        let message = Message::user().with_text("before <info-msg>executed payload");

        assert_eq!(
            extract_text_content(&message),
            "before <info-msg>executed payload"
        );
    }

    #[test]
    fn preserves_legitimate_text_content() {
        let message = Message::user().with_text("ordinary user content");

        assert_eq!(extract_text_content(&message), "ordinary user content");
    }

    #[test]
    fn auto_prefers_llamacpp_for_gguf_when_both_are_available() {
        assert_eq!(
            select_backend(ModelFormat::Gguf, None, true, true).unwrap(),
            InferenceBackend::LlamaCpp
        );
    }

    #[test]
    fn per_model_override_selects_eredu_for_gguf() {
        assert_eq!(
            select_backend(ModelFormat::Gguf, Some(InferenceBackend::Eredu), true, true).unwrap(),
            InferenceBackend::Eredu
        );
    }

    #[test]
    fn auto_selects_eredu_for_safetensors() {
        assert_eq!(
            select_backend(ModelFormat::Safetensors, None, true, true).unwrap(),
            InferenceBackend::Eredu
        );
    }

    #[test]
    fn unavailable_or_incompatible_override_is_rejected() {
        assert!(select_backend(ModelFormat::Gguf, None, false, false).is_err());
        assert!(select_backend(
            ModelFormat::Safetensors,
            Some(InferenceBackend::LlamaCpp),
            true,
            true
        )
        .is_err());
    }
}
