use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use eredu::api::{
    default_local_device, discover_local_hardware, local_device_plan, LocalBackendFactory,
    LocalDrafting, LocalModel, LocalPreparedChatGenerationRequest, LocalPreparedChatInput,
    LocalPreparedChatSpeculativeGenerationRequest, PreparedChatGenerationSettings,
    PreparedChatSpeculativeGenerationOptions,
};
use eredu::runtime::chat::{
    ChatTemplateRequest, ParallelToolCallPolicy, SemanticSupport, ToolChoice,
};
use eredu_core::{
    DraftPlacementPlan, DraftingPlan, ExecutionPlan, GenerationCancellationToken,
    GenerationConfigOverrides, SemanticEvent, SpeculativeSchedulerOptions, TextGenerationConfig,
};
use goose_provider_types::conversation::message::{Message, MessageContent};
use goose_provider_types::conversation::token_usage::ProviderStats;
use goose_provider_types::errors::ProviderError;
use goose_provider_types::formats::openai;
use goose_provider_types::images::ImageFormat;
use rmcp::model::CallToolRequestParams;
use serde_json::{json, Map, Value};

use crate::backend::{BackendLoadedModel, LocalGenerationRequest, LocalInferenceBackend};
use crate::local_model_registry::{ChatTemplate, ModelSettings, SamplingConfig, ToolCallingMode};
use crate::{finalize_usage, strip_image_parts_from_messages, ResolvedModelPaths};

pub(super) const EREDU_BACKEND_ID: &str = "eredu";

pub(super) struct EreduBackend;

struct EreduLoadedModel {
    commands: mpsc::Sender<EreduCommand>,
}

enum EreduCommand {
    Generate {
        request: LocalGenerationRequest,
        result: mpsc::Sender<Result<(), ProviderError>>,
    },
}

struct ThreadLocalEreduModel {
    model: LocalModel,
    drafting: LocalDrafting,
}

impl BackendLoadedModel for EreduLoadedModel {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
struct ToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl EreduBackend {
    pub(super) fn new() -> Self {
        Self
    }
}

impl LocalInferenceBackend for EreduBackend {
    fn id(&self) -> &'static str {
        EREDU_BACKEND_ID
    }

    fn load_model(
        &self,
        model_id: &str,
        resolved: &ResolvedModelPaths,
        _settings: &ModelSettings,
    ) -> Result<Box<dyn BackendLoadedModel>, ProviderError> {
        if !resolved.model_path.exists() {
            return Err(ProviderError::ExecutionError(format!(
                "Model not downloaded: {model_id}. Please download it from Settings > Local Inference."
            )));
        }

        let model_path = resolved.model_path.clone();
        let draft_model_path = resolved.draft_model_path.clone();
        let (commands, command_rx) = mpsc::channel::<EreduCommand>();
        let (loaded_tx, loaded_rx) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name(format!("eredu-{model_id}"))
            .spawn(move || {
                let loaded = load_thread_local_model(&model_path, draft_model_path.as_deref());
                match loaded {
                    Ok(mut loaded) => {
                        if loaded_tx.send(Ok(())).is_err() {
                            return;
                        }
                        while let Ok(command) = command_rx.recv() {
                            match command {
                                EreduCommand::Generate { request, result } => {
                                    let generation = generate_on_model(&mut loaded, request);
                                    let _ = result.send(generation);
                                }
                            }
                        }
                    }
                    Err(error) => {
                        let _ = loaded_tx.send(Err(error));
                    }
                }
            })
            .map_err(|error| ProviderError::ExecutionError(error.to_string()))?;
        loaded_rx.recv().map_err(|error| {
            ProviderError::ExecutionError(format!(
                "eredu model thread stopped while loading: {error}"
            ))
        })??;

        Ok(Box::new(EreduLoadedModel { commands }))
    }

    fn generate(
        &self,
        loaded: &mut dyn BackendLoadedModel,
        request: LocalGenerationRequest,
    ) -> Result<(), ProviderError> {
        let loaded = loaded
            .as_any_mut()
            .downcast_mut::<EreduLoadedModel>()
            .ok_or_else(|| {
                ProviderError::ExecutionError("Loaded model backend mismatch".to_string())
            })?;
        let (result_tx, result_rx) = mpsc::channel();
        loaded
            .commands
            .send(EreduCommand::Generate {
                request,
                result: result_tx,
            })
            .map_err(|_| {
                ProviderError::ExecutionError("eredu model thread is no longer running".to_string())
            })?;
        result_rx.recv().map_err(|_| {
            ProviderError::ExecutionError(
                "eredu model thread stopped during generation".to_string(),
            )
        })?
    }

    fn available_memory_bytes(&self) -> u64 {
        discover_local_hardware()
            .available_memory_bytes
            .value()
            .copied()
            .unwrap_or_default()
    }
}

fn load_thread_local_model(
    model_path: &std::path::Path,
    draft_model_path: Option<&std::path::Path>,
) -> Result<ThreadLocalEreduModel, ProviderError> {
    let device = local_device_plan(default_local_device())
        .map_err(|error| ProviderError::ExecutionError(error.to_string()))?;
    let mut plan = ExecutionPlan::fully_resident(device);
    if let Some(draft_model_path) = draft_model_path {
        plan = plan.with_drafting(DraftingPlan::External {
            model: draft_model_path.display().to_string(),
            placement: DraftPlacementPlan::Target,
            max_draft_tokens: 3,
            lookahead: true,
            adaptive_lookahead: false,
        });
    }
    let planned =
        LocalModel::load_execution_plan(&LocalBackendFactory::default(), model_path, &plan)
            .map_err(|error| ProviderError::ExecutionError(error.to_string()))?;
    let (model, drafting) = planned.into_parts();
    Ok(ThreadLocalEreduModel { model, drafting })
}

fn generate_on_model(
    loaded: &mut ThreadLocalEreduModel,
    request: LocalGenerationRequest,
) -> Result<(), ProviderError> {
    if !matches!(request.settings.chat_template, ChatTemplate::Embedded) {
        return Err(ProviderError::InvalidValue(
                "eredu uses the chat template bundled with the model; custom and llama.cpp built-in templates are not supported"
                    .to_string(),
            ));
    }
    if !request.tools.is_empty()
        && matches!(
            request.settings.tool_calling,
            ToolCallingMode::ForceEmulated
        )
    {
        return Err(ProviderError::InvalidValue(
                "eredu provides native constrained tool calling and does not support Goose's legacy tool-call emulator"
                    .to_string(),
            ));
    }

    let mut messages = Vec::new();
    if !request.system.trim().is_empty() {
        messages.push(json!({"role": "system", "content": &request.system}));
    }
    messages.extend(openai::format_messages(
        &request.messages,
        &ImageFormat::OpenAi,
    ));
    strip_image_parts_from_messages(&mut messages);
    let tools = openai::format_tools(&request.tools)
        .map_err(|error| ProviderError::InvalidValue(error.to_string()))?;

    let prepared = loaded
        .model
        .prepare_chat(ChatTemplateRequest {
            messages,
            tools,
            tool_choice: if request.tools.is_empty() {
                ToolChoice::None
            } else {
                ToolChoice::Auto
            },
            parallel_tool_calls: ParallelToolCallPolicy::Enabled { max_calls: None },
            enable_thinking: Some(request.settings.enable_thinking),
            allow_unparsed_reasoning: false,
            add_generation_prompt: true,
            ..ChatTemplateRequest::default()
        })
        .map_err(|error| ProviderError::ExecutionError(error.to_string()))?;

    if !request.tools.is_empty() && !prepared.native_tool_support().is_supported() {
        let reason = prepared
            .native_tool_support()
            .unsupported_reason()
            .unwrap_or("the model's chat protocol is not registered");
        return Err(ProviderError::NotImplemented(format!(
            "eredu native tool calling is unavailable for this model: {reason}"
        )));
    }

    let prompt_token_count = loaded
        .model
        .encode(prepared.rendered_prompt(), false)
        .map_err(|error| ProviderError::ExecutionError(error.to_string()))?
        .len();
    let max_new_tokens = output_token_limit(&request, prompt_token_count)?;
    let (overrides, seed) = generation_settings(&request, max_new_tokens);
    let started = Instant::now();

    let output_token_count =
        if matches!(request.settings.sampling, SamplingConfig::MirostatV2 { .. }) {
            if !request.tools.is_empty() {
                return Err(ProviderError::InvalidValue(
                    "Mirostat sampling cannot be combined with eredu constrained tool calling"
                        .to_string(),
                ));
            }
            generate_raw(
                &mut loaded.model,
                prepared.rendered_prompt(),
                overrides,
                seed,
                &request,
            )?
        } else {
            if !matches!(prepared.semantic_support(), SemanticSupport::Supported) {
                let reason = prepared
                    .semantic_support()
                    .unsupported_reason()
                    .unwrap_or("the model's response protocol is not registered");
                return Err(ProviderError::NotImplemented(format!(
                    "eredu semantic chat generation is unavailable for this model: {reason}"
                )));
            }

            generate_semantic(loaded, &prepared, overrides, seed, &request)?
        };

    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let mut log = request.log.lock().map_err(|_| {
        ProviderError::ExecutionError("Local inference request log lock poisoned".to_string())
    })?;
    let mut usage = finalize_usage(
        &mut log,
        request.model_name.clone(),
        EREDU_BACKEND_ID,
        prompt_token_count,
        output_token_count as i32,
        None,
    );
    usage.stats = Some(ProviderStats {
        model_load_ms: request.model_load_ms,
        elapsed_ms: Some(elapsed_ms),
        output_tokens: Some(output_token_count),
        ..ProviderStats::default()
    });
    let _ = request.tx.blocking_send(Ok((None, Some(usage))));
    Ok(())
}

fn output_token_limit(
    request: &LocalGenerationRequest,
    prompt_token_count: usize,
) -> Result<usize, ProviderError> {
    let requested = request
        .max_tokens
        .and_then(|value| usize::try_from(value).ok())
        .or(request.settings.max_output_tokens)
        .unwrap_or(2048);
    if request.context_limit == 0 {
        return Ok(requested.max(1));
    }
    if prompt_token_count >= request.context_limit {
        return Err(ProviderError::ContextLengthExceeded(format!(
            "Prompt uses {prompt_token_count} tokens, but the configured context size is {}",
            request.context_limit
        )));
    }
    let remaining = request.context_limit - prompt_token_count;
    Ok(requested.min(remaining).max(1))
}

fn generation_settings(
    request: &LocalGenerationRequest,
    max_new_tokens: usize,
) -> (GenerationConfigOverrides, u64) {
    let (mut overrides, seed) = match &request.settings.sampling {
        SamplingConfig::Greedy => (
            GenerationConfigOverrides {
                do_sample: Some(false),
                ..GenerationConfigOverrides::default()
            },
            0,
        ),
        SamplingConfig::Temperature {
            temperature,
            top_k,
            top_p,
            min_p,
            seed,
        } => (
            GenerationConfigOverrides {
                do_sample: Some(true),
                temperature: Some(*temperature),
                top_k: Some(*top_k),
                top_p: Some(*top_p),
                min_p: Some(*min_p),
                ..GenerationConfigOverrides::default()
            },
            seed.unwrap_or_default() as u64,
        ),
        SamplingConfig::MirostatV2 { seed, .. } => (
            GenerationConfigOverrides {
                do_sample: Some(true),
                ..GenerationConfigOverrides::default()
            },
            seed.unwrap_or_default() as u64,
        ),
    };
    if let Some(temperature) = request.temperature {
        overrides.do_sample = Some(temperature > 0.0);
        overrides.temperature = Some(temperature);
    }
    overrides.max_new_tokens = Some(max_new_tokens);
    overrides.repetition_penalty = Some(request.settings.repeat_penalty);
    overrides.repeat_last_n = Some(request.settings.repeat_last_n);
    overrides.frequency_penalty = Some(request.settings.frequency_penalty);
    overrides.presence_penalty = Some(request.settings.presence_penalty);
    (overrides, seed)
}

fn generate_semantic(
    loaded: &mut ThreadLocalEreduModel,
    prepared: &eredu::runtime::chat::PreparedChat,
    overrides: GenerationConfigOverrides,
    seed: u64,
    request: &LocalGenerationRequest,
) -> Result<usize, ProviderError> {
    let mut tool_calls = BTreeMap::<usize, ToolCall>::new();
    let mut accumulated_thinking = String::new();
    let cancellation = GenerationCancellationToken::new();
    let output_token_count = {
        let callback_cancellation = cancellation.clone();
        let mut on_event = |event| match event {
            SemanticEvent::ReasoningDelta(delta) => {
                accumulated_thinking.push_str(&delta);
                if !send_message(request, Message::assistant().with_thinking(delta, "")) {
                    callback_cancellation.cancel();
                }
            }
            SemanticEvent::TextDelta(delta) => {
                if !send_message(request, Message::assistant().with_text(delta)) {
                    callback_cancellation.cancel();
                }
            }
            SemanticEvent::ToolCallStart { index, id, name } => {
                tool_calls.insert(
                    index,
                    ToolCall {
                        id: if id.is_empty() {
                            uuid::Uuid::new_v4().to_string()
                        } else {
                            id
                        },
                        name,
                        arguments: String::new(),
                    },
                );
            }
            SemanticEvent::ToolArgumentsDelta {
                index,
                json_fragment,
            } => {
                if let Some(call) = tool_calls.get_mut(&index) {
                    call.arguments.push_str(&json_fragment);
                }
            }
            SemanticEvent::ToolCallEnd | SemanticEvent::Finished { .. } => {}
        };

        let settings = PreparedChatGenerationSettings { overrides, seed };
        if loaded.drafting.is_enabled() {
            loaded
                .model
                .generate_prepared_chat_speculative(LocalPreparedChatSpeculativeGenerationRequest {
                    input: LocalPreparedChatInput::rendered_prompt(prepared),
                    drafting: &mut loaded.drafting,
                    settings,
                    options: PreparedChatSpeculativeGenerationOptions {
                        max_draft_tokens: NonZeroUsize::new(3).expect("3 is non-zero"),
                        scheduler: SpeculativeSchedulerOptions::default(),
                    },
                    caller_stop_sequences: &[],
                    cancellation: cancellation.clone(),
                    on_event: &mut on_event,
                })
                .map_err(|error| ProviderError::ExecutionError(error.to_string()))?
                .token_ids()
                .len()
        } else {
            loaded
                .model
                .generate_prepared_chat(LocalPreparedChatGenerationRequest {
                    input: LocalPreparedChatInput::rendered_prompt(prepared),
                    settings,
                    caller_stop_sequences: &[],
                    cancellation,
                    on_event: &mut on_event,
                })
                .map_err(|error| ProviderError::ExecutionError(error.to_string()))?
                .token_ids
                .len()
        }
    };

    let mut contents = Vec::new();
    if !tool_calls.is_empty() && !accumulated_thinking.is_empty() {
        contents.push(MessageContent::thinking(accumulated_thinking, ""));
    }
    for (_, call) in tool_calls {
        let arguments = if call.arguments.is_empty() {
            None
        } else {
            Some(
                serde_json::from_str::<Map<String, Value>>(&call.arguments).map_err(|error| {
                    ProviderError::ExecutionError(format!(
                        "eredu produced invalid tool arguments for {}: {error}",
                        call.name
                    ))
                })?,
            )
        };
        let params = match arguments {
            Some(arguments) => {
                CallToolRequestParams::new(Cow::Owned(call.name)).with_arguments(arguments)
            }
            None => CallToolRequestParams::new(Cow::Owned(call.name)),
        };
        contents.push(MessageContent::tool_request(call.id, Ok(params)));
    }
    if !contents.is_empty() {
        let mut message = Message::new(
            rmcp::model::Role::Assistant,
            chrono::Utc::now().timestamp(),
            contents,
        );
        message.id = Some(request.message_id.to_string());
        let _ = request.tx.blocking_send(Ok((Some(message), None)));
    }

    Ok(output_token_count)
}

fn generate_raw(
    model: &mut LocalModel,
    prompt: &str,
    overrides: GenerationConfigOverrides,
    seed: u64,
    request: &LocalGenerationRequest,
) -> Result<usize, ProviderError> {
    let prompt_ids = model
        .encode(prompt, false)
        .map_err(|error| ProviderError::ExecutionError(error.to_string()))?;
    let resolved = model
        .resolve_generation_config(overrides)
        .map_err(|error| ProviderError::InvalidValue(error.to_string()))?;
    let mut config = TextGenerationConfig::new(resolved).with_seed(seed);
    if let SamplingConfig::MirostatV2 { tau, eta, .. } = &request.settings.sampling {
        config = config
            .with_mirostat_v2(*tau, *eta)
            .map_err(|error| ProviderError::InvalidValue(error.to_string()))?;
    }

    let eos_token_ids = model.eos_token_ids().to_vec();
    let mut decoder = model.text_decoder(true);
    let mut count = 0;
    for token in model
        .generate_tokens(prompt_ids, config)
        .map_err(|error| ProviderError::ExecutionError(error.to_string()))?
    {
        let token = token.map_err(|error| ProviderError::ExecutionError(error.to_string()))?;
        count += 1;
        if eos_token_ids.contains(&token) {
            break;
        }
        if let Some(delta) = decoder
            .step(token)
            .map_err(|error| ProviderError::ExecutionError(error.to_string()))?
        {
            if !send_message(request, Message::assistant().with_text(delta)) {
                break;
            }
        }
    }
    Ok(count)
}

fn send_message(request: &LocalGenerationRequest, mut message: Message) -> bool {
    message.id = Some(request.message_id.to_string());
    request.tx.blocking_send(Ok((Some(message), None))).is_ok()
}
