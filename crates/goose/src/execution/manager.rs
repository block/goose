//! Agent lifecycle management with session isolation

use super::{ExecutionMode, SessionId};
use crate::agents::Agent;
use crate::scheduler_trait::SchedulerTrait;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Manages agents with session awareness and isolation
pub struct AgentManager {
    /// Active sessions mapped to their agents and metadata
    sessions: Arc<RwLock<HashMap<SessionId, SessionData>>>,

    /// Shared scheduler for background tasks (optional)
    scheduler: Arc<RwLock<Option<Arc<dyn SchedulerTrait>>>>,

    /// Maximum number of concurrent sessions
    max_sessions: usize,
}

/// Metadata about an active session
struct SessionData {
    /// The agent instance for this session
    agent: Arc<Agent>,

    /// Execution mode for this session
    #[allow(dead_code)]
    mode: ExecutionMode,

    /// When the session was created
    #[allow(dead_code)]
    created_at: DateTime<Utc>,

    /// Last time the session was accessed
    last_used: DateTime<Utc>,
}

impl AgentManager {
    /// Create a new AgentManager with default settings
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            scheduler: Arc::new(RwLock::new(None)),
            max_sessions: 100, // Default limit
        }
    }

    /// Create with custom max sessions limit
    pub fn with_max_sessions(max_sessions: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            scheduler: Arc::new(RwLock::new(None)),
            max_sessions,
        }
    }

    /// Set the scheduler to be used for background tasks
    pub async fn set_scheduler(&self, scheduler: Arc<dyn SchedulerTrait>) {
        debug!("Setting scheduler on AgentManager");
        *self.scheduler.write().await = Some(scheduler);
    }

    /// Get or create an agent for the given session
    pub async fn get_agent(
        &self,
        session_id: SessionId,
        mode: ExecutionMode,
    ) -> Result<Arc<Agent>> {
        // First check if we already have this session
        {
            let sessions = self.sessions.read().await;
            if let Some(data) = sessions.get(&session_id) {
                debug!("Found existing agent for session {}", session_id);
                let agent = Arc::clone(&data.agent);
                // Drop the lock before calling touch_session
                drop(sessions);
                self.touch_session(&session_id).await;
                return Ok(agent);
            }
        }

        // Need to create a new agent
        let mut sessions = self.sessions.write().await;

        // Double-check after acquiring write lock (another thread might have created it)
        if let Some(data) = sessions.get(&session_id) {
            debug!(
                "Found existing agent for session {} (after write lock)",
                session_id
            );
            return Ok(Arc::clone(&data.agent));
        }

        info!(
            "Creating new agent for session {} with mode {}",
            session_id, mode
        );

        // Enforce session limit
        if sessions.len() >= self.max_sessions {
            warn!(
                "Session limit reached ({}), evicting oldest session",
                self.max_sessions
            );
            self.evict_oldest_session(&mut sessions);
        }

        // Create new agent
        let agent = Arc::new(Agent::new());

        // Configure based on execution mode
        match &mode {
            ExecutionMode::Interactive | ExecutionMode::Background => {
                // These modes might need the scheduler
                if let Some(scheduler) = &*self.scheduler.read().await {
                    debug!("Setting scheduler on agent for session {}", session_id);
                    agent.set_scheduler(Arc::clone(scheduler)).await;
                }
            }
            ExecutionMode::SubTask { .. } => {
                // Sub-tasks typically don't need direct scheduler access
                debug!(
                    "SubTask mode for session {}, skipping scheduler setup",
                    session_id
                );
            }
        }

        // Store the new session
        let now = Utc::now();
        sessions.insert(
            session_id.clone(),
            SessionData {
                agent: Arc::clone(&agent),
                mode,
                created_at: now,
                last_used: now,
            },
        );

        Ok(agent)
    }

    /// Remove a specific session
    pub async fn remove_session(&self, session_id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions
            .remove(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;
        info!("Removed session {}", session_id);
        Ok(())
    }

    /// Check if a session exists
    pub async fn has_session(&self, session_id: &SessionId) -> bool {
        self.sessions.read().await.contains_key(session_id)
    }

    /// Get the number of active sessions
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Update the last_used timestamp for a session
    async fn touch_session(&self, session_id: &SessionId) {
        let mut sessions = self.sessions.write().await;
        if let Some(data) = sessions.get_mut(session_id) {
            data.last_used = Utc::now();
        }
    }

    /// Remove the oldest session (by last_used time)
    fn evict_oldest_session(&self, sessions: &mut HashMap<SessionId, SessionData>) {
        if let Some((oldest_id, _)) = sessions
            .iter()
            .min_by_key(|(_, data)| data.last_used)
            .map(|(id, data)| (id.clone(), data.last_used))
        {
            info!("Evicting oldest session: {}", oldest_id);
            sessions.remove(&oldest_id);
        }
    }

    /// Future-ready execution method (stub for now)
    /// This will become the unified pipeline for recipes, tasks, and scheduled jobs
    pub async fn execute_recipe(
        &self,
        session_id: SessionId,
        _recipe: serde_json::Value,
        mode: ExecutionMode,
    ) -> Result<serde_json::Value> {
        // For now, just ensure agent creation works
        let _agent = self.get_agent(session_id.clone(), mode).await?;

        // Future: This will become the unified execution pipeline
        // - Parse recipe
        // - Configure agent with recipe settings
        // - Execute recipe steps
        // - Return results

        Ok(serde_json::json!({
            "status": "ready_for_future",
            "agent_created": true,
            "session_id": session_id.to_string(),
        }))
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_isolation() {
        let manager = AgentManager::new();

        let session1 = SessionId::generate();
        let session2 = SessionId::generate();

        // Get agents for different sessions
        let agent1 = manager
            .get_agent(session1.clone(), ExecutionMode::chat())
            .await
            .unwrap();
        let agent2 = manager
            .get_agent(session2.clone(), ExecutionMode::chat())
            .await
            .unwrap();

        // Should be different agents
        assert!(!Arc::ptr_eq(&agent1, &agent2));

        // Getting same session should return same agent
        let agent1_again = manager
            .get_agent(session1, ExecutionMode::chat())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&agent1, &agent1_again));
    }

    #[tokio::test]
    async fn test_execution_modes() {
        let manager = AgentManager::new();

        // Create agents with different modes
        let interactive = manager
            .get_agent(SessionId::generate(), ExecutionMode::Interactive)
            .await
            .unwrap();

        let background = manager
            .get_agent(SessionId::generate(), ExecutionMode::Background)
            .await
            .unwrap();

        let subtask = manager
            .get_agent(
                SessionId::generate(),
                ExecutionMode::SubTask {
                    parent_session: "parent-123".to_string(),
                },
            )
            .await
            .unwrap();

        // All should be different agents
        assert!(!Arc::ptr_eq(&interactive, &background));
        assert!(!Arc::ptr_eq(&background, &subtask));
        assert!(!Arc::ptr_eq(&interactive, &subtask));
    }

    #[tokio::test]
    async fn test_session_limit() {
        let manager = AgentManager::with_max_sessions(3);

        // Create 3 sessions
        let sessions: Vec<_> = (0..3)
            .map(|i| SessionId::from(format!("session-{}", i)))
            .collect();

        for session in &sessions {
            manager
                .get_agent(session.clone(), ExecutionMode::chat())
                .await
                .unwrap();
        }

        assert_eq!(manager.session_count().await, 3);

        // Creating 4th should evict oldest
        let new_session = SessionId::from("session-new");
        manager
            .get_agent(new_session, ExecutionMode::chat())
            .await
            .unwrap();

        // Should still have only 3 sessions
        assert_eq!(manager.session_count().await, 3);

        // First session should have been evicted
        assert!(!manager.has_session(&sessions[0]).await);
    }

    #[tokio::test]
    async fn test_remove_session() {
        let manager = AgentManager::new();
        let session = SessionId::from("remove-test");

        // Create session
        manager
            .get_agent(session.clone(), ExecutionMode::chat())
            .await
            .unwrap();
        assert!(manager.has_session(&session).await);

        // Remove it
        manager.remove_session(&session).await.unwrap();
        assert!(!manager.has_session(&session).await);

        // Removing again should error
        assert!(manager.remove_session(&session).await.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        use std::sync::Arc;

        let manager = Arc::new(AgentManager::new());
        let session = SessionId::from("concurrent-test");

        // Spawn multiple tasks accessing the same session
        let mut handles = vec![];
        for _ in 0..10 {
            let mgr = Arc::clone(&manager);
            let sess = session.clone();
            let handle =
                tokio::spawn(
                    async move { mgr.get_agent(sess, ExecutionMode::chat()).await.unwrap() },
                );
            handles.push(handle);
        }

        // Collect all agents
        let agents: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All should be the same agent
        for agent in &agents[1..] {
            assert!(Arc::ptr_eq(&agents[0], agent));
        }

        // Only one session should exist
        assert_eq!(manager.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_execute_recipe_stub() {
        let manager = AgentManager::new();
        let session = SessionId::generate();

        let recipe = serde_json::json!({
            "name": "test_recipe",
            "instructions": "test"
        });

        let result = manager
            .execute_recipe(session.clone(), recipe, ExecutionMode::Interactive)
            .await
            .unwrap();

        // Verify stub response
        assert_eq!(result["status"], "ready_for_future");
        assert_eq!(result["agent_created"], true);
        assert_eq!(result["session_id"], session.to_string());
    }
}
