use super::{calculate_stats, create_empty_response, create_error_response};
use crate::agents::sub_recipe_execution_tool::lib::{TaskResult, TaskStatus};
use serde_json::json;

fn create_test_task_result(task_id: &str, status: TaskStatus) -> TaskResult {
    let is_failed = matches!(status, TaskStatus::Failed);
    TaskResult {
        task_id: task_id.to_string(),
        status,
        data: Some(json!({"output": "test output"})),
        error: if is_failed {
            Some("Test error".to_string())
        } else {
            None
        },
    }
}

#[test]
fn test_calculate_stats() {
    let results = vec![
        create_test_task_result("task1", TaskStatus::Completed),
        create_test_task_result("task2", TaskStatus::Completed),
        create_test_task_result("task3", TaskStatus::Failed),
        create_test_task_result("task4", TaskStatus::Completed),
    ];

    let stats = calculate_stats(&results, 1500);

    assert_eq!(stats.total_tasks, 4);
    assert_eq!(stats.completed, 3);
    assert_eq!(stats.failed, 1);
    assert_eq!(stats.execution_time_ms, 1500);
}

#[test]
fn test_calculate_stats_empty_results() {
    let results = vec![];
    let stats = calculate_stats(&results, 0);

    assert_eq!(stats.total_tasks, 0);
    assert_eq!(stats.completed, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(stats.execution_time_ms, 0);
}

#[test]
fn test_calculate_stats_all_completed() {
    let results = vec![
        create_test_task_result("task1", TaskStatus::Completed),
        create_test_task_result("task2", TaskStatus::Completed),
    ];

    let stats = calculate_stats(&results, 800);

    assert_eq!(stats.total_tasks, 2);
    assert_eq!(stats.completed, 2);
    assert_eq!(stats.failed, 0);
    assert_eq!(stats.execution_time_ms, 800);
}

#[test]
fn test_calculate_stats_all_failed() {
    let results = vec![
        create_test_task_result("task1", TaskStatus::Failed),
        create_test_task_result("task2", TaskStatus::Failed),
    ];

    let stats = calculate_stats(&results, 1200);

    assert_eq!(stats.total_tasks, 2);
    assert_eq!(stats.completed, 0);
    assert_eq!(stats.failed, 2);
    assert_eq!(stats.execution_time_ms, 1200);
}

#[test]
fn test_create_empty_response() {
    let response = create_empty_response();

    assert_eq!(response.status, "completed");
    assert_eq!(response.results.len(), 0);
    assert_eq!(response.stats.total_tasks, 0);
    assert_eq!(response.stats.completed, 0);
    assert_eq!(response.stats.failed, 0);
    assert_eq!(response.stats.execution_time_ms, 0);
}

#[test]
fn test_create_error_response() {
    let error_msg = "Test error message";
    let response = create_error_response(error_msg.to_string());

    assert_eq!(response.status, "failed");
    assert_eq!(response.results.len(), 0);
    assert_eq!(response.stats.total_tasks, 0);
    assert_eq!(response.stats.completed, 0);
    assert_eq!(response.stats.failed, 1);
    assert_eq!(response.stats.execution_time_ms, 0);
}
