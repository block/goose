#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::agents::sub_recipe_execution_tool::types::Task;
    use serde_json::json;

    #[tokio::test]
    async fn test_execute_single_task_with_custom_timeout() {
        // Create a task with custom timeout
        let task = Task {
            id: "test-id".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": "test",
                    "task_timeout": 120,
                    "command_parameters": {},
                    "recipe_path": "test.yaml"
                }
            }),
        };

        let config = Config {
            timeout_seconds: 60, // Default timeout
            max_workers: 10,
            initial_workers: 2,
        };

        // The timeout extraction logic is tested here
        // We verify that get_task_timeout returns the expected value
        assert_eq!(task.get_task_timeout(), Some(120));
    }

    #[tokio::test]
    async fn test_execute_single_task_default_timeout() {
        // Create a task without custom timeout
        let task = Task {
            id: "test-id".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": "test",
                    "command_parameters": {},
                    "recipe_path": "test.yaml"
                }
            }),
        };

        let config = Config {
            timeout_seconds: 60, // Default timeout
            max_workers: 10,
            initial_workers: 2,
        };

        // Should return None when no timeout is specified
        assert_eq!(task.get_task_timeout(), None);
    }
}