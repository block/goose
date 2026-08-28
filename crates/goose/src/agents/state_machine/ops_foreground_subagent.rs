use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::child_executor::ChildExecutor;
use crate::agents::state_machine::{
    applied, not_applicable, Emitter, GooseEffect, Operation, OperationResult,
    SPIKE_SUBAGENT_EXECUTION_EXTENSION, SPIKE_SUBAGENT_EXECUTION_VERSION,
};
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::session::{Session, SessionType};

const DELIVERY_PREFIX: &str = "[subagent-result:";

pub(crate) struct ForegroundSubagentOperation {
    child_executor: Arc<ChildExecutor>,
}

impl ForegroundSubagentOperation {
    pub(crate) fn new(child_executor: Arc<ChildExecutor>) -> Self {
        Self { child_executor }
    }

    fn is_spike_foreground_child(session: &Session, parent_session_id: &str) -> bool {
        if session.parent_session_id.as_deref() != Some(parent_session_id) {
            return false;
        }
        session
            .extension_data
            .get_extension_state(
                SPIKE_SUBAGENT_EXECUTION_EXTENSION,
                SPIKE_SUBAGENT_EXECUTION_VERSION,
            )
            .and_then(|value| value.get("mode"))
            .and_then(serde_json::Value::as_str)
            == Some("foreground")
    }

    fn delivered(conversation: &Conversation, child_session_id: &str) -> bool {
        let marker = format!("{DELIVERY_PREFIX}{child_session_id}]");
        conversation.messages().iter().any(|message| {
            message
                .agent_visible_content()
                .as_concat_text()
                .contains(&marker)
        })
    }

    async fn child_result_message(
        &self,
        child_session_id: &str,
        output: String,
        emit: &Emitter,
    ) -> Message {
        emit.message(
            Message::user()
                .with_text(format!("{DELIVERY_PREFIX}{child_session_id}]\n{output}"))
                .with_visibility(false, true),
        )
        .await
    }
}

#[async_trait]
impl Operation<Session, GooseEffect> for ForegroundSubagentOperation {
    fn name(&self) -> &'static str {
        "foreground_subagent"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<GooseEffect>> {
        let children = self
            .child_executor
            .session_manager()
            .list_sessions_by_types(&[SessionType::SubAgent])
            .await?;
        let Some(child) = children.into_iter().find(|child| {
            Self::is_spike_foreground_child(child, &session.id)
                && !Self::delivered(conversation, &child.id)
        }) else {
            return not_applicable();
        };

        let outcome = self.child_executor.step(&child.id).await?;
        tracing::debug!(
            child_session_id = %child.id,
            step = outcome.applied_step,
            "advanced foreground spike child"
        );

        if let Some(output) = outcome.completed {
            let message = self.child_result_message(&child.id, output, emit).await;
            return applied([message.into()]);
        }

        if outcome.progressed {
            return applied([]);
        }

        not_applicable()
    }
}
