//! Bridges A2A protocol to Goose's Agent::reply().

use std::sync::Arc;

use a2a::error::A2AError;
use a2a::server::context::RequestContext;
use a2a::server::executor::AgentExecutor;
use a2a::types::{
    AgentExecutionEvent, Artifact, Part, Role as A2ARole, TaskArtifactUpdateEvent, TaskState,
    TaskStatus, TaskStatusUpdateEvent,
};
use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::a2a_compat::message::{a2a_message_to_goose, goose_content_to_a2a_part};
use crate::agents::{Agent, AgentEvent};
use crate::execution::manager::AgentManager;

/// Executor that delegates A2A requests to a Goose Agent.
///
/// Creates a fresh agent session per A2A task via the `AgentManager`.
pub struct GooseAgentExecutor {
    agent_manager: Arc<AgentManager>,
    cancel_token: CancellationToken,
}

impl GooseAgentExecutor {
    pub fn new(agent_manager: Arc<AgentManager>) -> Self {
        Self {
            agent_manager,
            cancel_token: CancellationToken::new(),
        }
    }

    async fn get_agent(&self, session_id: &str) -> Result<Arc<Agent>, A2AError> {
        self.agent_manager
            .get_or_create_agent(session_id.to_string())
            .await
            .map_err(|e| A2AError::InternalError {
                message: format!("Failed to create agent: {e}"),
            })
    }
}

fn make_a2a_error_msg(task_id: &str, context_id: &str, text: String) -> Box<a2a::types::Message> {
    Box::new(a2a::types::Message {
        message_id: uuid::Uuid::new_v4().to_string(),
        role: A2ARole::Agent,
        parts: vec![Part::text(text)],
        context_id: Some(context_id.to_string()),
        task_id: Some(task_id.to_string()),
        metadata: None,
        extensions: Vec::new(),
        reference_task_ids: Vec::new(),
    })
}

#[async_trait]
impl AgentExecutor for GooseAgentExecutor {
    async fn execute(
        &self,
        context: RequestContext,
        event_tx: mpsc::Sender<AgentExecutionEvent>,
    ) -> Result<(), A2AError> {
        let goose_msg = a2a_message_to_goose(&context.user_message);

        let session_id = if context.task_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            context.task_id.clone()
        };

        let agent = self.get_agent(&session_id).await?;

        let session_config = crate::agents::SessionConfig {
            id: session_id,
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let task_id = context.task_id.clone();
        let context_id = context.context_id.clone();

        // Signal working state.
        event_tx
            .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                task_id: task_id.clone(),
                context_id: context_id.clone(),
                status: TaskStatus {
                    state: TaskState::Working,
                    message: None,
                    timestamp: None,
                },
                metadata: None,
            }))
            .await
            .map_err(|e| A2AError::InternalError {
                message: e.to_string(),
            })?;

        let stream_result = agent
            .reply(goose_msg, session_config, Some(self.cancel_token.clone()))
            .await;

        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                let _ = event_tx
                    .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                        task_id: task_id.clone(),
                        context_id: context_id.clone(),
                        status: TaskStatus {
                            state: TaskState::Failed,
                            message: Some(make_a2a_error_msg(&task_id, &context_id, e.to_string())),
                            timestamp: None,
                        },
                        metadata: None,
                    }))
                    .await;
                return Ok(());
            }
        };

        let mut artifact_idx: u32 = 0;
        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(AgentEvent::Message(msg)) => {
                    if matches!(msg.role, rmcp::model::Role::Assistant) {
                        let parts: Vec<Part> = msg
                            .content
                            .iter()
                            .filter_map(goose_content_to_a2a_part)
                            .collect();

                        if !parts.is_empty() {
                            let artifact = Artifact {
                                artifact_id: format!("artifact-{artifact_idx}"),
                                name: None,
                                description: None,
                                parts,
                                metadata: None,
                                extensions: Vec::new(),
                            };
                            artifact_idx += 1;

                            let _ = event_tx
                                .send(AgentExecutionEvent::ArtifactUpdate(
                                    TaskArtifactUpdateEvent {
                                        task_id: task_id.clone(),
                                        context_id: context_id.clone(),
                                        artifact,
                                        append: false,
                                        last_chunk: false,
                                        metadata: None,
                                    },
                                ))
                                .await;
                        }
                    }
                }
                Err(e) => {
                    let _ = event_tx
                        .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                            task_id: task_id.clone(),
                            context_id: context_id.clone(),
                            status: TaskStatus {
                                state: TaskState::Failed,
                                message: Some(make_a2a_error_msg(
                                    &task_id,
                                    &context_id,
                                    e.to_string(),
                                )),
                                timestamp: None,
                            },
                            metadata: None,
                        }))
                        .await;
                    return Ok(());
                }
                _ => {} // Skip non-message events
            }
        }

        // Stream completed — signal success.
        let _ = event_tx
            .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                task_id,
                context_id,
                status: TaskStatus {
                    state: TaskState::Completed,
                    message: None,
                    timestamp: None,
                },
                metadata: None,
            }))
            .await;

        Ok(())
    }

    async fn cancel(
        &self,
        _task_id: &str,
        _event_tx: mpsc::Sender<AgentExecutionEvent>,
    ) -> Result<(), A2AError> {
        self.cancel_token.cancel();
        Ok(())
    }
}
