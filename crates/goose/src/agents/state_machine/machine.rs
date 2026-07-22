use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_stream::try_stream;
use futures::stream::BoxStream;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing_futures::Instrument;

use crate::agents::state_machine::operation::{
    Emitter, Inference, InferenceInput, Operation, OperationFuture, OperationResult, TurnEffect,
    TurnOutcome,
};
use crate::agents::state_machine::ops_bang_shell::BangShellOperation;
use crate::agents::state_machine::ops_compaction::CompactionOperation;
use crate::agents::state_machine::ops_doctor::DoctorOperation;
use crate::agents::state_machine::ops_exit_on_error::ExitOnErrorOperation;
use crate::agents::state_machine::ops_llm::InferenceRunner;
use crate::agents::state_machine::ops_maxturns::{MaxTurnsOperation, DEFAULT_MAX_TURNS};
use crate::agents::state_machine::ops_recipe::RecipeOperation;
use crate::agents::state_machine::ops_retry::RetryOperation;
use crate::agents::state_machine::ops_skills::SkillOperation;
use crate::agents::state_machine::ops_slash_command::SlashCommandOperation;
use crate::agents::state_machine::ops_steer::SteerOperation;
use crate::agents::state_machine::ops_stop_hook::{StopHookOperation, DEFAULT_STOP_HOOK_BLOCK_CAP};
use crate::agents::state_machine::ops_tool_approval::ToolApprovalOperation;
use crate::agents::state_machine::ops_tool_pair_compaction::ToolPairCompactionOperation;
use crate::agents::state_machine::ops_toolcalling::ToolExecutionOperation;
use crate::agents::state_machine::ops_unknown_tool::UnknownToolOperation;
use crate::agents::types::SessionConfig;
use crate::agents::{Agent, AgentEvent};
use crate::config::Config;
use crate::context_mgmt::{
    compute_tool_call_cutoff, tool_pair_summarization_enabled, DEFAULT_COMPACTION_THRESHOLD,
};
use crate::conversation::message::Message;
use crate::hooks::{HookContext, HookEvent};
use crate::providers::base::ProviderUsage;

enum Step<'a> {
    Operation(&'a dyn Operation),
    Inference(&'a dyn Inference),
}

struct Pipeline<'a> {
    operations: Vec<Arc<dyn Operation + 'a>>,
    inference: Arc<dyn Inference + 'a>,
}

impl Pipeline<'_> {
    fn steps(&self) -> impl Iterator<Item = Step<'_>> {
        self.operations
            .iter()
            .map(|operation| Step::Operation(operation.as_ref()))
            .chain(std::iter::once(Step::Inference(self.inference.as_ref())))
    }
}

fn attach_usage_to_last_assistant(effects: &mut [TurnEffect], usage: &ProviderUsage) {
    let Some(message) = effects.iter_mut().rev().find_map(|effect| match effect {
        TurnEffect::AppendMessage(message)
            if message.role == rmcp::model::Role::Assistant && message.error_kind().is_none() =>
        {
            Some(message)
        }
        _ => None,
    }) else {
        return;
    };
    message.metadata.usage = Some(Box::new(
        crate::conversation::message::MessageUsage::from_provider_usage(usage, false),
    ));
}

pub async fn reply(
    agent: &Agent,
    user_message: Message,
    session_config: SessionConfig,
    cancel_token: Option<CancellationToken>,
) -> Result<BoxStream<'_, Result<AgentEvent>>> {
    let session_manager = agent.config.session_manager.clone();

    let cancel = cancel_token.unwrap_or_default();

    let session_id = session_config.id.clone();

    let entry_session = session_manager.get_session(&session_id, false).await?;
    if entry_session.message_count == 0 {
        agent
            .hook_manager
            .emit(
                HookEvent::SessionStart,
                HookContext::new(HookEvent::SessionStart, &session_id),
            )
            .await;
    }
    let prompt_text = user_message.as_concat_text();
    if !prompt_text.is_empty() {
        agent
            .hook_manager
            .emit(
                HookEvent::UserPromptSubmit,
                HookContext::new(HookEvent::UserPromptSubmit, &session_id)
                    .with_message(prompt_text),
            )
            .await;
    }

    session_manager
        .add_message(&session_config.id, &user_message)
        .await?;

    let initial_messages = if session_config.retry_config.is_some() {
        session_manager
            .get_session(&session_id, true)
            .await?
            .conversation
            .map(|conversation| conversation.messages().clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let provider = agent
        .provider
        .lock()
        .await
        .clone()
        .ok_or_else(|| anyhow!("Provider not set"))?;

    if !agent.config.disable_session_naming {
        let manager = session_manager.clone();
        let tx = agent.config.session_name_update_tx.clone();
        let id = session_id.clone();
        let provider = provider.clone();
        tokio::spawn(async move {
            match manager.maybe_update_name(&id, provider).await {
                Ok(Some(update)) => {
                    if let Some(tx) = tx {
                        if tx.send(update).is_err() {
                            tracing::warn!("Failed to publish generated session name");
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => tracing::warn!("Failed to generate session description: {}", e),
            }
        });
    }

    let model_config = match entry_session.model_config {
        Some(model_config) => model_config,
        None => {
            let provider_name = Config::global()
                .get_goose_provider()
                .map_err(|_| anyhow!("Could not resolve model config: missing provider"))?;
            let model_name = Config::global()
                .get_goose_model()
                .map_err(|_| anyhow!("Could not resolve model config: missing model"))?;
            crate::model_config::model_config_from_user_config(&provider_name, &model_name)
                .map_err(|error| anyhow!("Could not resolve model config: {error}"))?
        }
    };

    let max_turns = session_config.max_turns.unwrap_or_else(|| {
        Config::global()
            .get_param::<u32>("GOOSE_MAX_TURNS")
            .unwrap_or(DEFAULT_MAX_TURNS)
    });

    let stop_hook_decided_exit = Arc::new(std::sync::atomic::AtomicBool::new(false));
    #[cfg(test)]
    let stop_hook_block_cap = agent.stop_hook_block_cap_override.unwrap_or_else(|| {
        Config::global()
            .get_param::<u32>("GOOSE_STOP_HOOK_BLOCK_CAP")
            .unwrap_or(DEFAULT_STOP_HOOK_BLOCK_CAP)
    });
    #[cfg(not(test))]
    let stop_hook_block_cap = Config::global()
        .get_param::<u32>("GOOSE_STOP_HOOK_BLOCK_CAP")
        .unwrap_or(DEFAULT_STOP_HOOK_BLOCK_CAP);
    let compaction_threshold = Config::global()
        .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
        .unwrap_or(DEFAULT_COMPACTION_THRESHOLD);
    let tool_call_cutoff = Config::global()
        .get_param::<usize>("GOOSE_TOOL_CALL_CUTOFF")
        .unwrap_or_else(|_| {
            compute_tool_call_cutoff(model_config.context_limit(), compaction_threshold)
        });
    let tool_pair_compaction_enabled =
        tool_pair_summarization_enabled() && !provider.manages_own_context();
    let regular_operations: Vec<Arc<dyn Operation + '_>> = vec![
        Arc::new(SteerOperation::new(
            &agent.pending_steers,
            agent.hook_manager.clone(),
        )),
        Arc::new(MaxTurnsOperation::new(max_turns)),
        Arc::new(BangShellOperation::new()),
        Arc::new(CompactionOperation::new(
            provider.clone(),
            model_config.clone(),
            compaction_threshold,
        )),
        Arc::new(ToolPairCompactionOperation::new(
            provider.clone(),
            model_config.clone(),
            tool_call_cutoff,
            tool_pair_compaction_enabled,
        )),
        Arc::new(ToolApprovalOperation::new(
            &agent.current_goose_mode,
            &agent.tool_inspection_manager,
        )),
        Arc::new(DoctorOperation),
        Arc::new(SkillOperation),
        Arc::new(RecipeOperation),
        Arc::new(ToolExecutionOperation::new(
            &agent.current_goose_mode,
            session_manager.clone(),
            agent.extension_manager.clone(),
            agent.hook_manager.clone(),
        )),
        Arc::new(UnknownToolOperation),
        Arc::new(RetryOperation::new(
            &agent.retry_manager,
            &agent.goal,
            &agent.grind,
            session_config.clone(),
            initial_messages,
        )),
        Arc::new(StopHookOperation::new(
            agent.hook_manager.clone(),
            stop_hook_block_cap,
            stop_hook_decided_exit.clone(),
        )),
        Arc::new(ExitOnErrorOperation),
    ];
    let inference = Arc::new(InferenceRunner::new(provider, model_config.clone()));
    let mut command_handlers = regular_operations.clone();
    command_handlers.push(inference.clone());
    let command_operation: Arc<dyn Operation + '_> =
        Arc::new(SlashCommandOperation::new(command_handlers));
    let operations = std::iter::once(command_operation)
        .chain(regular_operations)
        .collect();
    let pipeline = Pipeline {
        operations,
        inference,
    };

    let reply_stream_span = tracing::info_span!(
        target: "goose::agents::agent",
        "reply_stream",
        trace_output = tracing::field::Empty,
        session.id = %session_config.id,
        session.user = %crate::session_context::session_user(),
        session.host = %crate::session_context::session_host(),
        session.agent_type = "goose",
    );

    Ok(Box::pin(try_stream! {
        loop {
            if cancel.is_cancelled() {
                break;
            }

            let session = session_manager
                .get_session(&session_id, true)
                .await?;
            let conversation = session
                .conversation
                .as_ref()
                .ok_or_else(|| anyhow!("state-machine session loaded without conversation"))?;

            let (tx, mut rx) = mpsc::channel::<AgentEvent>(32);
            let mut emitter = Some(Emitter::new(tx, cancel.clone()));
            let mut outcome: Option<TurnOutcome> = None;

            for step in pipeline.steps() {
                let emit = emitter
                    .take()
                    .ok_or_else(|| anyhow!("step did not return the event emitter"))?;
                let (name, step_fut): (_, OperationFuture<'_, Result<OperationResult>>) =
                    match step {
                        Step::Operation(operation) => {
                            (operation.name(), operation.run(&session, conversation, emit))
                        }
                        Step::Inference(inference) => {
                            let mut tools = Vec::new();
                            let mut moim_parts = Vec::new();
                            let mut prompt_parts = Vec::new();
                            for operation in &pipeline.operations {
                                tools.extend(operation.inference_tools(&session).await?);
                                prompt_parts.extend(
                                    operation.prompt_parts(&session, conversation).await?,
                                );
                                moim_parts.extend(
                                    operation.moim_parts(&session, conversation).await?,
                                );
                            }
                            tools.extend(inference.inference_tools(&session).await?);
                            prompt_parts
                                .extend(inference.prompt_parts(&session, conversation).await?);
                            moim_parts
                                .extend(inference.moim_parts(&session, conversation).await?);

                            let mut names = std::collections::HashSet::new();
                            for tool in &tools {
                                if !names.insert(tool.name.to_string()) {
                                    Err::<(), _>(anyhow!(
                                        "more than one operation advertised tool '{}'",
                                        tool.name
                                    ))?;
                                }
                            }

                            #[cfg(feature = "code-mode")]
                            let code_execution_mode = agent
                                .extension_manager
                                .is_extension_enabled(
                                    crate::agents::platform_extensions::code_execution::EXTENSION_NAME,
                                )
                                .await;
                            #[cfg(not(feature = "code-mode"))]
                            let code_execution_mode = false;

                            if *agent.current_goose_mode.lock().await
                                == crate::config::GooseMode::SmartApprove
                            {
                                agent.tool_inspection_manager.apply_tool_annotations(&tools);
                            }
                            let tools = crate::agents::reply_parts::prepare_inference_tools(
                                tools,
                                code_execution_mode,
                            );
                            let goose_mode = *agent.current_goose_mode.lock().await;
                            if let Some(frontend_instructions) =
                                agent.frontend_instructions.lock().await.clone()
                            {
                                prompt_parts
                                    .push(("frontend".to_string(), frontend_instructions));
                            }
                            let system_prompt = agent
                                .prompt_manager
                                .lock()
                                .await
                                .build_system_prompt(
                                    &session.working_dir,
                                    prompt_parts,
                                    goose_mode,
                                );
                            let (tools, toolshim_tools, system_prompt) =
                                crate::agents::reply_parts::prepare_tools_for_provider(
                                    tools,
                                    system_prompt,
                                    &model_config,
                                );
                            let input = InferenceInput {
                                system_prompt,
                                tools,
                                toolshim_tools,
                                moim_parts,
                            };
                            (
                                inference.name(),
                                inference.infer(&session, conversation, input, emit),
                            )
                        }
                    };
                tokio::pin!(step_fut);

                let result = loop {
                    tokio::select! {
                        biased;
                        Some(event) = rx.recv() => yield event,
                        result = &mut step_fut => break result,
                    }
                };

                match result? {
                    OperationResult::NotApplicable(emit) => {
                        emitter = Some(emit);
                    }
                    OperationResult::Applied(effects) => {
                        tracing::debug!(target: "goose::state_machine", step = name, "applied step");
                        outcome = Some(effects);
                        break;
                    }
                }
            }

            drop(emitter);

            let Some(mut outcome) = outcome else {
                break;
            };

            for index in 0..outcome.len() {
                let (usage, is_compaction) = match &outcome[index] {
                    TurnEffect::RecordUsage {
                        usage,
                        is_compaction,
                    } => (usage.clone(), *is_compaction),
                    _ => continue,
                };
                let enriched = crate::agents::reply_parts::update_session_metrics(
                    &session_manager,
                    &session.id,
                    session_config.schedule_id.clone(),
                    &usage,
                    is_compaction,
                )
                .await?;
                if !is_compaction {
                    attach_usage_to_last_assistant(&mut outcome, &enriched);
                }
                outcome[index] = TurnEffect::RecordUsage {
                    usage: enriched,
                    is_compaction,
                };
            }

            while let Some(event) = rx.recv().await {
                yield event;
            }

            let mut should_yield = false;
            for effect in outcome {
                match effect {
                    TurnEffect::AppendMessage(message) => {
                        let message_usage = message
                            .metadata
                            .usage
                            .as_deref()
                            .filter(|_| !message.user_visible_content().content.is_empty())
                            .cloned();
                        let message_id = message.id.clone();
                        session_manager.add_message(&session.id, &message).await?;
                        if let Some(usage) = message_usage {
                            yield AgentEvent::MessageUsage { message_id, usage };
                        }
                    }
                    TurnEffect::ReplaceConversation(conversation) => {
                        session_manager
                            .replace_conversation(&session.id, &conversation)
                            .await?;
                        yield AgentEvent::HistoryReplaced(conversation);
                    }
                    TurnEffect::PatchToolRequestMeta {
                        message_id,
                        tool_call_id,
                        patch,
                    } => {
                        session_manager
                            .update_tool_request_meta(&session.id, &message_id, &tool_call_id, patch)
                            .await?;
                    }
                    TurnEffect::SetMessageVisibility {
                        message_id,
                        user_visible,
                        agent_visible,
                    } => {
                        session_manager
                            .update_message_metadata(&session.id, &message_id, |mut metadata| {
                                metadata.user_visible = user_visible;
                                metadata.agent_visible = agent_visible;
                                metadata
                            })
                            .await?;
                    }
                    TurnEffect::SetRecipe(recipe) => {
                        session_manager.update(&session.id).recipe(recipe).apply().await?;
                    }
                    TurnEffect::RecordUsage {
                        usage,
                        is_compaction,
                    } => {
                        if !is_compaction {
                            yield AgentEvent::Usage(usage);
                        }
                    }
                    TurnEffect::ResetContextUsage => {
                        session_manager
                            .update(&session.id)
                            .usage(goose_providers::conversation::token_usage::Usage::new(
                                Some(0),
                                Some(0),
                                Some(0),
                            ))
                            .apply()
                            .await?;
                    }
                    TurnEffect::YieldToClient => {
                        should_yield = true;
                        break;
                    }
                }
            }
            if should_yield {
                break;
            }
        }

        let last_assistant_text = session_manager
            .get_session(&session_id, true)
            .await?
            .conversation
            .unwrap_or_default()
            .messages()
            .iter()
            .rev()
            .filter(|m| m.role == rmcp::model::Role::Assistant)
            .map(Message::as_concat_text)
            .find(|text| !text.is_empty())
            .unwrap_or_default();
        if !last_assistant_text.is_empty() {
            tracing::Span::current().record("trace_output", last_assistant_text.as_str());
        }
        if !stop_hook_decided_exit.load(std::sync::atomic::Ordering::Relaxed) {
            agent
                .hook_manager
                .emit(
                    HookEvent::Stop,
                    HookContext::new(HookEvent::Stop, &session_id)
                        .with_last_assistant_message(last_assistant_text),
                )
                .await;
        }
    }.instrument(reply_stream_span)))
}
