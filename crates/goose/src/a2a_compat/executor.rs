//! GooseAgentExecutor: bridges A2A AgentExecutor to Goose Agent::reply().

use std::sync::Arc;

use a2a::server::context::RequestContext;
use a2a::server::executor::AgentExecutor;
use a2a::types::core::{Artifact, TaskState, TaskStatus};
use a2a::types::events::{AgentExecutionEvent, TaskArtifactUpdateEvent, TaskStatusUpdateEvent};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use super::message::goose_message_to_a2a;
use crate::agents::{Agent, AgentEvent, SessionConfig};

/// An A2A AgentExecutor that delegates to a Goose Agent.
pub struct GooseAgentExecutor {
    agent: Arc<Agent>,
    session_config: SessionConfig,
    cancel_token: CancellationToken,
}

impl GooseAgentExecutor {
    pub fn new(agent: Arc<Agent>, session_config: SessionConfig) -> Self {
        Self {
            agent,
            session_config,
            cancel_token: CancellationToken::new(),
        }
    }
}

#[async_trait::async_trait]
impl AgentExecutor for GooseAgentExecutor {
    async fn execute(
        &self,
        context: RequestContext,
        event_tx: mpsc::Sender<AgentExecutionEvent>,
    ) -> std::result::Result<(), a2a::A2AError> {
        let goose_msg = super::message::a2a_message_to_goose(&context.user_message);

        // Signal working
        let _ = event_tx
            .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                task_id: context.task_id.clone(),
                context_id: context.context_id.clone(),
                status: TaskStatus {
                    state: TaskState::Working,
                    message: None,
                    timestamp: None,
                },
                metadata: None,
            }))
            .await;

        let reply_result = self
            .agent
            .reply(
                goose_msg,
                self.session_config.clone(),
                Some(self.cancel_token.clone()),
            )
            .await;

        let mut stream = match reply_result {
            Ok(s) => s,
            Err(e) => {
                let _ = event_tx
                    .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                        task_id: context.task_id.clone(),
                        context_id: context.context_id.clone(),
                        status: TaskStatus {
                            state: TaskState::Failed,
                            message: None,
                            timestamp: None,
                        },
                        metadata: None,
                    }))
                    .await;
                return Err(a2a::A2AError::internal_error(e.to_string()));
            }
        };

        let mut artifact_index: u32 = 0;

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(AgentEvent::Message(msg)) => {
                    let a2a_msg = goose_message_to_a2a(&msg);
                    if a2a_msg.parts.is_empty() {
                        continue;
                    }

                    // Emit artifact for agent messages
                    if msg.role == rmcp::model::Role::Assistant {
                        let _ = event_tx
                            .send(AgentExecutionEvent::ArtifactUpdate(
                                TaskArtifactUpdateEvent {
                                    task_id: context.task_id.clone(),
                                    context_id: context.context_id.clone(),
                                    artifact: Artifact {
                                        artifact_id: format!("artifact-{}", artifact_index),
                                        name: None,
                                        description: None,
                                        parts: a2a_msg.parts,
                                        metadata: None,
                                        extensions: vec![],
                                    },
                                    append: false,
                                    last_chunk: false,
                                    metadata: None,
                                },
                            ))
                            .await;
                        artifact_index += 1;
                    }
                }
                Ok(_) => {
                    debug!("Skipping non-message AgentEvent in A2A executor");
                }
                Err(e) => {
                    warn!("Error in agent stream: {}", e);
                    let _ = event_tx
                        .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                            task_id: context.task_id.clone(),
                            context_id: context.context_id.clone(),
                            status: TaskStatus {
                                state: TaskState::Failed,
                                message: None,
                                timestamp: None,
                            },
                            metadata: None,
                        }))
                        .await;
                    return Err(a2a::A2AError::internal_error(e.to_string()));
                }
            }
        }

        // Signal completion
        let _ = event_tx
            .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                task_id: context.task_id.clone(),
                context_id: context.context_id.clone(),
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
    ) -> std::result::Result<(), a2a::A2AError> {
        self.cancel_token.cancel();
        Ok(())
    }
}
