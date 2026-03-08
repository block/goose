//! Event bus for agent execution events.
//!
//! Uses tokio broadcast channels so multiple consumers (streaming, push notifications)
//! can receive events from a single agent execution.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::types::events::AgentExecutionEvent;

const DEFAULT_CHANNEL_SIZE: usize = 256;

/// Event bus for a single task execution.
///
/// Wraps a tokio broadcast channel. Producers `send` events; consumers
/// `subscribe` to receive them. Multiple subscribers are supported.
pub struct ExecutionEventBus {
    sender: broadcast::Sender<AgentExecutionEvent>,
}

impl ExecutionEventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(DEFAULT_CHANNEL_SIZE);
        Self { sender }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Send an event to all subscribers.
    pub fn send(
        &self,
        event: AgentExecutionEvent,
    ) -> Result<usize, Box<broadcast::error::SendError<AgentExecutionEvent>>> {
        self.sender.send(event).map_err(Box::new)
    }

    /// Subscribe to events. Returns a receiver that will get all future events.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentExecutionEvent> {
        self.sender.subscribe()
    }

    /// Number of active receivers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for ExecutionEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages event buses keyed by task ID.
pub struct EventBusManager {
    buses: Arc<RwLock<HashMap<String, Arc<ExecutionEventBus>>>>,
}

impl EventBusManager {
    pub fn new() -> Self {
        Self {
            buses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create an event bus for the given task ID.
    pub async fn get_or_create(&self, task_id: &str) -> Arc<ExecutionEventBus> {
        {
            let buses = self.buses.read().await;
            if let Some(bus) = buses.get(task_id) {
                return Arc::clone(bus);
            }
        }

        let mut buses = self.buses.write().await;
        let bus = buses
            .entry(task_id.to_string())
            .or_insert_with(|| Arc::new(ExecutionEventBus::new()));
        Arc::clone(bus)
    }

    /// Get an existing event bus for a task, if it exists.
    pub async fn get(&self, task_id: &str) -> Option<Arc<ExecutionEventBus>> {
        let buses = self.buses.read().await;
        buses.get(task_id).cloned()
    }

    /// Remove and return the event bus for a task.
    pub async fn remove(&self, task_id: &str) -> Option<Arc<ExecutionEventBus>> {
        let mut buses = self.buses.write().await;
        buses.remove(task_id)
    }

    /// Number of active event buses.
    pub async fn len(&self) -> usize {
        let buses = self.buses.read().await;
        buses.len()
    }

    /// Check if there are no active event buses.
    pub async fn is_empty(&self) -> bool {
        let buses = self.buses.read().await;
        buses.is_empty()
    }
}

impl Default for EventBusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::core::{TaskState, TaskStatus};
    use crate::types::events::{AgentExecutionEvent, TaskStatusUpdateEvent};

    #[tokio::test]
    async fn test_event_bus_send_receive() {
        let bus = ExecutionEventBus::new();
        let mut rx = bus.subscribe();

        let event = AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Working,
                message: None,
                timestamp: None,
            },
            metadata: None,
        });

        bus.send(event.clone()).unwrap();
        let received = rx.recv().await.unwrap();

        match received {
            AgentExecutionEvent::StatusUpdate(update) => {
                assert_eq!(update.task_id, "t1");
                assert_eq!(update.status.state, TaskState::Working);
            }
            _ => panic!("Expected StatusUpdate"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = ExecutionEventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let event = AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Completed,
                message: None,
                timestamp: None,
            },
            metadata: None,
        });

        let count = bus.send(event).unwrap();
        assert_eq!(count, 2);

        let r1 = rx1.recv().await.unwrap();
        let r2 = rx2.recv().await.unwrap();
        match (r1, r2) {
            (AgentExecutionEvent::StatusUpdate(u1), AgentExecutionEvent::StatusUpdate(u2)) => {
                assert_eq!(u1.task_id, "t1");
                assert_eq!(u2.task_id, "t1");
            }
            _ => panic!("Expected two StatusUpdates"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_manager_get_or_create() {
        let mgr = EventBusManager::new();
        assert!(mgr.is_empty().await);

        let bus1 = mgr.get_or_create("task-1").await;
        assert_eq!(mgr.len().await, 1);

        let bus2 = mgr.get_or_create("task-1").await;
        assert_eq!(mgr.len().await, 1);

        // Should be the same bus
        assert!(Arc::ptr_eq(&bus1, &bus2));
    }

    #[tokio::test]
    async fn test_event_bus_manager_get_missing() {
        let mgr = EventBusManager::new();
        assert!(mgr.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_event_bus_manager_remove() {
        let mgr = EventBusManager::new();
        mgr.get_or_create("task-1").await;
        assert_eq!(mgr.len().await, 1);

        let removed = mgr.remove("task-1").await;
        assert!(removed.is_some());
        assert!(mgr.is_empty().await);
    }

    #[tokio::test]
    async fn test_event_bus_receiver_count() {
        let bus = ExecutionEventBus::new();
        assert_eq!(bus.receiver_count(), 0);

        let _rx1 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 1);

        let _rx2 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);
    }
}
