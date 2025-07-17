use super::{
    extract_failed_tasks, format_error_summary, format_failed_task_error, get_task_description,
    handle_response,
};
use crate::agents::sub_recipe_execution_tool::lib::{
    ExecutionResponse, ExecutionStats, TaskResult, TaskStatus,
};
use serde_json::json;

fn create_test_task_result(task_id: &str, status: TaskStatus, error: Option<String>) -> TaskResult {
    TaskResult {
        task_id: task_id.to_string(),
        status,
        data: Some(json!({"partial_output": "test output"})),
        error,
    }
}

fn create_test_execution_response(
    results: Vec<TaskResult>,
    failed_count: usize,
) -> ExecutionResponse {
    ExecutionResponse {
        status: "completed".to_string(),
        results: results.clone(),
        stats: ExecutionStats {
            total_tasks: results.len(),
            completed: results.len() - failed_count,
            failed: failed_count,
            execution_time_ms: 1000,
        },
    }
}

#[test]
fn test_extract_failed_tasks() {
    let results = vec![
        create_test_task_result("task1", TaskStatus::Completed, None),
        create_test_task_result(
            "task2",
            TaskStatus::Failed,
            Some("Error message".to_string()),
        ),
        create_test_task_result("task3", TaskStatus::Completed, None),
        create_test_task_result(
            "task4",
            TaskStatus::Failed,
            Some("Another error".to_string()),
        ),
    ];

    let failed_tasks = extract_failed_tasks(&results);

    assert_eq!(failed_tasks.len(), 2);
    assert!(failed_tasks[0].contains("task2"));
    assert!(failed_tasks[0].contains("Error message"));
    assert!(failed_tasks[1].contains("task4"));
    assert!(failed_tasks[1].contains("Another error"));
}

#[test]
fn test_extract_failed_tasks_empty() {
    let results = vec![
        create_test_task_result("task1", TaskStatus::Completed, None),
        create_test_task_result("task2", TaskStatus::Completed, None),
    ];

    let failed_tasks = extract_failed_tasks(&results);

    assert_eq!(failed_tasks.len(), 0);
}

#[test]
fn test_format_failed_task_error_with_error_message() {
    let result = create_test_task_result(
        "task1",
        TaskStatus::Failed,
        Some("Test error message".to_string()),
    );

    let formatted = format_failed_task_error(&result);

    assert!(formatted.contains("task1"));
    assert!(formatted.contains("Test error message"));
    assert!(formatted.contains("test output"));
    assert!(formatted.contains("ID: task1"));
}

#[test]
fn test_format_failed_task_error_without_error_message() {
    let result = create_test_task_result("task2", TaskStatus::Failed, None);

    let formatted = format_failed_task_error(&result);

    assert!(formatted.contains("task2"));
    assert!(formatted.contains("Unknown error"));
    assert!(formatted.contains("test output"));
}

#[test]
fn test_format_failed_task_error_empty_partial_output() {
    let mut result =
        create_test_task_result("task3", TaskStatus::Failed, Some("Error".to_string()));
    result.data = Some(json!({"partial_output": ""}));

    let formatted = format_failed_task_error(&result);

    assert!(formatted.contains("No output captured"));
}

#[test]
fn test_format_failed_task_error_no_partial_output() {
    let mut result =
        create_test_task_result("task4", TaskStatus::Failed, Some("Error".to_string()));
    result.data = Some(json!({}));

    let formatted = format_failed_task_error(&result);

    assert!(formatted.contains("No output captured"));
}

#[test]
fn test_format_failed_task_error_no_data() {
    let mut result =
        create_test_task_result("task5", TaskStatus::Failed, Some("Error".to_string()));
    result.data = None;

    let formatted = format_failed_task_error(&result);

    assert!(formatted.contains("No output captured"));
}

#[test]
fn test_format_error_summary() {
    let failed_tasks = vec![
        "Task 'task1': Error 1\nOutput: output1".to_string(),
        "Task 'task2': Error 2\nOutput: output2".to_string(),
    ];

    let summary = format_error_summary(2, 5, failed_tasks);

    assert_eq!(summary, "2/5 tasks failed:\nTask 'task1': Error 1\nOutput: output1\nTask 'task2': Error 2\nOutput: output2");
}

#[test]
fn test_format_error_summary_single_failure() {
    let failed_tasks = vec!["Task 'task1': Error\nOutput: output".to_string()];

    let summary = format_error_summary(1, 3, failed_tasks);

    assert_eq!(
        summary,
        "1/3 tasks failed:\nTask 'task1': Error\nOutput: output"
    );
}

#[test]
fn test_handle_response_success() {
    let results = vec![
        create_test_task_result("task1", TaskStatus::Completed, None),
        create_test_task_result("task2", TaskStatus::Completed, None),
    ];
    let response = create_test_execution_response(results, 0);

    let result = handle_response(response);

    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["status"], "completed");
    assert_eq!(value["stats"]["failed"], 0);
}

#[test]
fn test_handle_response_with_failures() {
    let results = vec![
        create_test_task_result("task1", TaskStatus::Completed, None),
        create_test_task_result("task2", TaskStatus::Failed, Some("Test error".to_string())),
    ];
    let response = create_test_execution_response(results, 1);

    let result = handle_response(response);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("1/2 tasks failed"));
    assert!(error.contains("task2"));
    assert!(error.contains("Test error"));
}

#[test]
fn test_handle_response_all_failures() {
    let results = vec![
        create_test_task_result("task1", TaskStatus::Failed, Some("Error 1".to_string())),
        create_test_task_result("task2", TaskStatus::Failed, Some("Error 2".to_string())),
    ];
    let response = create_test_execution_response(results, 2);

    let result = handle_response(response);

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("2/2 tasks failed"));
    assert!(error.contains("task1"));
    assert!(error.contains("task2"));
    assert!(error.contains("Error 1"));
    assert!(error.contains("Error 2"));
}

#[test]
fn test_get_task_description() {
    let result = create_test_task_result("test_task_123", TaskStatus::Completed, None);

    let description = get_task_description(&result);

    assert_eq!(description, "ID: test_task_123");
}
