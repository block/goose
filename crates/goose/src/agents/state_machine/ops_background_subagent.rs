use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::child_executor::ChildExecutor;
use crate::agents::state_machine::{
    applied, not_applicable, Emitter, GooseEffect, Operation, OperationResult,
    SPIKE_SUBAGENT_EXECUTION_EXTENSION, SPIKE_SUBAGENT_EXECUTION_VERSION,
};
use crate::conversation::Conversation;
use crate::session::{ExtensionData, Session, SessionManager, SessionType};

pub(crate) struct BackgroundSubagentOperation {
    child_executor: Arc<ChildExecutor>,
}

impl BackgroundSubagentOperation {
    pub(crate) fn new(child_executor: Arc<ChildExecutor>) -> Self {
        Self { child_executor }
    }

    fn mode(session: &Session) -> Option<&str> {
        session
            .extension_data
            .get_extension_state(
                SPIKE_SUBAGENT_EXECUTION_EXTENSION,
                SPIKE_SUBAGENT_EXECUTION_VERSION,
            )
            .and_then(|value| value.get("mode"))
            .and_then(serde_json::Value::as_str)
    }

    async fn persist_mode(
        session_manager: &SessionManager,
        child_session_id: &str,
        mut extension_data: ExtensionData,
        mode: &str,
    ) -> Result<()> {
        extension_data.set_extension_state(
            SPIKE_SUBAGENT_EXECUTION_EXTENSION,
            SPIKE_SUBAGENT_EXECUTION_VERSION,
            serde_json::json!({ "mode": mode }),
        );
        session_manager
            .update(child_session_id)
            .extension_data(extension_data)
            .apply()
            .await
    }

    async fn run_child(child_executor: Arc<ChildExecutor>, child_session_id: String) {
        let terminal_mode = loop {
            match child_executor.step(&child_session_id).await {
                Ok(outcome) if outcome.completed.is_some() => break "background_completed",
                Ok(outcome) if outcome.progressed => {}
                Ok(_) | Err(_) => break "background_failed",
            }
        };

        let session_manager = child_executor.session_manager();
        if let Ok(session) = session_manager.get_session(&child_session_id, false).await {
            let _ = Self::persist_mode(
                session_manager,
                &child_session_id,
                session.extension_data,
                terminal_mode,
            )
            .await;
        }
    }
}

#[async_trait]
impl Operation<Session, GooseEffect> for BackgroundSubagentOperation {
    fn name(&self) -> &'static str {
        "background_subagent"
    }

    async fn run(
        &self,
        session: &Session,
        _conversation: &Conversation,
        _emit: &Emitter,
    ) -> Result<OperationResult<GooseEffect>> {
        let children = self
            .child_executor
            .session_manager()
            .list_sessions_by_types(&[SessionType::SubAgent])
            .await?;
        let Some(child) = children.into_iter().find(|child| {
            child.parent_session_id.as_deref() == Some(session.id.as_str())
                && Self::mode(child) == Some("background")
        }) else {
            return not_applicable();
        };

        Self::persist_mode(
            self.child_executor.session_manager(),
            &child.id,
            child.extension_data,
            "background_running",
        )
        .await?;
        tokio::spawn(Self::run_child(Arc::clone(&self.child_executor), child.id));

        applied([])
    }
}
