use goose::config::GooseMode;
use goose::conversation::message::{Message, MessageContent, ToolRequest};
use goose::security::adversary_inspector::AdversaryInspector;
use goose::tool_inspection::ToolInspector;
use rmcp::model::CallToolRequestParams;
use rmcp::object;
use std::sync::Arc;
use tokio::sync::Mutex;

fn make_shell_request(id: &str, command: &str) -> ToolRequest {
    ToolRequest {
        id: id.into(),
        tool_call: Ok(
            CallToolRequestParams::new("shell").with_arguments(object!({"command": command}))
        ),
        metadata: None,
        tool_meta: None,
    }
}

#[tokio::test]
async fn test_adversary_disabled_without_config_file() {
    let provider = Arc::new(Mutex::new(None));
    let inspector = AdversaryInspector::new(provider);

    assert_eq!(inspector.name(), "adversary");

    // Without GOOSE_PATH_ROOT pointing to a dir with adversary.md, inspector is disabled
    if std::env::var("GOOSE_PATH_ROOT").is_err() {
        assert!(!inspector.is_enabled());

        let results = inspector
            .inspect(
                "test-session",
                &[make_shell_request("r1", "rm -rf /")],
                &[],
                GooseMode::SmartApprove,
            )
            .await
            .unwrap();

        assert!(results.is_empty());
    }
}

#[tokio::test]
async fn test_adversary_enabled_with_config_file() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("adversary.md"),
        "BLOCK everything for testing",
    )
    .unwrap();

    // GOOSE_PATH_ROOT redirects Paths::config_dir() to tmp/config
    std::env::set_var("GOOSE_PATH_ROOT", tmp.path());

    let provider = Arc::new(Mutex::new(None));
    let inspector = AdversaryInspector::new(provider);

    assert!(inspector.is_enabled());

    // With no provider available, consult_llm returns allow (fail-open)
    let messages = vec![Message::new(
        rmcp::model::Role::User,
        chrono::Utc::now().timestamp(),
        vec![MessageContent::text("build the project")],
    )];

    let results = inspector
        .inspect(
            "test-session",
            &[make_shell_request("r1", "cargo build")],
            &messages,
            GooseMode::SmartApprove,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    // No provider → fail-open → Allow
    assert!(matches!(
        results[0].action,
        goose::tool_inspection::InspectionAction::Allow
    ));

    std::env::remove_var("GOOSE_PATH_ROOT");
}
