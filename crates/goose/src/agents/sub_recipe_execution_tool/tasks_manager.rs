use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agents::sub_recipe_execution_tool::task_types::Task;

#[derive(Debug, Clone)]
pub struct TasksManager {
    tasks: Arc<RwLock<HashMap<String, Task>>>,
}

impl Default for TasksManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TasksManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn save_tasks(&self, tasks: Vec<Task>) {
        let mut task_map = self.tasks.write().await;
        for task in tasks {
            task_map.insert(task.id.clone(), task);
        }
    }

    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_task(id: &str, sub_recipe_name: &str) -> Task {
        Task {
            id: id.to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": sub_recipe_name,
                    "command_parameters": {},
                    "recipe_path": "/test/path"
                }
            }),
        }
    }

    #[tokio::test]
    async fn test_save_and_get_task() {
        let manager = TasksManager::new();
        let tasks = vec![create_test_task("task1", "weather")];

        manager.save_tasks(tasks).await;

        let retrieved = manager.get_task("task1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "task1");
    }

    #[tokio::test]
    async fn test_save_multiple_tasks() {
        let manager = TasksManager::new();
        let tasks = vec![
            create_test_task("task1", "weather"),
            create_test_task("task2", "news"),
        ];

        manager.save_tasks(tasks).await;

        let task1 = manager.get_task("task1").await;
        let task2 = manager.get_task("task2").await;
        assert!(task1.is_some());
        assert!(task2.is_some());
        assert_eq!(task1.unwrap().id, "task1");
        assert_eq!(task2.unwrap().id, "task2");
    }
}
