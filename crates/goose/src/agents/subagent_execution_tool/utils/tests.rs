use crate::agents::subagent_execution_tool::task_types::{Task, TaskInfo, TaskPayload, TaskStatus};
use crate::agents::subagent_execution_tool::utils::{
    count_by_status, get_task_name, strip_ansi_codes,
};
use crate::recipe::Recipe;
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
    fn test_extracts_recipe_title() {
        let recipe = Recipe::builder()
            .version("1.0.0")
            .title("my_recipe")
            .description("Test")
            .instructions("do something")
            .build()
            .unwrap();

        let task = Task {
            id: "task_1".to_string(),
            payload: TaskPayload {
                recipe,
                return_last_only: false,
                sequential_when_repeated: false,
                parameter_values: None,
            },
        };

        let task_info = create_task_info_with_defaults(task, TaskStatus::Pending);

        assert_eq!(get_task_name(&task_info), "my_recipe");
    }
}

mod count_by_status {
    use super::*;

    fn create_test_task(id: &str, status: TaskStatus) -> TaskInfo {
        let recipe = Recipe::builder()
            .version("1.0.0")
            .title("Test Recipe")
            .description("Test")
            .instructions("Test")
            .build()
            .unwrap();

        let task = Task {
            id: id.to_string(),
            payload: TaskPayload {
                recipe,
                return_last_only: false,
                sequential_when_repeated: false,
                parameter_values: None,
            },
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
