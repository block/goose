use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::agents::state_machine::ops_recipe::RecipeOperation;
use crate::agents::state_machine::Emitter;
use crate::agents::AgentEvent;
use crate::execution::manager::AgentManager;
use crate::session::SessionManager;

pub(crate) struct ChildStepOutcome {
    pub(crate) progressed: bool,
    pub(crate) completed: Option<String>,
    pub(crate) applied_step: Option<&'static str>,
}

pub(crate) struct ChildExecutor {
    agent_manager: Arc<AgentManager>,
}

impl ChildExecutor {
    pub(crate) fn new(agent_manager: Arc<AgentManager>) -> Self {
        Self { agent_manager }
    }

    pub(crate) fn session_manager(&self) -> &SessionManager {
        self.agent_manager.session_manager()
    }

    pub(crate) async fn step(self: &Arc<Self>, child_session_id: &str) -> Result<ChildStepOutcome> {
        let cancel = CancellationToken::new();
        self.agent_manager
            .try_register_cancel_token(child_session_id, cancel.clone())
            .await?;

        let result = self.step_inner(child_session_id, cancel).await;
        self.agent_manager
            .unregister_cancel_token(child_session_id)
            .await;
        let removal = self.agent_manager.remove_session(child_session_id).await;
        match result {
            Ok(outcome) => {
                removal?;
                Ok(outcome)
            }
            Err(error) => {
                let _ = removal;
                Err(error)
            }
        }
    }

    async fn step_inner(
        self: &Arc<Self>,
        child_session_id: &str,
        cancel: CancellationToken,
    ) -> Result<ChildStepOutcome> {
        let session_manager = self.agent_manager.session_manager();
        let session = session_manager.get_session(child_session_id, true).await?;
        let conversation = session
            .conversation
            .as_ref()
            .ok_or_else(|| anyhow!("child session has no conversation"))?;
        if let Some(output) = RecipeOperation::successful_final_output(conversation.messages()) {
            return Ok(ChildStepOutcome {
                progressed: false,
                completed: Some(output),
                applied_step: None,
            });
        }

        let agent = self
            .agent_manager
            .get_or_create_agent(child_session_id.to_string())
            .await?;
        let provider = agent.provider().await?;
        let model_config = agent.model_config_for_session(child_session_id).await?;
        let context_limit =
            crate::context_limit::get_context_limit(provider.as_ref(), &model_config.model_name)
                .await?;
        let max_turns = session
            .recipe
            .as_ref()
            .and_then(|recipe| recipe.settings.as_ref())
            .and_then(|settings| settings.max_turns)
            .map(|turns| turns as u32);
        let steer_queue = agent.steer_queue(child_session_id).await;
        let machine = agent.create_state_machine(
            provider,
            model_config,
            context_limit,
            max_turns,
            cancel.clone(),
            steer_queue,
            Some(Arc::clone(self)),
        );

        let (tx, mut rx) = mpsc::channel::<AgentEvent>(32);
        let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });
        let emit = Emitter::new(tx, cancel);

        let mut result = machine.step(&session, &emit).await?;
        let applied_step = result.as_ref().and_then(|result| result.applied_step);
        if let Some(step_result) = result.as_mut() {
            machine
                .apply(session_manager, &session, step_result, &emit)
                .await?;
        }

        drop(emit);
        drain.abort();

        let updated = session_manager.get_session(child_session_id, true).await?;
        let completed = updated.conversation.as_ref().and_then(|conversation| {
            RecipeOperation::successful_final_output(conversation.messages())
        });

        Ok(ChildStepOutcome {
            progressed: result.is_some(),
            completed,
            applied_step,
        })
    }
}
