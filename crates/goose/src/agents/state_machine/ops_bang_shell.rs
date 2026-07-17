use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{CallToolRequestParams, Role};

use crate::agents::state_machine::operation::{Emitter, Operation, OperationResult, TurnEffect};
use crate::agents::AgentEvent;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::session::Session;

const BANG_SHELL_META_KEY: &str = "goose.bangShell";
const SHELL_TOOL_NAME: &str = "developer__shell";

pub struct BangShellOperation;

impl BangShellOperation {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Operation for BangShellOperation {
    fn name(&self) -> &'static str {
        "bang_shell"
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        if bang_shell_answered(conversation) {
            return Ok(OperationResult::Applied(vec![TurnEffect::YieldToClient]));
        }

        let Some(command) = bang_shell_command(conversation) else {
            return Ok(OperationResult::NotApplicable(emit));
        };

        let call = CallToolRequestParams::new(SHELL_TOOL_NAME.to_string()).with_arguments(
            serde_json::Map::from_iter([(
                "command".to_string(),
                serde_json::Value::String(command),
            )]),
        );
        let request = Message::assistant()
            .with_generated_id()
            .with_tool_request_with_metadata(
                format!("bang_shell_{}", uuid::Uuid::now_v7()),
                Ok(call),
                None,
                Some(serde_json::json!({ BANG_SHELL_META_KEY: true })),
            );
        emit.emit(AgentEvent::Message(request.clone())).await;

        Ok(OperationResult::Applied(vec![request.into()]))
    }
}

fn bang_shell_command(conversation: &Conversation) -> Option<String> {
    let last = conversation.last()?;
    if last.role != Role::User || last.is_tool_response() {
        return None;
    }

    last.as_concat_text()
        .trim_start()
        .strip_prefix('!')
        .map(str::trim_start)
        .map(str::to_string)
        .filter(|command| !command.is_empty())
}

fn bang_shell_answered(conversation: &Conversation) -> bool {
    let messages = conversation.messages();
    let Some(last) = messages.last() else {
        return false;
    };
    if !last.is_tool_response() {
        return false;
    }

    let Some(request_id) = messages[..messages.len().saturating_sub(1)]
        .iter()
        .rev()
        .find(|message| message.role == Role::Assistant)
        .into_iter()
        .flat_map(|message| message.content.iter())
        .filter_map(|content| match content {
            MessageContent::ToolRequest(request)
                if request.tool_meta.as_ref().is_some_and(|meta| {
                    meta.get(BANG_SHELL_META_KEY).and_then(|v| v.as_bool()) == Some(true)
                }) =>
            {
                Some(request.id.as_str())
            }
            _ => None,
        })
        .next()
    else {
        return false;
    };

    last.get_tool_response_ids()
        .into_iter()
        .any(|id| id == request_id)
}
