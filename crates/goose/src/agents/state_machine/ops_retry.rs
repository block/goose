use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::final_output_tool::FINAL_OUTPUT_CONTINUATION_MESSAGE;
use crate::agents::retry::RetryResult;
use crate::agents::state_machine::operation::{ends_turn, Emitter, Operation, OperationResult};
use crate::agents::types::SessionConfig;
use crate::agents::{Agent, AgentEvent};
use crate::conversation::message::{Message, SystemNotificationType};
use crate::conversation::Conversation;
use crate::session::Session;

pub struct RetryOperation<'a> {
    agent: &'a Agent,
    session_config: SessionConfig,
    initial_messages: Vec<Message>,
    goal_nudged: AtomicBool,
    finished: AtomicBool,
}

impl<'a> RetryOperation<'a> {
    pub fn new(
        agent: &'a Agent,
        session_config: SessionConfig,
        initial_messages: Vec<Message>,
    ) -> Self {
        Self {
            agent,
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

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        if self.finished.load(Ordering::Relaxed) || !ends_turn(conversation) {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let final_output = {
            let mut guard = self.agent.final_output_tool.lock().await;
            guard.as_mut().map(|tool| tool.final_output.take())
        };
        match final_output {
            Some(None) => {
                let message = Message::user().with_text(FINAL_OUTPUT_CONTINUATION_MESSAGE);
                emit.emit(AgentEvent::Message(message.clone())).await;
                return Ok(OperationResult::Applied(vec![message.into()]));
            }
            Some(Some(output)) => {
                self.finished.store(true, Ordering::Relaxed);
                let message = Message::assistant().with_text(output);
                emit.emit(AgentEvent::Message(message.clone())).await;
                return Ok(OperationResult::Applied(vec![message.into()]));
            }
            None => {}
        }

        if !self.goal_nudged.load(Ordering::Relaxed) {
            if let Some(goal) = self.agent.get_goal().await {
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
                return Ok(OperationResult::Applied(vec![message.into()]));
            }
        }

        if let Some(grind) = self.agent.get_grind().await {
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
            return Ok(OperationResult::Applied(vec![message.into()]));
        }

        self.agent.set_goal(None).await;
        self.agent.set_grind(None).await;

        if self.session_config.retry_config.is_none() {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let mut working = conversation.clone();
        match self
            .agent
            .retry_manager
            .handle_retry_logic(
                &mut working,
                &self.session_config,
                &self.initial_messages,
                &self.agent.final_output_tool,
            )
            .await
        {
            Ok(RetryResult::Retried) => Ok(OperationResult::Applied(vec![working.into()])),
            Ok(RetryResult::MaxAttemptsReached(message)) => {
                self.finished.store(true, Ordering::Relaxed);
                emit.emit(AgentEvent::Message(message.clone())).await;
                Ok(OperationResult::Applied(vec![message.into()]))
            }
            Ok(RetryResult::Skipped | RetryResult::SuccessChecksPassed) => {
                Ok(OperationResult::NotApplicable(emit))
            }
            Err(e) => {
                self.finished.store(true, Ordering::Relaxed);
                let message = Message::assistant()
                    .with_text(format!("Retry logic encountered an error: {e}"));
                emit.emit(AgentEvent::Message(message.clone())).await;
                Ok(OperationResult::Applied(vec![message.into()]))
            }
        }
    }
}
