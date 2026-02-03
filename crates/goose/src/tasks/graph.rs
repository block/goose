//! Task Graph with DAG dependencies and parallel execution

use super::events::{TaskEvent, TaskEventSender};
use super::{Task, TaskId, TaskOwner, TaskPriority, TaskStatus, TaskUpdate};
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::{Mutex, RwLock};

/// Configuration for the task graph
#[derive(Debug, Clone)]
pub struct TaskGraphConfig {
    pub concurrency_limit: usize,
    pub auto_unblock: bool,
    pub persist_on_change: bool,
}

impl Default for TaskGraphConfig {
    fn default() -> Self {
        Self {
            concurrency_limit: 4,
            auto_unblock: true,
            persist_on_change: true,
        }
    }
}

/// Task Graph with DAG-based dependencies
pub struct TaskGraph {
    tasks: RwLock<HashMap<TaskId, Task>>,
    config: TaskGraphConfig,
    event_sender: Option<TaskEventSender>,
    running_count: Mutex<usize>,
}

impl TaskGraph {
    pub fn new(config: TaskGraphConfig) -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            config,
            event_sender: None,
            running_count: Mutex::new(0),
        }
    }

    pub fn with_event_sender(mut self, sender: TaskEventSender) -> Self {
        self.event_sender = Some(sender);
        self
    }

    /// Create a new task in the graph
    pub async fn create(&self, task: Task) -> Result<TaskId> {
        let task_id = task.id.clone();

        // Check for dependency cycles
        self.check_for_cycles(&task).await?;

        // Set initial blockers from dependencies
        let mut task = task;
        let tasks = self.tasks.read().await;
        for dep_id in &task.dependencies {
            if let Some(dep) = tasks.get(dep_id) {
                if !dep.is_terminal() {
                    task.blockers.push(dep_id.clone());
                    task.status = TaskStatus::Blocked;
                }
            }
        }
        drop(tasks);

        // Insert task
        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id.clone(), task.clone());

        // Emit event
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(TaskEvent::Created(task_id.clone()));
        }

        Ok(task_id)
    }

    /// List all tasks
    pub async fn list(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// List tasks filtered by status
    pub async fn list_by_status(&self, status: TaskStatus) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .filter(|t| t.status == status)
            .cloned()
            .collect()
    }

    /// List tasks filtered by owner
    pub async fn list_by_owner(&self, owner: &TaskOwner) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .filter(|t| t.owner.as_ref() == Some(owner))
            .cloned()
            .collect()
    }

    /// Get a specific task
    pub async fn get(&self, id: &TaskId) -> Option<Task> {
        let tasks = self.tasks.read().await;
        tasks.get(id).cloned()
    }

    /// Update a task
    pub async fn update(&self, id: &TaskId, update: TaskUpdate) -> Result<Task> {
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(id)
            .ok_or_else(|| anyhow!("Task not found: {}", id))?;

        let old_status = task.status;

        // Apply status change
        if let Some(new_status) = update.status {
            task.status = new_status;
            task.updated_at = Utc::now();

            match new_status {
                TaskStatus::Running => task.started_at = Some(Utc::now()),
                TaskStatus::Done | TaskStatus::Failed | TaskStatus::Cancelled => {
                    task.completed_at = Some(Utc::now());
                }
                _ => {}
            }
        }

        // Apply result
        if let Some(result) = update.result {
            task.result = Some(result);
        }

        // Add blockers
        for blocker in update.add_blockers {
            if !task.blockers.contains(&blocker) {
                task.blockers.push(blocker);
                if task.status == TaskStatus::Queued {
                    task.status = TaskStatus::Blocked;
                }
            }
        }

        // Remove blockers
        for blocker in &update.remove_blockers {
            task.blockers.retain(|b| b != blocker);
        }

        // Check if task should be unblocked
        if task.blockers.is_empty() && task.status == TaskStatus::Blocked {
            task.status = TaskStatus::Queued;
        }

        // Apply metadata
        for (key, value) in update.metadata {
            task.metadata.insert(key, value);
        }

        let updated_task = task.clone();
        let task_id = id.clone();

        // Emit status change event
        if let Some(ref sender) = self.event_sender {
            if old_status != updated_task.status {
                let _ = sender.send(TaskEvent::StatusChanged {
                    id: task_id.clone(),
                    old: old_status,
                    new: updated_task.status,
                });
            }
        }

        // Handle completion - unblock dependent tasks
        if updated_task.is_terminal() && self.config.auto_unblock {
            let dependent_tasks: Vec<TaskId> = tasks
                .iter()
                .filter(|(_, t)| t.blockers.contains(&task_id))
                .map(|(id, _)| id.clone())
                .collect();

            for dep_id in dependent_tasks {
                if let Some(dep_task) = tasks.get_mut(&dep_id) {
                    dep_task.blockers.retain(|b| b != &task_id);
                    if dep_task.blockers.is_empty() && dep_task.status == TaskStatus::Blocked {
                        dep_task.status = TaskStatus::Queued;
                        if let Some(ref sender) = self.event_sender {
                            let _ = sender.send(TaskEvent::DependencyUnblocked {
                                id: dep_id,
                                unblocked_by: task_id.clone(),
                            });
                        }
                    }
                }
            }
        }

        Ok(updated_task)
    }

    /// Get ready tasks (queued with no blockers)
    pub async fn get_ready_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        let mut ready: Vec<Task> = tasks.values().filter(|t| t.is_ready()).cloned().collect();

        // Sort by priority (critical first)
        ready.sort_by(|a, b| {
            let priority_order = |p: &TaskPriority| match p {
                TaskPriority::Critical => 0,
                TaskPriority::High => 1,
                TaskPriority::Normal => 2,
                TaskPriority::Low => 3,
            };
            priority_order(&a.priority).cmp(&priority_order(&b.priority))
        });

        ready
    }

    /// Check if all tasks are complete
    pub async fn is_complete(&self) -> bool {
        let tasks = self.tasks.read().await;
        tasks.values().all(|t| t.is_terminal())
    }

    /// Get task count by status
    pub async fn get_status_counts(&self) -> HashMap<TaskStatus, usize> {
        let tasks = self.tasks.read().await;
        let mut counts = HashMap::new();
        for task in tasks.values() {
            *counts.entry(task.status).or_insert(0) += 1;
        }
        counts
    }

    /// Check for dependency cycles (would cause deadlock)
    async fn check_for_cycles(&self, new_task: &Task) -> Result<()> {
        let tasks = self.tasks.read().await;
        let mut visited = HashSet::new();
        let mut stack = VecDeque::new();

        for dep_id in &new_task.dependencies {
            stack.push_back(dep_id.clone());
        }

        while let Some(current) = stack.pop_front() {
            if current == new_task.id {
                return Err(anyhow!(
                    "Dependency cycle detected: task {} depends on itself",
                    new_task.id
                ));
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(task) = tasks.get(&current) {
                for dep in &task.dependencies {
                    stack.push_back(dep.clone());
                }
            }
        }

        Ok(())
    }

    /// Acquire a slot for running a task (respects concurrency limit)
    pub async fn acquire_run_slot(&self) -> bool {
        let mut count = self.running_count.lock().await;
        if *count < self.config.concurrency_limit {
            *count += 1;
            true
        } else {
            false
        }
    }

    /// Release a run slot
    pub async fn release_run_slot(&self) {
        let mut count = self.running_count.lock().await;
        if *count > 0 {
            *count -= 1;
        }
    }

    /// Get running task count
    pub async fn running_count(&self) -> usize {
        *self.running_count.lock().await
    }
}

/// Builder for creating task graphs
pub struct TaskGraphBuilder {
    config: TaskGraphConfig,
    tasks: Vec<Task>,
}

impl TaskGraphBuilder {
    pub fn new() -> Self {
        Self {
            config: TaskGraphConfig::default(),
            tasks: Vec::new(),
        }
    }

    pub fn concurrency(mut self, limit: usize) -> Self {
        self.config.concurrency_limit = limit;
        self
    }

    pub fn task(mut self, task: Task) -> Self {
        self.tasks.push(task);
        self
    }

    pub async fn build(self) -> Result<TaskGraph> {
        let graph = TaskGraph::new(self.config);
        for task in self.tasks {
            graph.create(task).await?;
        }
        Ok(graph)
    }
}

impl Default for TaskGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::TaskResult;

    #[tokio::test]
    async fn test_task_graph_create() {
        let graph = TaskGraph::new(TaskGraphConfig::default());
        let task = Task::new("task-1", "Test task");

        let id = graph.create(task).await.unwrap();
        assert_eq!(id, "task-1");

        let retrieved = graph.get(&id).await.unwrap();
        assert_eq!(retrieved.subject, "Test task");
    }

    #[tokio::test]
    async fn test_task_graph_dependencies() {
        let graph = TaskGraph::new(TaskGraphConfig::default());

        // Create first task
        let task1 = Task::new("task-1", "First task");
        graph.create(task1).await.unwrap();

        // Create dependent task
        let task2 =
            Task::new("task-2", "Second task").with_dependencies(vec!["task-1".to_string()]);
        graph.create(task2).await.unwrap();

        // Task 2 should be blocked
        let t2 = graph.get(&"task-2".to_string()).await.unwrap();
        assert_eq!(t2.status, TaskStatus::Blocked);
        assert!(t2.blockers.contains(&"task-1".to_string()));

        // Complete task 1
        graph
            .update(
                &"task-1".to_string(),
                TaskUpdate::complete(TaskResult {
                    success: true,
                    output: None,
                    error: None,
                    artifacts: vec![],
                    duration_ms: 100,
                }),
            )
            .await
            .unwrap();

        // Task 2 should now be unblocked
        let t2 = graph.get(&"task-2".to_string()).await.unwrap();
        assert_eq!(t2.status, TaskStatus::Queued);
        assert!(t2.blockers.is_empty());
    }

    #[tokio::test]
    async fn test_task_graph_ready_tasks() {
        let graph = TaskGraph::new(TaskGraphConfig::default());

        graph
            .create(Task::new("task-1", "Ready task"))
            .await
            .unwrap();
        graph
            .create(Task::new("task-2", "Another ready task"))
            .await
            .unwrap();
        graph
            .create(
                Task::new("task-3", "Blocked task").with_dependencies(vec!["task-1".to_string()]),
            )
            .await
            .unwrap();

        let ready = graph.get_ready_tasks().await;
        assert_eq!(ready.len(), 2);
    }

    #[tokio::test]
    async fn test_task_graph_cycle_detection() {
        let graph = TaskGraph::new(TaskGraphConfig::default());

        // Create task that depends on itself (should fail)
        let task =
            Task::new("task-1", "Self-dependent").with_dependencies(vec!["task-1".to_string()]);

        let result = graph.create(task).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_task_graph_concurrency() {
        let graph = TaskGraph::new(TaskGraphConfig {
            concurrency_limit: 2,
            ..Default::default()
        });

        assert!(graph.acquire_run_slot().await);
        assert!(graph.acquire_run_slot().await);
        assert!(!graph.acquire_run_slot().await); // Should fail, limit reached

        graph.release_run_slot().await;
        assert!(graph.acquire_run_slot().await); // Should succeed now
    }

    #[tokio::test]
    async fn test_task_graph_priority_ordering() {
        let graph = TaskGraph::new(TaskGraphConfig::default());

        graph
            .create(Task::new("low", "Low priority").with_priority(TaskPriority::Low))
            .await
            .unwrap();
        graph
            .create(Task::new("critical", "Critical").with_priority(TaskPriority::Critical))
            .await
            .unwrap();
        graph
            .create(Task::new("normal", "Normal").with_priority(TaskPriority::Normal))
            .await
            .unwrap();

        let ready = graph.get_ready_tasks().await;
        assert_eq!(ready[0].id, "critical");
        assert_eq!(ready[2].id, "low");
    }
}
