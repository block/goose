use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_stream::try_stream;
use futures::stream::BoxStream;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing_futures::Instrument;

use crate::agents::agent::DEFAULT_MAX_TURNS;
use crate::agents::state_machine::operation::{
    Emitter, Operation, OperationResult, TurnEffect, TurnOutcome,
};
use crate::agents::state_machine::ops_bang_shell::BangShellOperation;
use crate::agents::state_machine::ops_compaction::CompactionOperation;
use crate::agents::state_machine::ops_exit_on_error::ExitOnErrorOperation;
use crate::agents::state_machine::ops_llm::LlmOperation;
use crate::agents::state_machine::ops_maxturns::MaxTurnsOperation;
use crate::agents::state_machine::ops_retry::RetryOperation;
use crate::agents::state_machine::ops_slash_command::SlashCommandOperation;
use crate::agents::state_machine::ops_steer::SteerOperation;
use crate::agents::state_machine::ops_stop_hook::StopHookOperation;
use crate::agents::state_machine::ops_tool_approval::ToolApprovalOperation;
use crate::agents::state_machine::ops_tool_pair_compaction::ToolPairCompactionOperation;
use crate::agents::state_machine::ops_toolcalling::ToolExecutionOperation;
use crate::agents::types::SessionConfig;
use crate::agents::{Agent, AgentEvent};
use crate::config::Config;
use crate::conversation::message::Message;

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
            .emit_hook(crate::hooks::HookEvent::SessionStart, &session_id)
            .await;
    }
    let prompt_text = user_message.as_concat_text();
    if !prompt_text.is_empty() {
        agent
            .emit_user_prompt_submit_hook(&session_id, &prompt_text)
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

    if !agent.config.disable_session_naming {
        let provider = agent.provider().await?;
        let manager = session_manager.clone();
        let tx = agent.config.session_name_update_tx.clone();
        let id = session_id.clone();
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

    let (tools, toolshim_tools, system_prompt, model_config) = agent
        .prepare_tools_and_prompt(&session_id, &entry_session.working_dir)
        .await?;
    if agent.goose_mode().await == crate::config::GooseMode::SmartApprove {
        agent.tool_inspection_manager.apply_tool_annotations(&tools);
    }

    let provider = agent.provider().await?;

    let max_turns = session_config.max_turns.unwrap_or_else(|| {
        Config::global()
            .get_param::<u32>("GOOSE_MAX_TURNS")
            .unwrap_or(DEFAULT_MAX_TURNS)
    });

    let stop_hook_decided_exit = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let operations: Vec<Arc<dyn Operation + '_>> = vec![
        Arc::new(SlashCommandOperation::new(agent)),
        Arc::new(SteerOperation::new(agent)),
        Arc::new(MaxTurnsOperation::new(max_turns)),
        Arc::new(BangShellOperation::new()),
        Arc::new(CompactionOperation::new(
            agent,
            provider.clone(),
            model_config.clone(),
            session_config.schedule_id.clone(),
        )),
        Arc::new(ToolPairCompactionOperation::new(
            provider.clone(),
            model_config.clone(),
        )),
        Arc::new(ToolApprovalOperation::new(agent)),
        Arc::new(ToolExecutionOperation::new(agent)),
        Arc::new(LlmOperation::new(
            agent,
            provider,
            model_config,
            system_prompt,
            tools,
            toolshim_tools,
            session_config.schedule_id.clone(),
            max_turns,
        )),
        Arc::new(RetryOperation::new(
            agent,
            session_config.clone(),
            initial_messages,
        )),
        Arc::new(StopHookOperation::new(
            agent,
            stop_hook_decided_exit.clone(),
        )),
        Arc::new(ExitOnErrorOperation),
    ];

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

            for op in &operations {
                let emit = emitter.take().expect("emitter should be returned by skipped ops");
                let op_fut = op.run(&session, conversation, emit);
                tokio::pin!(op_fut);

                let result = loop {
                    tokio::select! {
                        biased;
                        Some(event) = rx.recv() => yield event,
                        result = &mut op_fut => break result,
                    }
                };

                match result? {
                    OperationResult::NotApplicable(emit) => {
                        emitter = Some(emit);
                    }
                    OperationResult::Applied(effects) => {
                        tracing::debug!(target: "goose::state_machine", op = op.name(), "applied operation");
                        outcome = Some(effects);
                        break;
                    }
                }
            }

            drop(emitter);

            let Some(outcome) = outcome else {
                break;
            };

            while let Some(event) = rx.recv().await {
                yield event;
            }

            let mut should_yield = false;
            for effect in outcome {
                match effect {
                    TurnEffect::AppendMessage(message) => {
                        session_manager.add_message(&session.id, &message).await?;
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
                    TurnEffect::EmitCurrentHistoryReplaced => {
                        let updated = session_manager.get_session(&session.id, true).await?;
                        let conversation = updated.conversation.unwrap_or_default();
                        yield AgentEvent::HistoryReplaced(conversation);
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
            agent.emit_stop_hook(&session_id, &last_assistant_text).await;
        }
    }.instrument(reply_stream_span)))
}
