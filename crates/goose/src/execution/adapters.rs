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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dynamic_task_adapter() {
        let manager = AgentManager::new();

        let parent = "parent-session-123".to_string();
        let instructions = "test task instructions".to_string();

        // Should create task and return ID
        let task_id = adapt_dynamic_task(&manager, parent.clone(), instructions.clone())
            .await
            .unwrap();

        assert!(!task_id.is_empty());

        // Should have created agent with SubTask mode
        let session = SessionId::from(task_id.clone());
        assert!(manager.has_session(&session).await);
    }

    #[tokio::test]
    async fn test_scheduler_adapter() {
        let manager = AgentManager::new();

        let job_id = "test-job-456".to_string();

        // Should execute without error
        let result = adapt_scheduler_job(&manager, job_id.clone()).await;
        assert!(result.is_ok());

        // Should have created session
        let session = SessionId::from(job_id);
        assert!(manager.has_session(&session).await);
    }

    #[tokio::test]
    async fn test_chat_adapter_with_session() {
        let manager = AgentManager::new();

        // With existing session ID
        let session_str = "existing-chat-789";
        let agent = adapt_chat_session(&manager, Some(session_str.to_string()))
            .await
            .unwrap();

        // Should have created the session
        let session = SessionId::from(session_str);
        assert!(manager.has_session(&session).await);

        // Getting same session should return same agent
        let agent2 = adapt_chat_session(&manager, Some(session_str.to_string()))
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&agent, &agent2));
    }

    #[tokio::test]
    async fn test_chat_adapter_without_session() {
        let manager = AgentManager::new();

        // Without session ID (should generate)
        let agent1 = adapt_chat_session(&manager, None).await.unwrap();
        let agent2 = adapt_chat_session(&manager, None).await.unwrap();

        // Should create different agents (different generated sessions)
        assert!(!Arc::ptr_eq(&agent1, &agent2));
    }

    #[tokio::test]
    async fn test_session_id_adapter() {
        // With existing ID
        let id = adapt_session_id(Some("test-123".to_string()));
        assert_eq!(id.as_str(), "test-123");

        // Without ID (should generate)
        let generated = adapt_session_id(None);
        assert_eq!(generated.as_str().len(), 36); // UUID length
    }
}
