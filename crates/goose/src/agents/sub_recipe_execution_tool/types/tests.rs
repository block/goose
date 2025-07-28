#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    #[test]
    fn test_get_task_timeout_sub_recipe() {
        let task = Task {
            id: "test-id".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": "test",
                    "task_timeout": 3600
                }
            }),
        };

        assert_eq!(task.get_task_timeout(), Some(3600));
    }

    #[test]
    fn test_get_task_timeout_text_instruction() {
        let task = Task {
            id: "test-id".to_string(),
            task_type: "text_instruction".to_string(),
            payload: json!({
                "instruction": "test",
                "sub_recipe": {
                    "task_timeout": 1800
                }
            }),
        };

        assert_eq!(task.get_task_timeout(), Some(1800));
    }

    #[test]
    fn test_get_task_timeout_no_timeout() {
        let task = Task {
            id: "test-id".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": "test"
                }
            }),
        };

        assert_eq!(task.get_task_timeout(), None);
    }

    #[test]
    fn test_get_task_timeout_invalid_type() {
        let task = Task {
            id: "test-id".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": "test",
                    "task_timeout": "not_a_number"
                }
            }),
        };

        assert_eq!(task.get_task_timeout(), None);
    }

    #[test]
    fn test_get_task_timeout_negative_number() {
        let task = Task {
            id: "test-id".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": "test",
                    "task_timeout": -100
                }
            }),
        };

        // Negative numbers can't be converted to u64
        assert_eq!(task.get_task_timeout(), None);
    }
}