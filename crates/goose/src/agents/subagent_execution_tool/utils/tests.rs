use crate::agents::subagent_execution_tool::task_types::{Task, TaskInfo, TaskStatus};
use crate::agents::subagent_execution_tool::utils::{
    count_by_status, get_task_name, strip_ansi_codes,
};
use serde_json::json;
use std::collections::HashMap;

fn create_task_info_with_defaults(task: Task, status: TaskStatus) -> TaskInfo {
    TaskInfo {
        task,
        status,
        start_time: None,
        end_time: None,
        result: None,
        current_output: String::new(),
    }
}

mod test_get_task_name {
    use super::*;

    #[test]
    fn test_extracts_sub_recipe_name() {
        let sub_recipe_task = Task {
            id: "task_1".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "name": "my_recipe",
                    "recipe_path": "/path/to/recipe"
                }
            }),
        };

        let task_info = create_task_info_with_defaults(sub_recipe_task, TaskStatus::Pending);

        assert_eq!(get_task_name(&task_info), "my_recipe");
    }

    #[test]
    fn falls_back_to_task_id_for_text_instruction() {
        let text_task = Task {
            id: "task_2".to_string(),
            task_type: "text_instruction".to_string(),
            payload: json!({"text_instruction": "do something"}),
        };

        let task_info = create_task_info_with_defaults(text_task, TaskStatus::Pending);

        assert_eq!(get_task_name(&task_info), "task_2");
    }

    #[test]
    fn falls_back_to_task_id_when_sub_recipe_name_missing() {
        let malformed_task = Task {
            id: "task_3".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({
                "sub_recipe": {
                    "recipe_path": "/path/to/recipe"
                    // missing "name" field
                }
            }),
        };

        let task_info = create_task_info_with_defaults(malformed_task, TaskStatus::Pending);

        assert_eq!(get_task_name(&task_info), "task_3");
    }

    #[test]
    fn falls_back_to_task_id_when_sub_recipe_missing() {
        let malformed_task = Task {
            id: "task_4".to_string(),
            task_type: "sub_recipe".to_string(),
            payload: json!({}), // missing "sub_recipe" field
        };

        let task_info = create_task_info_with_defaults(malformed_task, TaskStatus::Pending);

        assert_eq!(get_task_name(&task_info), "task_4");
    }
}

mod count_by_status {
    use super::*;

    fn create_test_task(id: &str, status: TaskStatus) -> TaskInfo {
        let task = Task {
            id: id.to_string(),
            task_type: "test".to_string(),
            payload: json!({}),
        };
        create_task_info_with_defaults(task, status)
    }

    #[test]
    fn counts_empty_map() {
        let tasks = HashMap::new();
        let (total, pending, running, completed, failed) = count_by_status(&tasks);
        assert_eq!(
            (total, pending, running, completed, failed),
            (0, 0, 0, 0, 0)
        );
    }

    #[test]
    fn counts_single_status() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "task1".to_string(),
            create_test_task("task1", TaskStatus::Pending),
        );
        tasks.insert(
            "task2".to_string(),
            create_test_task("task2", TaskStatus::Pending),
        );

        let (total, pending, running, completed, failed) = count_by_status(&tasks);
        assert_eq!(
            (total, pending, running, completed, failed),
            (2, 2, 0, 0, 0)
        );
    }

    #[test]
    fn counts_mixed_statuses() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "task1".to_string(),
            create_test_task("task1", TaskStatus::Pending),
        );
        tasks.insert(
            "task2".to_string(),
            create_test_task("task2", TaskStatus::Running),
        );
        tasks.insert(
            "task3".to_string(),
            create_test_task("task3", TaskStatus::Completed),
        );
        tasks.insert(
            "task4".to_string(),
            create_test_task("task4", TaskStatus::Failed),
        );
        tasks.insert(
            "task5".to_string(),
            create_test_task("task5", TaskStatus::Completed),
        );

        let (total, pending, running, completed, failed) = count_by_status(&tasks);
        assert_eq!(
            (total, pending, running, completed, failed),
            (5, 1, 1, 2, 1)
        );
    }
}

mod strip_ansi_codes {
    use super::*;

    #[test]
    fn test_strip_ansi_codes() {
        assert_eq!(strip_ansi_codes("hello world"), "hello world");
        assert_eq!(strip_ansi_codes("\x1b[31mred text\x1b[0m"), "red text");
        assert_eq!(
            strip_ansi_codes("\x1b[1;32mbold green\x1b[0m"),
            "bold green"
        );
        assert_eq!(
            strip_ansi_codes("normal\x1b[33myellow\x1b[0mnormal"),
            "normalyellownormal"
        );
        assert_eq!(strip_ansi_codes("\x1bhello"), "\x1bhello");
        assert_eq!(strip_ansi_codes("hello\x1b"), "hello\x1b");
        assert_eq!(strip_ansi_codes(""), "");
    }
}
