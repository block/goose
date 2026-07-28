//! Regression: the `orchestrator` platform extension must not resurrect
//! scheduling inside a runtime whose host disabled it.
//!
//! `orchestrator` is loadable into an ACP process (`goose acp --with-builtin
//! orchestrator`, or via persisted extension configuration) and every one of
//! its tools reaches the process-global `AgentManager` through the private
//! `get_agent_manager()`. That singleton owned `Paths::data_dir()/schedule.json`
//! unconditionally, so touching any orchestrator tool restarted cron execution
//! and schedule-file mutation behind the kill-switch's back.
//!
//! This binds the edge at the real orchestrator tool call rather than at a
//! stand-in: the test dispatches `orchestrator__list_sessions`, which calls
//! `get_agent_manager()` before it can produce a result.
//!
//! Its own integration-test binary, like the sibling
//! `agent_manager_scheduler_disabled` test: `AGENT_MANAGER` is a process-global
//! `OnceCell`, so the first initializer in a process decides what every later
//! caller observes.

use std::sync::Arc;

use goose::agents::{Agent, AgentConfig, ExtensionConfig, GoosePlatform};
use goose::config::permission::PermissionManager;
use goose::config::GooseMode;
use goose::scheduler::GOOSE_ACP_SCHEDULER_DISABLED_ENV;
use goose::session::session_manager::SessionType;
use goose::session::SessionManager;
use rmcp::model::CallToolRequestParams;

/// Stale running state, which an enabled scheduler rewrites at startup — so an
/// unchanged file proves the read never happened.
const STALE_RUNNING_JOB: &str = r#"[{"id":"sentinel","source":"/nonexistent/sentinel.yaml","cron":"0 0 0 * * *","currently_running":true}]"#;

/// Named so the assertion on the tool's output can prove `list_sessions`
/// produced a real listing rather than an error rendered as success.
const SESSION_NAME: &str = "orchestrator-disabled";

#[tokio::test]
async fn orchestrator_tool_call_starts_no_scheduler_when_disabled() {
    let root = tempfile::tempdir().unwrap();
    // `Paths::data_dir()` is `$GOOSE_PATH_ROOT/data`, and a non-absolute root is
    // silently ignored — which would aim this test at the real schedule file.
    assert!(
        root.path().is_absolute(),
        "GOOSE_PATH_ROOT is ignored unless absolute"
    );
    let data_dir = root.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let schedule_file = data_dir.join("schedule.json");
    std::fs::write(&schedule_file, STALE_RUNNING_JOB).unwrap();
    let modified_before = std::fs::metadata(&schedule_file)
        .unwrap()
        .modified()
        .unwrap();

    let _guard = env_lock::lock_env([
        (GOOSE_ACP_SCHEDULER_DISABLED_ENV, Some("true")),
        ("GOOSE_DISABLE_KEYRING", Some("true")),
        ("GOOSE_PATH_ROOT", root.path().to_str()),
    ]);

    // An ACP-shaped agent: no scheduler of its own, matching what
    // `AcpServer::create_agent()` builds when the switch is set.
    let session_manager = Arc::new(SessionManager::new(data_dir.clone()));
    let agent = Agent::with_config(AgentConfig::new(
        Arc::clone(&session_manager),
        PermissionManager::instance(),
        None,
        GooseMode::default(),
        false,
        GoosePlatform::GooseCli,
    ));
    let session = session_manager
        .create_session(
            root.path().to_path_buf(),
            SESSION_NAME.to_string(),
            // `list_sessions` reports only User and Scheduled sessions, and the
            // assertion below needs this one to appear in the listing.
            SessionType::User,
            GooseMode::default(),
        )
        .await
        .expect("session is creatable without a scheduler");

    agent
        .add_extension(
            ExtensionConfig::Platform {
                name: "orchestrator".to_string(),
                description: "Orchestrator".to_string(),
                display_name: Some("Orchestrator".to_string()),
                bundled: Some(true),
                available_tools: vec![],
            },
            &session.id,
        )
        .await
        .expect("orchestrator is loadable into a runtime without a scheduler");

    // `list_sessions` resolves the process-global AgentManager before it can
    // answer, which is the resurrection edge under test.
    let (_id, result) = agent
        .dispatch_tool_call(
            CallToolRequestParams::new("orchestrator__list_sessions"),
            "req-1".to_string(),
            None,
            &session,
        )
        .await;
    let tool_result = result.expect("orchestrator tool call is dispatchable");
    // The orchestrator converts every internal failure — including one from
    // resolving the agent manager — into `Ok(CallToolResult::error(..))`, so
    // the Rust `Ok` alone would also cover a run that never reached the
    // singleton. Assert on the MCP-level outcome and its payload instead.
    let call_result = tool_result
        .result
        .await
        .expect("orchestrator tool call is dispatchable");
    assert_ne!(
        call_result.is_error,
        Some(true),
        "the orchestrator tool must not return an MCP error, otherwise it might \
         never have reached the agent manager: {call_result:?}"
    );
    let listing = call_result
        .content
        .iter()
        .filter_map(|block| block.as_text().map(|text| text.text.as_str()))
        .collect::<String>();
    assert!(
        listing.contains(SESSION_NAME),
        "the listing must name the session this test created, proving \
         list_sessions ran past the agent manager: {listing:?}"
    );

    assert_eq!(
        std::fs::read_to_string(&schedule_file).unwrap(),
        STALE_RUNNING_JOB,
        "an orchestrator tool call must not let a scheduler rewrite the schedule file"
    );
    assert_eq!(
        std::fs::metadata(&schedule_file)
            .unwrap()
            .modified()
            .unwrap(),
        modified_before,
        "an orchestrator tool call must not touch the schedule file at all"
    );
}
