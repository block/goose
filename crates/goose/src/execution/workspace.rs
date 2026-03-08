//! Workspace coordination for multi-agent file access.
//!
//! When multiple agent instances work on the same project, they need to
//! coordinate file edits, build triggers, and status changes. This module
//! provides:
//!
//! - [`WorkspaceEvent`] — typed events for file changes, builds, and agent activity
//! - [`WorkspaceEventBus`] — broadcast channel for event distribution
//! - [`FileLockManager`] — advisory soft locking for file coordination
//! - [`SharedWorkspace`] — combines event bus + file locks for a project directory

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};

/// Events broadcast across agents working in the same workspace.
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    /// A file was modified by an agent.
    FileChanged { path: PathBuf, agent_id: String },
    /// A file was created by an agent.
    FileCreated { path: PathBuf, agent_id: String },
    /// A file was deleted by an agent.
    FileDeleted { path: PathBuf, agent_id: String },
    /// A build/compilation was started.
    BuildStarted { agent_id: String, command: String },
    /// A build/compilation completed successfully.
    BuildSucceeded { agent_id: String, duration_ms: u64 },
    /// A build/compilation failed.
    BuildFailed { agent_id: String, error: String },
    /// An agent acquired a soft lock on a file.
    FileLocked { path: PathBuf, agent_id: String },
    /// An agent released a soft lock on a file.
    FileUnlocked { path: PathBuf, agent_id: String },
    /// An agent joined the workspace.
    AgentJoined { agent_id: String },
    /// An agent left the workspace.
    AgentLeft { agent_id: String },
}

impl std::fmt::Display for WorkspaceEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileChanged { path, agent_id } => {
                write!(f, "[{agent_id}] changed {}", path.display())
            }
            Self::FileCreated { path, agent_id } => {
                write!(f, "[{agent_id}] created {}", path.display())
            }
            Self::FileDeleted { path, agent_id } => {
                write!(f, "[{agent_id}] deleted {}", path.display())
            }
            Self::BuildStarted { agent_id, command } => {
                write!(f, "[{agent_id}] build started: {command}")
            }
            Self::BuildSucceeded {
                agent_id,
                duration_ms,
            } => write!(f, "[{agent_id}] build succeeded ({duration_ms}ms)"),
            Self::BuildFailed { agent_id, error } => {
                write!(f, "[{agent_id}] build failed: {error}")
            }
            Self::FileLocked { path, agent_id } => {
                write!(f, "[{agent_id}] locked {}", path.display())
            }
            Self::FileUnlocked { path, agent_id } => {
                write!(f, "[{agent_id}] unlocked {}", path.display())
            }
            Self::AgentJoined { agent_id } => write!(f, "[{agent_id}] joined workspace"),
            Self::AgentLeft { agent_id } => write!(f, "[{agent_id}] left workspace"),
        }
    }
}

/// Broadcast channel for workspace events.
///
/// Subscribers receive all events published after they subscribe.
/// Uses tokio broadcast with a bounded buffer — slow receivers
/// may miss events (lagged).
pub struct WorkspaceEventBus {
    sender: broadcast::Sender<WorkspaceEvent>,
}

impl WorkspaceEventBus {
    /// Create a new event bus with the given channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an event to all subscribers.
    pub fn publish(&self, event: WorkspaceEvent) -> usize {
        // send() returns the number of receivers that got the message
        // If no receivers, that's fine — events are fire-and-forget
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribe to workspace events.
    pub fn subscribe(&self) -> broadcast::Receiver<WorkspaceEvent> {
        self.sender.subscribe()
    }

    /// Current number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Information about who holds a soft lock on a file.
#[derive(Debug, Clone)]
pub struct LockInfo {
    pub agent_id: String,
    pub acquired_at: Instant,
}

/// Advisory file locking for multi-agent coordination.
///
/// These are **soft locks** — they don't prevent file access at the OS level.
/// Agents check locks before editing and respect them cooperatively.
/// Locks are automatically released when an agent leaves the workspace.
pub struct FileLockManager {
    locks: RwLock<HashMap<PathBuf, LockInfo>>,
}

impl FileLockManager {
    pub fn new() -> Self {
        Self {
            locks: RwLock::new(HashMap::new()),
        }
    }

    /// Try to acquire a soft lock on a file.
    /// Returns `Ok(())` if acquired, `Err(LockInfo)` if already held by another agent.
    pub async fn try_lock(&self, path: PathBuf, agent_id: String) -> Result<(), LockInfo> {
        let mut locks = self.locks.write().await;
        if let Some(existing) = locks.get(&path) {
            if existing.agent_id != agent_id {
                return Err(existing.clone());
            }
            // Same agent re-locking is idempotent
        }
        locks.insert(
            path,
            LockInfo {
                agent_id,
                acquired_at: Instant::now(),
            },
        );
        Ok(())
    }

    /// Release a soft lock on a file.
    /// Returns true if the lock was held by this agent and released.
    pub async fn unlock(&self, path: &PathBuf, agent_id: &str) -> bool {
        let mut locks = self.locks.write().await;
        if let Some(info) = locks.get(path) {
            if info.agent_id == agent_id {
                locks.remove(path);
                return true;
            }
        }
        false
    }

    /// Check if a file is locked and by whom.
    pub async fn check_lock(&self, path: &PathBuf) -> Option<LockInfo> {
        self.locks.read().await.get(path).cloned()
    }

    /// Release all locks held by a specific agent.
    pub async fn release_all(&self, agent_id: &str) -> Vec<PathBuf> {
        let mut locks = self.locks.write().await;
        let released: Vec<PathBuf> = locks
            .iter()
            .filter(|(_, info)| info.agent_id == agent_id)
            .map(|(path, _)| path.clone())
            .collect();
        for path in &released {
            locks.remove(path);
        }
        released
    }

    /// List all currently locked files.
    pub async fn list_locks(&self) -> Vec<(PathBuf, LockInfo)> {
        self.locks
            .read()
            .await
            .iter()
            .map(|(p, i)| (p.clone(), i.clone()))
            .collect()
    }

    /// Number of active locks.
    pub async fn lock_count(&self) -> usize {
        self.locks.read().await.len()
    }
}

impl Default for FileLockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A shared workspace that coordinates multiple agents working on the same project.
///
/// Combines event broadcasting with advisory file locking.
pub struct SharedWorkspace {
    pub root: PathBuf,
    pub event_bus: WorkspaceEventBus,
    pub file_locks: FileLockManager,
    agents: RwLock<Vec<String>>,
}

impl SharedWorkspace {
    /// Create a new shared workspace rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            event_bus: WorkspaceEventBus::new(256),
            file_locks: FileLockManager::new(),
            agents: RwLock::new(Vec::new()),
        }
    }

    /// Register an agent as working in this workspace.
    pub async fn join(&self, agent_id: String) {
        let mut agents = self.agents.write().await;
        if !agents.contains(&agent_id) {
            agents.push(agent_id.clone());
            self.event_bus
                .publish(WorkspaceEvent::AgentJoined { agent_id });
        }
    }

    /// Remove an agent from the workspace, releasing all its locks.
    pub async fn leave(&self, agent_id: &str) {
        let mut agents = self.agents.write().await;
        agents.retain(|a| a != agent_id);
        drop(agents);

        let released = self.file_locks.release_all(agent_id).await;
        for path in released {
            self.event_bus.publish(WorkspaceEvent::FileUnlocked {
                path,
                agent_id: agent_id.to_string(),
            });
        }
        self.event_bus.publish(WorkspaceEvent::AgentLeft {
            agent_id: agent_id.to_string(),
        });
    }

    /// Notify the workspace that a file was changed, with optional soft lock check.
    pub async fn notify_file_changed(&self, path: PathBuf, agent_id: &str) {
        self.event_bus.publish(WorkspaceEvent::FileChanged {
            path,
            agent_id: agent_id.to_string(),
        });
    }

    /// Try to lock a file, broadcasting the lock event on success.
    pub async fn lock_file(&self, path: PathBuf, agent_id: String) -> Result<(), LockInfo> {
        self.file_locks
            .try_lock(path.clone(), agent_id.clone())
            .await?;
        self.event_bus
            .publish(WorkspaceEvent::FileLocked { path, agent_id });
        Ok(())
    }

    /// Unlock a file, broadcasting the unlock event on success.
    pub async fn unlock_file(&self, path: &PathBuf, agent_id: &str) -> bool {
        let released = self.file_locks.unlock(path, agent_id).await;
        if released {
            self.event_bus.publish(WorkspaceEvent::FileUnlocked {
                path: path.clone(),
                agent_id: agent_id.to_string(),
            });
        }
        released
    }

    /// List agents currently in the workspace.
    pub async fn active_agents(&self) -> Vec<String> {
        self.agents.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_event_display() {
        let event = WorkspaceEvent::FileChanged {
            path: PathBuf::from("src/main.rs"),
            agent_id: "dev-1".to_string(),
        };
        assert_eq!(format!("{event}"), "[dev-1] changed src/main.rs");

        let event = WorkspaceEvent::BuildFailed {
            agent_id: "dev-2".to_string(),
            error: "compilation error".to_string(),
        };
        assert_eq!(
            format!("{event}"),
            "[dev-2] build failed: compilation error"
        );
    }

    #[test]
    fn test_event_bus_no_subscribers() {
        let bus = WorkspaceEventBus::new(16);
        let count = bus.publish(WorkspaceEvent::AgentJoined {
            agent_id: "test".to_string(),
        });
        assert_eq!(count, 0);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = WorkspaceEventBus::new(16);
        let mut rx = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        bus.publish(WorkspaceEvent::FileChanged {
            path: PathBuf::from("test.rs"),
            agent_id: "agent-1".to_string(),
        });

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, WorkspaceEvent::FileChanged { .. }));
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = WorkspaceEventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);

        bus.publish(WorkspaceEvent::BuildStarted {
            agent_id: "builder".to_string(),
            command: "cargo build".to_string(),
        });

        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }

    #[tokio::test]
    async fn test_file_lock_acquire_release() {
        let mgr = FileLockManager::new();

        mgr.try_lock(PathBuf::from("file.rs"), "agent-1".to_string())
            .await
            .unwrap();
        assert_eq!(mgr.lock_count().await, 1);

        let info = mgr.check_lock(&PathBuf::from("file.rs")).await.unwrap();
        assert_eq!(info.agent_id, "agent-1");

        assert!(mgr.unlock(&PathBuf::from("file.rs"), "agent-1").await);
        assert_eq!(mgr.lock_count().await, 0);
    }

    #[tokio::test]
    async fn test_file_lock_conflict() {
        let mgr = FileLockManager::new();

        mgr.try_lock(PathBuf::from("file.rs"), "agent-1".to_string())
            .await
            .unwrap();

        let result = mgr
            .try_lock(PathBuf::from("file.rs"), "agent-2".to_string())
            .await;
        assert!(result.is_err());
        let holder = result.unwrap_err();
        assert_eq!(holder.agent_id, "agent-1");
    }

    #[tokio::test]
    async fn test_file_lock_same_agent_idempotent() {
        let mgr = FileLockManager::new();

        mgr.try_lock(PathBuf::from("file.rs"), "agent-1".to_string())
            .await
            .unwrap();
        mgr.try_lock(PathBuf::from("file.rs"), "agent-1".to_string())
            .await
            .unwrap();
        assert_eq!(mgr.lock_count().await, 1);
    }

    #[tokio::test]
    async fn test_file_lock_wrong_agent_cant_unlock() {
        let mgr = FileLockManager::new();

        mgr.try_lock(PathBuf::from("file.rs"), "agent-1".to_string())
            .await
            .unwrap();
        assert!(!mgr.unlock(&PathBuf::from("file.rs"), "agent-2").await);
        assert_eq!(mgr.lock_count().await, 1);
    }

    #[tokio::test]
    async fn test_release_all_by_agent() {
        let mgr = FileLockManager::new();

        mgr.try_lock(PathBuf::from("a.rs"), "agent-1".to_string())
            .await
            .unwrap();
        mgr.try_lock(PathBuf::from("b.rs"), "agent-1".to_string())
            .await
            .unwrap();
        mgr.try_lock(PathBuf::from("c.rs"), "agent-2".to_string())
            .await
            .unwrap();

        let released = mgr.release_all("agent-1").await;
        assert_eq!(released.len(), 2);
        assert_eq!(mgr.lock_count().await, 1);
    }

    #[tokio::test]
    async fn test_shared_workspace_join_leave() {
        let ws = SharedWorkspace::new(PathBuf::from("/tmp/test-project"));
        let mut rx = ws.event_bus.subscribe();

        ws.join("agent-1".to_string()).await;
        ws.join("agent-2".to_string()).await;
        assert_eq!(ws.active_agents().await.len(), 2);

        // Drain join events
        let _ = rx.recv().await;
        let _ = rx.recv().await;

        // Lock a file as agent-1
        ws.lock_file(PathBuf::from("main.rs"), "agent-1".to_string())
            .await
            .unwrap();

        // Agent-1 leaves — lock should be auto-released
        ws.leave("agent-1").await;
        assert_eq!(ws.active_agents().await.len(), 1);
        assert_eq!(ws.file_locks.lock_count().await, 0);
    }

    #[tokio::test]
    async fn test_shared_workspace_lock_conflict() {
        let ws = SharedWorkspace::new(PathBuf::from("/tmp/test-project"));

        ws.lock_file(PathBuf::from("file.rs"), "agent-1".to_string())
            .await
            .unwrap();

        let result = ws
            .lock_file(PathBuf::from("file.rs"), "agent-2".to_string())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shared_workspace_file_notification() {
        let ws = SharedWorkspace::new(PathBuf::from("/tmp/test-project"));
        let mut rx = ws.event_bus.subscribe();

        ws.notify_file_changed(PathBuf::from("lib.rs"), "agent-1")
            .await;

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, WorkspaceEvent::FileChanged { .. }));
    }

    #[tokio::test]
    async fn test_shared_workspace_duplicate_join() {
        let ws = SharedWorkspace::new(PathBuf::from("/tmp/test-project"));

        ws.join("agent-1".to_string()).await;
        ws.join("agent-1".to_string()).await;
        assert_eq!(ws.active_agents().await.len(), 1);
    }

    #[tokio::test]
    async fn test_list_locks() {
        let mgr = FileLockManager::new();

        mgr.try_lock(PathBuf::from("a.rs"), "agent-1".to_string())
            .await
            .unwrap();
        mgr.try_lock(PathBuf::from("b.rs"), "agent-2".to_string())
            .await
            .unwrap();

        let locks = mgr.list_locks().await;
        assert_eq!(locks.len(), 2);
    }

    #[tokio::test]
    async fn test_event_bus_dropped_subscriber() {
        let bus = WorkspaceEventBus::new(16);
        let rx = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        drop(rx);
        assert_eq!(bus.subscriber_count(), 0);

        // Publishing with no subscribers is fine
        let count = bus.publish(WorkspaceEvent::AgentJoined {
            agent_id: "test".to_string(),
        });
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_lock_info_timing() {
        let mgr = FileLockManager::new();

        mgr.try_lock(PathBuf::from("file.rs"), "agent-1".to_string())
            .await
            .unwrap();

        let info = mgr.check_lock(&PathBuf::from("file.rs")).await.unwrap();
        assert!(info.acquired_at.elapsed() < Duration::from_secs(1));
    }
}
