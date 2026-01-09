use goose::agents::Agent;
use goose::config::paths::Paths;
use goose::scheduler::Scheduler;
use rmcp::model::{Content, ErrorCode};

#[tokio::test]
async fn schedule_tool_errors_without_scheduler() {
    let agent = Agent::new();
    let args = serde_json::json!({
        "action": "list"
    });

    let result = agent
        .handle_schedule_management(args, "req-1".to_string())
        .await;

    assert!(result.is_err(), "Expected error when scheduler is not set");
    let err = result.err().unwrap();
    assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
    assert!(
        err.message.contains("Scheduler not available"),
        "Unexpected error message: {}",
        err.message
    );
}

#[tokio::test]
async fn schedule_tool_lists_jobs_with_scheduler() {
    let agent = Agent::new();
    let schedule_file_path = Paths::data_dir().join("schedule.json");
    let scheduler = Scheduler::new(schedule_file_path)
        .await
        .expect("Failed to init scheduler");
    agent.set_scheduler(scheduler).await;

    let args = serde_json::json!({
        "action": "list"
    });

    let result = agent
        .handle_schedule_management(args, "req-2".to_string())
        .await;

    let ok = result.expect("Expected Ok with scheduler set");
    assert!(ok.iter().any(|c| c.as_text().is_some_and(|t| t.text.contains("Scheduled Jobs"))),
        "Expected 'Scheduled Jobs' in response");
}
