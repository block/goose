use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_stream::try_stream;
use futures::stream::BoxStream;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::agents::agent::DEFAULT_MAX_TURNS;
use crate::agents::state_machine::operation::{
    Emitter, Operation, OperationResult, TurnEffect, TurnOutcome,
};
use crate::agents::state_machine::ops_compaction::CompactionOperation;
use crate::agents::state_machine::ops_exit_on_error::ExitOnErrorOperation;
use crate::agents::state_machine::ops_llm::LlmOperation;
use crate::agents::state_machine::ops_maxturns::MaxTurnsOperation;
use crate::agents::state_machine::ops_slash_command::SlashCommandOperation;
use crate::agents::state_machine::ops_tool_approval::ToolApprovalOperation;
use crate::agents::state_machine::ops_toolcalling::ToolExecutionOperation;
use crate::agents::types::SessionConfig;
use crate::agents::{Agent, AgentEvent};
use crate::config::Config;
use crate::conversation::message::Message;

/// State-machine replacement for `Agent::reply`.
pub async fn reply(
    agent: &Agent,
    user_message: Message,
    session_config: SessionConfig,
    cancel_token: Option<CancellationToken>,
) -> Result<BoxStream<'_, Result<AgentEvent>>> {
    let session_manager = agent.config.session_manager.clone();

    // A never-cancelled token stands in for the un-cancellable caller so the
    // loop body never has to branch on the `Option`.
    let cancel = cancel_token.unwrap_or_default();

    session_manager
        .add_message(&session_config.id, &user_message)
        .await?;

    let session_id = session_config.id.clone();

    // Session naming is out-of-band: a detached task that overlaps the reply
    // loop, generates a title once early in a session, persists it, and pushes
    // it to the UI. It never reads or mutates the conversation, so it is not an
    // operation — see WIP.md "Two kinds of work".
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

    let working_dir = session_manager
        .get_session(&session_id, false)
        .await?
        .working_dir;
    let (tools, _toolshim_tools, system_prompt, model_config) = agent
        .prepare_tools_and_prompt(&session_id, &working_dir)
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

    let operations: Vec<Arc<dyn Operation + '_>> = vec![
        Arc::new(SlashCommandOperation::new(agent)),
        Arc::new(MaxTurnsOperation::new(max_turns)),
        Arc::new(CompactionOperation::new(
            agent,
            provider.clone(),
            model_config.clone(),
            session_config.schedule_id.clone(),
        )),
        Arc::new(ToolApprovalOperation::new(agent)),
        Arc::new(ToolExecutionOperation::new(
            agent.extension_manager.clone(),
            session_manager.clone(),
        )),
        Arc::new(LlmOperation::new(
            agent,
            provider,
            model_config,
            system_prompt,
            tools,
            session_config.schedule_id.clone(),
        )),
        Arc::new(ExitOnErrorOperation),
    ];

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

            // Op returned; its Emitter dropped; channel closed. Drain leftovers.
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
    }))
}
