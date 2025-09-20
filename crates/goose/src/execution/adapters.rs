//! Adapters for backward compatibility with existing code paths
//!
//! These adapters allow existing code to work unchanged while routing through
//! the new unified AgentManager. They will be gradually removed as code is
//! migrated to use AgentManager directly.

use super::{ExecutionMode, SessionId};
use crate::agents::Agent;
use crate::execution::manager::AgentManager;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info};

/// Adapt current dynamic task system to use AgentManager
///
/// This maintains backward compatibility with the existing dynamic task
/// creation while routing through the unified execution system.
pub async fn adapt_dynamic_task(
    manager: &AgentManager,
    parent_session: String,
    instructions: String,
) -> Result<String> {
    let task_id = SessionId::generate();
    let mode = ExecutionMode::task(parent_session.clone());

    info!(
        "Adapting dynamic task with parent {} as session {}",
        parent_session, task_id
    );

    // Get agent through new system
    let _agent = manager.get_agent(task_id.clone(), mode).await?;

    // In future, this will create a SubAgent and execute
    // For now, we're focusing on the structure
    debug!("Would execute instructions: {}", instructions);

    Ok(task_id.0)
}

/// Adapt current scheduler to use AgentManager
///
/// This allows scheduled jobs to run through the unified system while
/// maintaining compatibility with the existing scheduler interface.
///
/// Note: This is a placeholder that will be implemented when we integrate
/// with the actual scheduler. For now it demonstrates the pattern.
pub async fn adapt_scheduler_job(manager: &AgentManager, job_id: String) -> Result<()> {
    let session_id = SessionId::from(job_id.clone());
    let mode = ExecutionMode::scheduled();

    info!(
        "Adapting scheduled job {} as session {}",
        job_id, session_id
    );

    // Get agent through new system
    let _agent = manager.get_agent(session_id, mode).await?;

    // Use existing execution (unchanged for now)
    // In real implementation, this would call the actual job execution
    debug!("Would execute scheduled job with agent");

    Ok(())
}

/// Adapt goose-server chat sessions to use AgentManager
///
/// This provides a compatibility layer for the server routes to get
/// agents through the new system.
pub async fn adapt_chat_session(
    manager: &AgentManager,
    session_id: Option<String>,
) -> Result<Arc<Agent>> {
    let id = session_id
        .map(SessionId::from)
        .unwrap_or_else(SessionId::generate);

    info!("Adapting chat session as {}", id);

    manager.get_agent(id, ExecutionMode::chat()).await
}

/// Adapt a session identifier string to the new SessionId type
///
/// Helper for gradual migration of code using string session IDs
pub fn adapt_session_id(session: Option<String>) -> SessionId {
    session
        .map(SessionId::from)
        .unwrap_or_else(SessionId::generate)
}
