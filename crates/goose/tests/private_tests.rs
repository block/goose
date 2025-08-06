#![cfg(test)]

use mcp_core::ToolError;
use serde_json::json;

use goose::agents::platform_tools::PLATFORM_MANAGE_SCHEDULE_TOOL_NAME;
mod test_support;
use test_support::{
    create_temp_recipe, create_test_session_metadata, MockBehavior, ScheduleToolTestBuilder,
};

// Test all actions of the scheduler platform tool
#[tokio::test]
async fn test_schedule_tool_list_action() {
    // Create a test builder with existing jobs
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .with_existing_job("job2", "0 0 * * * *")
        .await
        .build()
        .await;

    // Test list action
    let arguments = json!({
        "action": "list"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content.text.contains("Scheduled Jobs:"));
        assert!(text_content.text.contains("job1"));
        assert!(text_content.text.contains("job2"));
    } else {
        panic!("Expected text content");
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"list_scheduled_jobs".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_list_action_empty() {
    // Create a test builder with no jobs
    let (agent, scheduler) = ScheduleToolTestBuilder::new().build().await;

    // Test list action
    let arguments = json!({
        "action": "list"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content.text.contains("Scheduled Jobs:"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"list_scheduled_jobs".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_list_action_error() {
    // Create a test builder with a list error
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_scheduler_behavior(
            "list_scheduled_jobs",
            MockBehavior::InternalError("Database error".to_string()),
        )
        .await
        .build()
        .await;

    // Test list action
    let arguments = json!({
        "action": "list"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Failed to list jobs"));
        assert!(msg.contains("Database error"));
    } else {
        panic!("Expected ExecutionError");
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"list_scheduled_jobs".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_create_action() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new().build().await;

    // Create a temporary recipe file
    let temp_recipe = create_temp_recipe(true, "json");

    // Test create action
    let arguments = json!({
        "action": "create",
        "recipe_path": temp_recipe.path.to_str().unwrap(),
        "cron_expression": "*/5 * * * * *"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content
            .text
            .contains("Successfully created scheduled job"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"add_scheduled_job".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_create_action_missing_params() {
    let (agent, _) = ScheduleToolTestBuilder::new().build().await;

    // Test create action with missing recipe_path
    let arguments = json!({
        "action": "create",
        "cron_expression": "*/5 * * * * *"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Missing 'recipe_path' parameter"));
    } else {
        panic!("Expected ExecutionError");
    }

    // Test create action with missing cron_expression
    let temp_recipe = create_temp_recipe(true, "json");
    let arguments = json!({
        "action": "create",
        "recipe_path": temp_recipe.path.to_str().unwrap()
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Missing 'cron_expression' parameter"));
    } else {
        panic!("Expected ExecutionError");
    }
}

#[tokio::test]
async fn test_schedule_tool_create_action_nonexistent_recipe() {
    let (agent, _) = ScheduleToolTestBuilder::new().build().await;

    // Test create action with nonexistent recipe
    let arguments = json!({
        "action": "create",
        "recipe_path": "/nonexistent/recipe.json",
        "cron_expression": "*/5 * * * * *"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Recipe file not found"));
    } else {
        panic!("Expected ExecutionError");
    }
}

#[tokio::test]
async fn test_schedule_tool_create_action_invalid_recipe() {
    let (agent, _) = ScheduleToolTestBuilder::new().build().await;

    // Create an invalid recipe file
    let temp_recipe = create_temp_recipe(false, "json");

    // Test create action with invalid recipe
    let arguments = json!({
        "action": "create",
        "recipe_path": temp_recipe.path.to_str().unwrap(),
        "cron_expression": "*/5 * * * * *"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Invalid JSON recipe"));
    } else {
        panic!("Expected ExecutionError");
    }
}

#[tokio::test]
async fn test_schedule_tool_create_action_scheduler_error() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_scheduler_behavior(
            "add_scheduled_job",
            MockBehavior::AlreadyExists("job1".to_string()),
        )
        .await
        .build()
        .await;

    // Create a temporary recipe file
    let temp_recipe = create_temp_recipe(true, "json");

    // Test create action
    let arguments = json!({
        "action": "create",
        "recipe_path": temp_recipe.path.to_str().unwrap(),
        "cron_expression": "*/5 * * * * *"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Failed to create job"));
        assert!(msg.contains("job1"));
    } else {
        panic!("Expected ExecutionError");
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"add_scheduled_job".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_run_now_action() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .build()
        .await;

    // Test run_now action
    let arguments = json!({
        "action": "run_now",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content
            .text
            .contains("Successfully started job 'job1'"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"run_now".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_run_now_action_missing_job_id() {
    let (agent, _) = ScheduleToolTestBuilder::new().build().await;

    // Test run_now action with missing job_id
    let arguments = json!({
        "action": "run_now"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Missing 'job_id' parameter"));
    } else {
        panic!("Expected ExecutionError");
    }
}

#[tokio::test]
async fn test_schedule_tool_run_now_action_nonexistent_job() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_scheduler_behavior("run_now", MockBehavior::NotFound("nonexistent".to_string()))
        .await
        .build()
        .await;

    // Test run_now action with nonexistent job
    let arguments = json!({
        "action": "run_now",
        "job_id": "nonexistent"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Failed to run job"));
        assert!(msg.contains("nonexistent"));
    } else {
        panic!("Expected ExecutionError");
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"run_now".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_pause_action() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .build()
        .await;

    // Test pause action
    let arguments = json!({
        "action": "pause",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content.text.contains("Successfully paused job 'job1'"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"pause_schedule".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_pause_action_missing_job_id() {
    let (agent, _) = ScheduleToolTestBuilder::new().build().await;

    // Test pause action with missing job_id
    let arguments = json!({
        "action": "pause"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Missing 'job_id' parameter"));
    } else {
        panic!("Expected ExecutionError");
    }
}

#[tokio::test]
async fn test_schedule_tool_pause_action_running_job() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_scheduler_behavior(
            "pause_schedule",
            MockBehavior::JobCurrentlyRunning("job1".to_string()),
        )
        .await
        .build()
        .await;

    // Test pause action with a running job
    let arguments = json!({
        "action": "pause",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Failed to pause job"));
        assert!(msg.contains("job1"));
    } else {
        panic!("Expected ExecutionError");
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"pause_schedule".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_unpause_action() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .build()
        .await;

    // Test unpause action
    let arguments = json!({
        "action": "unpause",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content
            .text
            .contains("Successfully unpaused job 'job1'"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"unpause_schedule".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_delete_action() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .build()
        .await;

    // Test delete action
    let arguments = json!({
        "action": "delete",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content
            .text
            .contains("Successfully deleted job 'job1'"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"remove_scheduled_job".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_kill_action() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .with_running_job("job1")
        .await
        .build()
        .await;

    // Test kill action
    let arguments = json!({
        "action": "kill",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content
            .text
            .contains("Successfully killed running job 'job1'"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"kill_running_job".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_kill_action_not_running() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .build()
        .await;

    // Test kill action with a job that's not running
    let arguments = json!({
        "action": "kill",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Failed to kill job"));
    } else {
        panic!("Expected ExecutionError");
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"kill_running_job".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_inspect_action_running() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .with_running_job("job1")
        .await
        .build()
        .await;

    // Test inspect action
    let arguments = json!({
        "action": "inspect",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content
            .text
            .contains("Job 'job1' is currently running"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"get_running_job_info".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_inspect_action_not_running() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .build()
        .await;

    // Test inspect action with a job that's not running
    let arguments = json!({
        "action": "inspect",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content
            .text
            .contains("Job 'job1' is not currently running"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"get_running_job_info".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_sessions_action() {
    // Create test session metadata
    let sessions = vec![
        (
            "1234567890_session1".to_string(),
            create_test_session_metadata(5, "/tmp"),
        ),
        (
            "0987654321_session2".to_string(),
            create_test_session_metadata(10, "/home"),
        ),
    ];

    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .with_sessions_data("job1", sessions)
        .await
        .build()
        .await;

    // Test sessions action
    let arguments = json!({
        "action": "sessions",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content.text.contains("Sessions for job 'job1'"));
        assert!(text_content.text.contains("session1"));
        assert!(text_content.text.contains("session2"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"sessions".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_sessions_action_with_limit() {
    // Create test session metadata
    let sessions = vec![
        (
            "1234567890_session1".to_string(),
            create_test_session_metadata(5, "/tmp"),
        ),
        (
            "0987654321_session2".to_string(),
            create_test_session_metadata(10, "/home"),
        ),
        (
            "5555555555_session3".to_string(),
            create_test_session_metadata(15, "/usr"),
        ),
    ];

    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .with_sessions_data("job1", sessions)
        .await
        .build()
        .await;

    // Test sessions action with limit
    let arguments = json!({
        "action": "sessions",
        "job_id": "job1",
        "limit": 2
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"sessions".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_sessions_action_empty() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .build()
        .await;

    // Test sessions action with no sessions
    let arguments = json!({
        "action": "sessions",
        "job_id": "job1"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.len(), 1);
    if let Some(text_content) = content[0].as_text() {
        assert!(text_content
            .text
            .contains("No sessions found for job 'job1'"));
    }

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"sessions".to_string()));
}

#[tokio::test]
async fn test_schedule_tool_session_content_action() {
    let (agent, _) = ScheduleToolTestBuilder::new().build().await;

    // Test with a non-existent session
    let arguments = json!({
        "action": "session_content",
        "session_id": "non_existent_session"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Session 'non_existent_session' not found"));
    } else {
        panic!("Expected ExecutionError");
    }
}

#[tokio::test]
async fn test_schedule_tool_session_content_action_with_real_session() {
    let (agent, _) = ScheduleToolTestBuilder::new().build().await;

    // Create a temporary session file in the proper session directory
    let session_dir = goose::session::storage::ensure_session_dir().unwrap();
    let session_id = "test_session_real";
    let session_path = session_dir.join(format!("{}.jsonl", session_id));

    // Create test metadata and messages
    let metadata = create_test_session_metadata(2, "/tmp");
    let messages = goose::conversation::Conversation::new_unvalidated(vec![
        goose::conversation::message::Message::user().with_text("Hello"),
        goose::conversation::message::Message::assistant().with_text("Hi there!"),
    ]);

    // Save the session file
    goose::session::storage::save_messages_with_metadata(&session_path, &metadata, &messages)
        .unwrap();

    // Test the session_content action
    let arguments = json!({
        "action": "session_content",
        "session_id": session_id
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;

    // Clean up the test session file
    let _ = std::fs::remove_file(&session_path);

    // Verify the result
    assert!(result.is_ok());

    if let Ok(content) = result {
        assert_eq!(content.len(), 1);
        if let Some(text_content) = content[0].as_text() {
            assert!(text_content
                .text
                .contains("Session 'test_session_real' Content:"));
            assert!(text_content.text.contains("Metadata:"));
            assert!(text_content.text.contains("Messages:"));
            assert!(text_content.text.contains("Hello"));
            assert!(text_content.text.contains("Hi there!"));
            assert!(text_content.text.contains("Test session"));
        } else {
            panic!("Expected text content");
        }
    } else {
        panic!("Expected successful result");
    }
}

#[tokio::test]
async fn test_schedule_tool_session_content_action_missing_session_id() {
    let (agent, _) = ScheduleToolTestBuilder::new().build().await;

    // Test session_content action with missing session_id
    let arguments = json!({
        "action": "session_content"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Missing 'session_id' parameter"));
    } else {
        panic!("Expected ExecutionError");
    }
}

#[tokio::test]
async fn test_schedule_tool_unknown_action() {
    let (agent, _) = ScheduleToolTestBuilder::new().build().await;

    // Test unknown action
    let arguments = json!({
        "action": "unknown_action"
    });

    let result = agent
        .handle_schedule_management(arguments, "test_req".to_string())
        .await;
    assert!(result.is_err());

    if let Err(ToolError::ExecutionError(msg)) = result {
        assert!(msg.contains("Unknown action"));
    } else {
        panic!("Expected ExecutionError");
    }
}

#[tokio::test]
async fn test_schedule_tool_dispatch() {
    let (agent, scheduler) = ScheduleToolTestBuilder::new()
        .with_existing_job("job1", "*/5 * * * * *")
        .await
        .build()
        .await;

    // Test that the tool is properly dispatched through dispatch_tool_call
    let tool_call = mcp_core::tool::ToolCall {
        name: PLATFORM_MANAGE_SCHEDULE_TOOL_NAME.to_string(),
        arguments: json!({
            "action": "list"
        }),
    };

    let (request_id, result) = agent
        .dispatch_tool_call(tool_call, "test_dispatch".to_string(), None)
        .await;
    assert_eq!(request_id, "test_dispatch");
    assert!(result.is_ok());

    let tool_result = result.unwrap();
    // The result should be a future that resolves to the tool output
    let output = tool_result.result.await;
    assert!(output.is_ok());

    // Verify the scheduler was called
    let calls = scheduler.get_calls().await;
    assert!(calls.contains(&"list_scheduled_jobs".to_string()));
}
