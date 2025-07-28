#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::agents::sub_recipe_execution_tool::types::Task;
    use serde_json::json;

    #[test]
    fn test_task_timeout_extraction() {
        // Test that the task timeout is properly extracted and used
        let task_with_timeout = Task {
            id: "test-id".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": "test",
                    "task_timeout": 180,
                    "command_parameters": {},
                    "recipe_path": "test.yaml"
                }
            }),
        };

        assert_eq!(task_with_timeout.get_task_timeout(), Some(180));

        let task_without_timeout = Task {
            id: "test-id-2".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": "test",
                    "command_parameters": {},
                    "recipe_path": "test.yaml"
                }
            }),
        };

        assert_eq!(task_without_timeout.get_task_timeout(), None);
    }
}