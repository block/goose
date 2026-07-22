//! Decides whether a completed response should finish, continue, or retry the turn.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::agents::retry::{RetryManager, RetryResult};
use crate::agents::state_machine::operation::{
    applied, ends_turn, messages_since_kickoff, not_applicable, Emitter, Operation,
    OperationResult, SlashCommand, TurnEffect,
};
use crate::agents::types::SessionConfig;
use crate::agents::AgentEvent;
use crate::conversation::message::{Message, SystemNotificationType};
use crate::conversation::Conversation;
use crate::session::Session;
use tokio::sync::Mutex;

pub struct RetryOperation<'a> {
    retry_manager: &'a RetryManager,
    goal: &'a Mutex<Option<String>>,
    grind: &'a Mutex<Option<String>>,
    session_config: SessionConfig,
    initial_messages: Vec<Message>,
    goal_nudged: AtomicBool,
    finished: AtomicBool,
}

impl<'a> RetryOperation<'a> {
    pub fn new(
        retry_manager: &'a RetryManager,
        goal: &'a Mutex<Option<String>>,
        grind: &'a Mutex<Option<String>>,
        session_config: SessionConfig,
        initial_messages: Vec<Message>,
    ) -> Self {
        Self {
            retry_manager,
            goal,
            grind,
            session_config,
            initial_messages,
            goal_nudged: AtomicBool::new(false),
            finished: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl Operation for RetryOperation<'_> {
    fn name(&self) -> &'static str {
        "retry"
    }

    async fn run_command(
        &self,
        command: &SlashCommand<'_>,
        _session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        let target = match command.command {
            "goal" => &self.goal,
            "grind" => &self.grind,
            _ => return not_applicable(emit),
        };
        let label = if command.command == "goal" {
            "goal"
        } else {
            "grind goal"
        };
        let params = command.params_str;
        let starts_turn = !params.is_empty() && !matches!(params, "off" | "clear" | "none");

        let response = if params.is_empty() {
            match target.lock().await.clone() {
                Some(value) => Message::assistant().with_text(format!("Current {label}: {value}")),
                None => Message::assistant().with_text(format!(
                    "No {label} set. Use `/{command_name} <description>` to set one.",
                    command_name = command.command
                )),
            }
        } else if !starts_turn {
            *target.lock().await = None;
            let text = if command.command == "goal" {
                "Goal cleared. The agent will finish normally."
            } else {
                "Grind cleared. The agent will finish normally."
            };
            Message::assistant().with_text(text)
        } else {
            *target.lock().await = Some(params.to_string());
            let text = if command.command == "goal" {
                format!(
                    "Goal set. The agent will verify this goal is met before finishing:\n\n> {params}"
                )
            } else {
                format!(
                    "Grind goal set. The agent will keep working until max_turns is reached:\n\n> {params}"
                )
            };
            Message::assistant().with_text(text)
        };

        let command_message = messages_since_kickoff(conversation)?
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("slash command conversation has no kickoff message"))?;
        let message_id = command_message
            .id
            .clone()
            .ok_or_else(|| anyhow!("Persisted slash command message has no id"))?;
        let command_message = command_message.with_visibility(true, false);
        let response = response.with_visibility(true, false);
        emit.emit(AgentEvent::Message(command_message)).await;
        emit.emit(AgentEvent::Message(response.clone())).await;

        let mut effects = vec![
            TurnEffect::SetMessageVisibility {
                message_id,
                user_visible: true,
                agent_visible: false,
            },
            response.into(),
        ];
        if starts_turn {
            effects.push(
                Message::user()
                    .with_text(format!(
                        "Start working toward this goal now:\n\n**Goal:** {params}"
                    ))
                    .with_visibility(false, true)
                    .into(),
            );
        } else {
            effects.push(TurnEffect::YieldToClient);
        }
        applied(effects)
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        if self.finished.load(Ordering::Relaxed)
            || !ends_turn(messages_since_kickoff(conversation)?)
        {
            return not_applicable(emit);
        }

        if !self.goal_nudged.load(Ordering::Relaxed) {
            if let Some(goal) = self.goal.lock().await.clone() {
                self.goal_nudged.store(true, Ordering::Relaxed);
                let nudge = format!(
                    "Before finishing, check whether the following goal has been fully met:\n\n\
                     **Goal:** {goal}\n\n\
                     If not, continue working toward it."
                );
                let message = Message::user()
                    .with_text(&nudge)
                    .with_visibility(false, true);
                emit.emit(AgentEvent::Message(
                    Message::assistant().with_system_notification(
                        SystemNotificationType::InlineMessage,
                        format!("Goal: {goal}"),
                    ),
                ))
                .await;
                return applied([message.into()]);
            }
        }

        if let Some(grind) = self.grind.lock().await.clone() {
            let nudge = format!(
                "Keep working. The grind goal is not yet complete:\n\n\
                 **Goal:** {grind}\n\n\
                 Continue until it is fully done."
            );
            let message = Message::user()
                .with_text(&nudge)
                .with_visibility(false, true);
            emit.emit(AgentEvent::Message(
                Message::assistant().with_system_notification(
                    SystemNotificationType::InlineMessage,
                    format!("Grind: {grind}"),
                ),
            ))
            .await;
            return applied([message.into()]);
        }

        *self.goal.lock().await = None;
        *self.grind.lock().await = None;

        if self.session_config.retry_config.is_none() {
            return not_applicable(emit);
        }

        let mut working = conversation.clone();
        match self
            .retry_manager
            .handle_retry_logic(&mut working, &self.session_config, &self.initial_messages)
            .await
        {
            Ok(RetryResult::Retried) => applied([working.into()]),
            Ok(RetryResult::MaxAttemptsReached(message)) => {
                self.finished.store(true, Ordering::Relaxed);
                emit.emit(AgentEvent::Message(message.clone())).await;
                applied([message.into()])
            }
            Ok(RetryResult::Skipped | RetryResult::SuccessChecksPassed) => not_applicable(emit),
            Err(e) => {
                self.finished.store(true, Ordering::Relaxed);
                let message = Message::assistant()
                    .with_text(format!("Retry logic encountered an error: {e}"));
                emit.emit(AgentEvent::Message(message.clone())).await;
                applied([message.into()])
            }
        }
    }
}
