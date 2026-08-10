//! Handles a recognized slash command before the normal agent turn begins.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::operation::{
    messages_since_kickoff, not_applicable, yielded_with, Emitter, Operation, OperationResult,
    SlashCommand, StateEffect,
};
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::session::Session;

pub struct SlashCommandOperation<'a> {
    operations: Vec<Arc<dyn Operation + 'a>>,
}

fn parse_slash_command(message: &str) -> Option<SlashCommand<'_>> {
    let mut message = message.trim();
    if matches!(
        message,
        "/compact" | "Please compact this conversation" | "/summarize"
    ) {
        message = "/compact";
    }
    let command = message.strip_prefix('/')?;
    let (command, params_str) = command
        .split_once(' ')
        .map(|(command, params)| (command, params.trim()))
        .unwrap_or((command, ""));
    Some(SlashCommand {
        command,
        params_str,
    })
}

impl<'a> SlashCommandOperation<'a> {
    pub fn new(operations: Vec<Arc<dyn Operation + 'a>>) -> Self {
        Self { operations }
    }
}

#[async_trait]
impl Operation for SlashCommandOperation<'_> {
    fn name(&self) -> &'static str {
        "slash_command"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        let messages = messages_since_kickoff(conversation)?;
        let Some(user_message) = messages.first() else {
            return not_applicable();
        };
        let message_text = user_message.as_concat_text();
        if messages.len() != 1 {
            return not_applicable();
        }
        let Some(command) = parse_slash_command(&message_text) else {
            return not_applicable();
        };

        for operation in &self.operations {
            match operation
                .run_command(&command, session, conversation, emit)
                .await?
            {
                OperationResult::NotApplicable => {}
                applied @ OperationResult::Applied(_) => return Ok(applied),
            }
        }

        let message_id = user_message
            .id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Persisted slash command message has no id"))?;
        emit.message(user_message.clone().with_visibility(true, false))
            .await;
        let response = emit
            .message(
                Message::assistant()
                    .with_text(format!(
                        "Unknown or unavailable slash command: /{}",
                        command.command
                    ))
                    .with_visibility(true, false),
            )
            .await;
        yielded_with([
            StateEffect::SetMessageVisibility {
                message_id,
                user_visible: true,
                agent_visible: false,
            },
            response.into(),
        ])
    }
}
