//! Agent executor trait — the core abstraction for agent execution logic.

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::A2AError;
use crate::types::events::AgentExecutionEvent;

use super::context::RequestContext;

/// Trait for agent execution logic.
///
/// Implementors receive a `RequestContext` and emit events through the `mpsc::Sender`.
/// The executor runs on a background task; events flow to the `DefaultRequestHandler`.
#[async_trait]
pub trait AgentExecutor: Send + Sync {
    /// Execute the agent logic for the given request context.
    async fn execute(
        &self,
        context: RequestContext,
        event_sender: mpsc::Sender<AgentExecutionEvent>,
    ) -> Result<(), A2AError>;

    /// Cancel an in-progress task.
    async fn cancel(
        &self,
        task_id: &str,
        event_sender: mpsc::Sender<AgentExecutionEvent>,
    ) -> Result<(), A2AError>;
}
