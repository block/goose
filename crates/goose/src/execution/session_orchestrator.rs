//! Autonomous session management: fork, park, and resume sessions by topic.
//!
//! Agents can autonomously organize their work into topic-based sessions:
//! - **Fork**: Create a child session from a parent, inheriting context
//! - **Park**: Suspend a session when blocked or switching topics
//! - **Resume**: Pick up a parked session to continue work
//!
//! Session state is stored in the session's `extension_data` under the
//! `"session_orchestrator"` key, avoiding schema migrations.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::session::session_manager::{SessionManager, SessionType};

/// Session lifecycle state managed by the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Session is actively being worked on.
    Active,
    /// Session is suspended, waiting to be resumed.
    Parked {
        reason: String,
        parked_at: DateTime<Utc>,
    },
    /// Session has completed its work.
    Completed {
        summary: String,
        completed_at: DateTime<Utc>,
    },
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Active => write!(f, "active"),
            SessionState::Parked { reason, .. } => write!(f, "parked: {reason}"),
            SessionState::Completed { summary, .. } => write!(f, "completed: {summary}"),
        }
    }
}

/// Metadata stored in session extension_data under "session_orchestrator".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOrchestratorData {
    pub state: SessionState,
    pub topic: String,
    pub parent_session_id: Option<String>,
    pub child_session_ids: Vec<String>,
    pub budget: SessionBudget,
    pub history: Vec<StateTransition>,
}

/// Budget controls for autonomous session management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBudget {
    /// Maximum number of child sessions that can be forked.
    pub max_children: u32,
    /// Maximum total tokens across all sessions in this tree.
    pub max_total_tokens: Option<i64>,
    /// Maximum depth of session forking.
    pub max_depth: u32,
}

impl Default for SessionBudget {
    fn default() -> Self {
        Self {
            max_children: 5,
            max_total_tokens: None,
            max_depth: 3,
        }
    }
}

/// Record of a state transition for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: SessionState,
    pub to: SessionState,
    pub timestamp: DateTime<Utc>,
}

/// A snapshot of a managed session for external queries.
#[derive(Debug, Clone, Serialize)]
pub struct ManagedSessionInfo {
    pub session_id: String,
    pub topic: String,
    pub state: SessionState,
    pub parent_session_id: Option<String>,
    pub child_count: usize,
    pub depth: u32,
}

const ORCHESTRATOR_NAME: &str = "session_orchestrator";
const ORCHESTRATOR_VERSION: &str = "v1";

/// Manages autonomous session lifecycle: fork, park, resume.
///
/// State is persisted in each session's `extension_data` field,
/// and an in-memory index provides fast lookups by state.
pub struct SessionOrchestrator {
    session_manager: Arc<SessionManager>,
    /// In-memory index: session_id → (topic, state)
    index: RwLock<HashMap<String, (String, SessionState)>>,
}

impl SessionOrchestrator {
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self {
            session_manager,
            index: RwLock::new(HashMap::new()),
        }
    }

    /// Register an existing session with the orchestrator.
    pub async fn register(
        &self,
        session_id: &str,
        topic: String,
        budget: Option<SessionBudget>,
    ) -> Result<()> {
        let data = SessionOrchestratorData {
            state: SessionState::Active,
            topic: topic.clone(),
            parent_session_id: None,
            child_session_ids: Vec::new(),
            budget: budget.unwrap_or_default(),
            history: Vec::new(),
        };

        self.persist_data(session_id, &data).await?;
        self.index
            .write()
            .await
            .insert(session_id.to_string(), (topic, SessionState::Active));
        Ok(())
    }

    /// Fork a child session from a parent, inheriting topic context.
    pub async fn fork(
        &self,
        parent_session_id: &str,
        child_topic: String,
        child_budget: Option<SessionBudget>,
    ) -> Result<String> {
        let mut parent_data = self.load_data(parent_session_id).await?;

        // Check budget
        if parent_data.child_session_ids.len() as u32 >= parent_data.budget.max_children {
            bail!(
                "Cannot fork: parent {} already has {} children (max {})",
                parent_session_id,
                parent_data.child_session_ids.len(),
                parent_data.budget.max_children
            );
        }

        // Check depth
        let depth = self.get_depth(parent_session_id).await?;
        if depth >= parent_data.budget.max_depth {
            bail!(
                "Cannot fork: depth {} exceeds max depth {}",
                depth,
                parent_data.budget.max_depth
            );
        }

        // Create child session via SessionManager
        let child_session = self
            .session_manager
            .create_session(
                std::path::PathBuf::from("."),
                child_topic.clone(),
                SessionType::User,
            )
            .await?;
        let child_id = child_session.id.clone();

        // Set up child data
        let child_data = SessionOrchestratorData {
            state: SessionState::Active,
            topic: child_topic.clone(),
            parent_session_id: Some(parent_session_id.to_string()),
            child_session_ids: Vec::new(),
            budget: child_budget.unwrap_or(SessionBudget {
                max_children: parent_data.budget.max_children.saturating_sub(1),
                max_total_tokens: parent_data.budget.max_total_tokens,
                max_depth: parent_data.budget.max_depth.saturating_sub(1),
            }),
            history: Vec::new(),
        };

        self.persist_data(&child_id, &child_data).await?;

        // Update parent
        parent_data.child_session_ids.push(child_id.clone());
        self.persist_data(parent_session_id, &parent_data).await?;

        // Update index
        let mut index = self.index.write().await;
        index.insert(child_id.clone(), (child_topic, SessionState::Active));

        Ok(child_id)
    }

    /// Park a session — suspend it with a reason.
    pub async fn park(&self, session_id: &str, reason: String) -> Result<()> {
        let mut data = self.load_data(session_id).await?;

        let old_state = data.state.clone();
        let new_state = SessionState::Parked {
            reason: reason.clone(),
            parked_at: Utc::now(),
        };

        data.history.push(StateTransition {
            from: old_state,
            to: new_state.clone(),
            timestamp: Utc::now(),
        });
        data.state = new_state.clone();

        self.persist_data(session_id, &data).await?;
        self.index
            .write()
            .await
            .insert(session_id.to_string(), (data.topic, new_state));

        Ok(())
    }

    /// Resume a parked session — make it active again.
    pub async fn resume(&self, session_id: &str) -> Result<()> {
        let mut data = self.load_data(session_id).await?;

        if !matches!(data.state, SessionState::Parked { .. }) {
            bail!(
                "Cannot resume: session {} is not parked (state: {})",
                session_id,
                data.state
            );
        }

        let old_state = data.state.clone();
        let new_state = SessionState::Active;

        data.history.push(StateTransition {
            from: old_state,
            to: new_state.clone(),
            timestamp: Utc::now(),
        });
        data.state = new_state.clone();

        self.persist_data(session_id, &data).await?;
        self.index
            .write()
            .await
            .insert(session_id.to_string(), (data.topic, new_state));

        Ok(())
    }

    /// Complete a session with a summary.
    pub async fn complete(&self, session_id: &str, summary: String) -> Result<()> {
        let mut data = self.load_data(session_id).await?;

        let old_state = data.state.clone();
        let new_state = SessionState::Completed {
            summary,
            completed_at: Utc::now(),
        };

        data.history.push(StateTransition {
            from: old_state,
            to: new_state.clone(),
            timestamp: Utc::now(),
        });
        data.state = new_state.clone();

        self.persist_data(session_id, &data).await?;
        self.index
            .write()
            .await
            .insert(session_id.to_string(), (data.topic, new_state));

        Ok(())
    }

    /// List all managed sessions, optionally filtered by state.
    pub async fn list(&self, state_filter: Option<&str>) -> Vec<ManagedSessionInfo> {
        let index = self.index.read().await;
        index
            .iter()
            .filter(|(_, (_, state))| {
                state_filter.is_none_or(|filter| match filter {
                    "active" => matches!(state, SessionState::Active),
                    "parked" => matches!(state, SessionState::Parked { .. }),
                    "completed" => matches!(state, SessionState::Completed { .. }),
                    _ => true,
                })
            })
            .map(|(id, (topic, state))| ManagedSessionInfo {
                session_id: id.clone(),
                topic: topic.clone(),
                state: state.clone(),
                parent_session_id: None, // Would need load_data for full info
                child_count: 0,
                depth: 0,
            })
            .collect()
    }

    /// Get the highest-priority parked session (FIFO — oldest parked first).
    pub async fn next_resumable(&self) -> Option<ManagedSessionInfo> {
        let index = self.index.read().await;
        index
            .iter()
            .filter_map(|(id, (topic, state))| {
                if let SessionState::Parked { parked_at, .. } = state {
                    Some((id.clone(), topic.clone(), state.clone(), *parked_at))
                } else {
                    None
                }
            })
            .min_by_key(|(_, _, _, parked_at)| *parked_at)
            .map(|(id, topic, state, _)| ManagedSessionInfo {
                session_id: id,
                topic,
                state,
                parent_session_id: None,
                child_count: 0,
                depth: 0,
            })
    }

    /// Get the depth of a session in the fork tree.
    async fn get_depth(&self, session_id: &str) -> Result<u32> {
        let mut depth = 0u32;
        let mut current = session_id.to_string();

        while let Ok(data) = self.load_data(&current).await {
            match data.parent_session_id {
                Some(parent) => {
                    depth += 1;
                    current = parent;
                }
                None => break,
            }
        }

        Ok(depth)
    }

    /// Persist orchestrator data into the session's extension_data.
    async fn persist_data(&self, session_id: &str, data: &SessionOrchestratorData) -> Result<()> {
        let session = self.session_manager.get_session(session_id, false).await?;
        let mut ext_data = session.extension_data;
        ext_data.set_extension_state(
            ORCHESTRATOR_NAME,
            ORCHESTRATOR_VERSION,
            serde_json::to_value(data)?,
        );

        self.session_manager
            .update(session_id)
            .extension_data(ext_data)
            .apply()
            .await?;

        Ok(())
    }

    /// Load orchestrator data from a session's extension_data.
    async fn load_data(&self, session_id: &str) -> Result<SessionOrchestratorData> {
        let session = self.session_manager.get_session(session_id, false).await?;
        let value = session
            .extension_data
            .get_extension_state(ORCHESTRATOR_NAME, ORCHESTRATOR_VERSION)
            .ok_or_else(|| {
                anyhow::anyhow!("Session {} is not managed by orchestrator", session_id)
            })?;
        let data: SessionOrchestratorData = serde_json::from_value(value.clone())?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_orchestrator() -> (SessionOrchestrator, Arc<SessionManager>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let sm = Arc::new(SessionManager::new(dir.path().to_path_buf()));
        let orch = SessionOrchestrator::new(sm.clone());
        (orch, sm, dir)
    }

    #[tokio::test]
    async fn test_register_session() {
        let (orch, sm, _dir) = test_orchestrator().await;
        let session = sm
            .create_session(
                std::path::PathBuf::from("."),
                "test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        orch.register(&session.id, "Fix auth module".to_string(), None)
            .await
            .unwrap();

        let sessions = orch.list(None).await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].topic, "Fix auth module");
        assert!(matches!(sessions[0].state, SessionState::Active));
    }

    #[tokio::test]
    async fn test_park_and_resume() {
        let (orch, sm, _dir) = test_orchestrator().await;
        let session = sm
            .create_session(
                std::path::PathBuf::from("."),
                "test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        orch.register(&session.id, "Database refactor".to_string(), None)
            .await
            .unwrap();

        // Park
        orch.park(&session.id, "Waiting for migration review".to_string())
            .await
            .unwrap();

        let sessions = orch.list(Some("parked")).await;
        assert_eq!(sessions.len(), 1);

        let active = orch.list(Some("active")).await;
        assert_eq!(active.len(), 0);

        // Resume
        orch.resume(&session.id).await.unwrap();

        let active = orch.list(Some("active")).await;
        assert_eq!(active.len(), 1);

        let parked = orch.list(Some("parked")).await;
        assert_eq!(parked.len(), 0);
    }

    #[tokio::test]
    async fn test_complete_session() {
        let (orch, sm, _dir) = test_orchestrator().await;
        let session = sm
            .create_session(
                std::path::PathBuf::from("."),
                "test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        orch.register(&session.id, "API endpoint".to_string(), None)
            .await
            .unwrap();

        orch.complete(&session.id, "Endpoint deployed successfully".to_string())
            .await
            .unwrap();

        let completed = orch.list(Some("completed")).await;
        assert_eq!(completed.len(), 1);

        let active = orch.list(Some("active")).await;
        assert_eq!(active.len(), 0);
    }

    #[tokio::test]
    async fn test_fork_session() {
        let (orch, sm, _dir) = test_orchestrator().await;
        let parent = sm
            .create_session(
                std::path::PathBuf::from("."),
                "test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        orch.register(&parent.id, "Main project".to_string(), None)
            .await
            .unwrap();

        let child_id = orch
            .fork(&parent.id, "Sub-task: fix tests".to_string(), None)
            .await
            .unwrap();

        // Both should be active
        let all = orch.list(Some("active")).await;
        assert_eq!(all.len(), 2);

        // Child should have parent
        let child_data = orch.load_data(&child_id).await.unwrap();
        assert_eq!(child_data.parent_session_id, Some(parent.id.clone()));
        assert_eq!(child_data.topic, "Sub-task: fix tests");

        // Parent should track child
        let parent_data = orch.load_data(&parent.id).await.unwrap();
        assert!(parent_data.child_session_ids.contains(&child_id));
    }

    #[tokio::test]
    async fn test_fork_budget_limit() {
        let (orch, sm, _dir) = test_orchestrator().await;
        let parent = sm
            .create_session(
                std::path::PathBuf::from("."),
                "test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        let budget = SessionBudget {
            max_children: 2,
            max_total_tokens: None,
            max_depth: 3,
        };

        orch.register(&parent.id, "Main".to_string(), Some(budget))
            .await
            .unwrap();

        // Fork two children — should succeed
        orch.fork(&parent.id, "Child 1".to_string(), None)
            .await
            .unwrap();
        orch.fork(&parent.id, "Child 2".to_string(), None)
            .await
            .unwrap();

        // Third fork should fail
        let result = orch.fork(&parent.id, "Child 3".to_string(), None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already has 2 children"));
    }

    #[tokio::test]
    async fn test_resume_non_parked_fails() {
        let (orch, sm, _dir) = test_orchestrator().await;
        let session = sm
            .create_session(
                std::path::PathBuf::from("."),
                "test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        orch.register(&session.id, "Active task".to_string(), None)
            .await
            .unwrap();

        let result = orch.resume(&session.id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not parked"));
    }

    #[tokio::test]
    async fn test_next_resumable_fifo() {
        let (orch, sm, _dir) = test_orchestrator().await;
        let s1 = sm
            .create_session(
                std::path::PathBuf::from("."),
                "test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();
        let s2 = sm
            .create_session(
                std::path::PathBuf::from("."),
                "test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        orch.register(&s1.id, "First task".to_string(), None)
            .await
            .unwrap();
        orch.register(&s2.id, "Second task".to_string(), None)
            .await
            .unwrap();

        // Park first, then second
        orch.park(&s1.id, "Blocked on review".to_string())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        orch.park(&s2.id, "Blocked on CI".to_string())
            .await
            .unwrap();

        // First parked should be returned (oldest)
        let next = orch.next_resumable().await.unwrap();
        assert_eq!(next.topic, "First task");
    }

    #[tokio::test]
    async fn test_state_transition_history() {
        let (orch, sm, _dir) = test_orchestrator().await;
        let session = sm
            .create_session(
                std::path::PathBuf::from("."),
                "test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        orch.register(&session.id, "Tracked task".to_string(), None)
            .await
            .unwrap();

        orch.park(&session.id, "Lunch break".to_string())
            .await
            .unwrap();
        orch.resume(&session.id).await.unwrap();
        orch.complete(&session.id, "Done!".to_string())
            .await
            .unwrap();

        let data = orch.load_data(&session.id).await.unwrap();
        assert_eq!(data.history.len(), 3);
        assert!(matches!(data.history[0].from, SessionState::Active));
        assert!(matches!(data.history[0].to, SessionState::Parked { .. }));
        assert!(matches!(data.history[1].from, SessionState::Parked { .. }));
        assert!(matches!(data.history[1].to, SessionState::Active));
        assert!(matches!(data.history[2].from, SessionState::Active));
        assert!(matches!(data.history[2].to, SessionState::Completed { .. }));
    }

    #[tokio::test]
    async fn test_session_state_serde_roundtrip() {
        let states = vec![
            SessionState::Active,
            SessionState::Parked {
                reason: "blocked".to_string(),
                parked_at: Utc::now(),
            },
            SessionState::Completed {
                summary: "done".to_string(),
                completed_at: Utc::now(),
            },
        ];

        for state in states {
            let json = serde_json::to_string(&state).unwrap();
            let roundtripped: SessionState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, roundtripped);
        }
    }
}
