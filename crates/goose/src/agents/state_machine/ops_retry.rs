//! Decides whether a completed response should finish, continue, or retry the turn.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use crate::agents::retry::{
    execute_on_failure_command_with_timeout, execute_success_checks_with_timeout,
};
use crate::agents::state_machine::operation::{
    applied, ends_turn, messages_since_kickoff, not_applicable, yielded_with, Emitter, Operation,
    OperationResult, SlashCommand, StateEffect,
};
use crate::agents::types::RetryConfig;
use crate::conversation::message::{Message, SystemNotificationType};
use crate::conversation::Conversation;
use crate::session::Session;
use tokio::sync::Mutex;

pub struct RetryOperation<'a> {
    goal: &'a Mutex<Option<String>>,
    grind: &'a Mutex<Option<String>>,
    retry_timeout: Duration,
    on_failure_timeout: Duration,
    attempts: AtomicU32,
    goal_nudged: AtomicBool,
    finished: AtomicBool,
}

impl<'a> RetryOperation<'a> {
    pub fn new(
        goal: &'a Mutex<Option<String>>,
        grind: &'a Mutex<Option<String>>,
        retry_timeout: Duration,
        on_failure_timeout: Duration,
    ) -> Self {
        Self {
            goal,
            grind,
            retry_timeout,
            on_failure_timeout,
            attempts: AtomicU32::new(0),
            goal_nudged: AtomicBool::new(false),
            finished: AtomicBool::new(false),
        }
    }

    fn retry_config(session: &Session) -> Option<&RetryConfig> {
        session
            .recipe
            .as_ref()
            .and_then(|recipe| recipe.retry.as_ref())
    }

    fn reset_conversation(conversation: &Conversation) -> Result<Conversation> {
        let messages = messages_since_kickoff(conversation)?;
        let kickoff = conversation.len() - messages.len();
        Ok(Conversation::new_unvalidated(
            conversation.messages()[..=kickoff].to_vec(),
        ))
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
        emit: &Emitter,
    ) -> Result<OperationResult> {
        let target = match command.command {
            "goal" => &self.goal,
            "grind" => &self.grind,
            _ => return not_applicable(),
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
        emit.message(command_message).await;
        let response = emit.message(response).await;

        let mut effects = vec![
            StateEffect::SetMessageVisibility {
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
            return yielded_with(effects);
        }
        applied(effects)
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        if self.finished.load(Ordering::Relaxed)
            || !ends_turn(messages_since_kickoff(conversation)?)
        {
            return not_applicable();
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
                emit.message(Message::assistant().with_system_notification(
                    SystemNotificationType::InlineMessage,
                    format!("Goal: {goal}"),
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
            emit.message(Message::assistant().with_system_notification(
                SystemNotificationType::InlineMessage,
                format!("Grind: {grind}"),
            ))
            .await;
            return applied([message.into()]);
        }

        *self.goal.lock().await = None;
        *self.grind.lock().await = None;

        let Some(retry_config) = Self::retry_config(session) else {
            return not_applicable();
        };

        let retry_timeout = retry_config
            .timeout_seconds
            .map(Duration::from_secs)
            .unwrap_or(self.retry_timeout);
        let success =
            execute_success_checks_with_timeout(&retry_config.checks, retry_timeout).await;
        let success = match success {
            Ok(success) => success,
            Err(error) => {
                self.finished.store(true, Ordering::Relaxed);
                let message = Message::assistant()
                    .with_text(format!("Retry logic encountered an error: {error}"));
                let message = emit.message(message).await;
                return applied([message.into()]);
            }
        };
        if success {
            return not_applicable();
        }

        if self.attempts.load(Ordering::Relaxed) >= retry_config.max_retries {
            self.finished.store(true, Ordering::Relaxed);
            let message = Message::assistant().with_text(format!(
                "Maximum retry attempts ({}) exceeded. Unable to complete the task successfully.",
                retry_config.max_retries
            ));
            #[cfg(feature = "telemetry")]
            crate::posthog::emit_error(
                "retry_max_exceeded",
                &format!("Max retries ({}) exceeded", retry_config.max_retries),
            );
            let message = emit.message(message).await;
            return applied([message.into()]);
        }

        if let Some(command) = &retry_config.on_failure {
            let timeout = retry_config
                .on_failure_timeout_seconds
                .map(Duration::from_secs)
                .unwrap_or(self.on_failure_timeout);
            if let Err(error) = execute_on_failure_command_with_timeout(command, timeout).await {
                self.finished.store(true, Ordering::Relaxed);
                let message = Message::assistant()
                    .with_text(format!("Retry logic encountered an error: {error}"));
                let message = emit.message(message).await;
                return applied([message.into()]);
            }
        }

        self.attempts.fetch_add(1, Ordering::Relaxed);
        applied([Self::reset_conversation(conversation)?.into()])
    }
}
