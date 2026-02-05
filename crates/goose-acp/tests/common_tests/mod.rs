// Required when compiled as standalone test "common"; harmless warning when included as module.
#![recursion_limit = "256"]
#![allow(unused_attributes)]

#[path = "../fixtures/mod.rs"]
pub mod fixtures;
use fixtures::{
    ExpectedSessionId, McpFixture, OpenAiFixture, PermissionDecision, Session, TestSessionConfig,
    FAKE_CODE,
};
use fs_err as fs;
use goose::config::base::CONFIG_YAML_NAME;
use goose::config::GooseMode;
use sacp::schema::{McpServer, McpServerHttp, ToolCallStatus};

pub async fn run_basic_completion<S: Session>() {
    let expected_session_id = ExpectedSessionId::default();
    let openai = OpenAiFixture::new(
        vec![(
            r#"</info-msg>\nwhat is 1+1""#.into(),
            include_str!("../test_data/openai_basic_response.txt"),
        )],
        expected_session_id.clone(),
    )
    .await;

    let mut session = S::new(TestSessionConfig::default(), openai).await;
    expected_session_id.set(session.id());

    let output = session
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");
    expected_session_id.assert_matches(&session.id().0);
}

pub async fn run_mcp_http_server<S: Session>() {
    let expected_session_id = ExpectedSessionId::default();
    let mcp = McpFixture::new(expected_session_id.clone()).await;
    let openai = OpenAiFixture::new(
        vec![
            (
                r#"</info-msg>\nUse the get_code tool and output only its result.""#.into(),
                include_str!("../test_data/openai_tool_call_response.txt"),
            ),
            (
                format!(r#""content":"{FAKE_CODE}""#),
                include_str!("../test_data/openai_tool_result_response.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestSessionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("lookup", &mcp.url))],
        ..Default::default()
    };
    let mut session = S::new(config, openai).await;
    expected_session_id.set(session.id());

    let output = session
        .prompt(
            "Use the get_code tool and output only its result.",
            PermissionDecision::Cancel,
        )
        .await;
    assert_eq!(output.text, FAKE_CODE);
    expected_session_id.assert_matches(&session.id().0);
}

pub async fn run_builtin_and_mcp<S: Session>() {
    let expected_session_id = ExpectedSessionId::default();
    let prompt =
        "Search for getCode and textEditor tools. Use them to save the code to /tmp/result.txt.";
    let mcp = McpFixture::new(expected_session_id.clone()).await;
    let openai = OpenAiFixture::new(
        vec![
            (
                format!(r#"</info-msg>\n{prompt}""#),
                include_str!("../test_data/openai_builtin_search.txt"),
            ),
            (
                r#"export async function getCode"#.into(),
                include_str!("../test_data/openai_builtin_execute.txt"),
            ),
            (
                r#"\"writeResult\": \"Successfully wrote to /tmp/result.txt"#.into(),
                include_str!("../test_data/openai_builtin_final.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestSessionConfig {
        builtins: vec!["code_execution".to_string(), "developer".to_string()],
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("lookup", &mcp.url))],
        ..Default::default()
    };

    let _ = fs::remove_file("/tmp/result.txt");

    let mut session = S::new(config, openai).await;
    expected_session_id.set(session.id());

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    if matches!(output.tool_status, Some(ToolCallStatus::Failed)) || output.text.contains("error") {
        panic!("{}", output.text);
    }

    let result = fs::read_to_string("/tmp/result.txt").unwrap_or_default();
    assert_eq!(result, format!("{FAKE_CODE}\n"));
    expected_session_id.assert_matches(&session.id().0);
}

pub async fn run_permission_persistence<S: Session>() {
    let cases = vec![
        (
            PermissionDecision::AllowAlways,
            ToolCallStatus::Completed,
            "user:\n  always_allow:\n  - lookup__get_code\n  ask_before: []\n  never_allow: []\n",
        ),
        (PermissionDecision::AllowOnce, ToolCallStatus::Completed, ""),
        (
            PermissionDecision::RejectAlways,
            ToolCallStatus::Failed,
            "user:\n  always_allow: []\n  ask_before: []\n  never_allow:\n  - lookup__get_code\n",
        ),
        (PermissionDecision::RejectOnce, ToolCallStatus::Failed, ""),
        (PermissionDecision::Cancel, ToolCallStatus::Failed, ""),
    ];

    let temp_dir = tempfile::tempdir().unwrap();
    let prompt = "Use the get_code tool and output only its result.";
    let expected_session_id = ExpectedSessionId::default();
    let mcp = McpFixture::new(expected_session_id.clone()).await;
    let openai = OpenAiFixture::new(
        vec![
            (
                prompt.to_string(),
                include_str!("../test_data/openai_tool_call_response.txt"),
            ),
            (
                format!(r#""content":"{FAKE_CODE}""#),
                include_str!("../test_data/openai_tool_result_response.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestSessionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("lookup", &mcp.url))],
        goose_mode: GooseMode::Approve,
        data_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let mut session = S::new(config, openai).await;
    expected_session_id.set(session.id());

    for (decision, expected_status, expected_yaml) in cases {
        session.reset_openai();
        session.reset_permissions();
        let _ = fs::remove_file(temp_dir.path().join("permission.yaml"));
        let output = session.prompt(prompt, decision).await;

        assert_eq!(
            output.tool_status.unwrap(),
            expected_status,
            "permission decision {:?}",
            decision
        );
        assert_eq!(
            fs::read_to_string(temp_dir.path().join("permission.yaml")).unwrap_or_default(),
            expected_yaml,
            "permission decision {:?}",
            decision
        );
    }
    expected_session_id.assert_matches(&session.id().0);
}

pub async fn run_configured_extension<S: Session>() {
    let temp_dir = tempfile::tempdir().unwrap();
    let expected_session_id = ExpectedSessionId::default();
    let prompt = "Use the get_code tool and output only its result.";
    let mcp = McpFixture::new(expected_session_id.clone()).await;

    let config_yaml = format!(
        "extensions:\n  lookup:\n    enabled: true\n    type: streamable_http\n    name: lookup\n    description: Lookup server\n    uri: \"{}\"\n",
        mcp.url
    );
    fs::write(temp_dir.path().join(CONFIG_YAML_NAME), config_yaml).unwrap();

    let openai = OpenAiFixture::new(
        vec![
            (
                prompt.to_string(),
                include_str!("../test_data/openai_tool_call_response.txt"),
            ),
            (
                format!(r#""content":"{FAKE_CODE}""#),
                include_str!("../test_data/openai_tool_result_response.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestSessionConfig {
        data_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let mut session = S::new(config, openai).await;
    expected_session_id.set(session.id());

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.text, FAKE_CODE);
    expected_session_id.assert_matches(&session.id().0);
}
