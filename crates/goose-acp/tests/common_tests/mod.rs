// Required when compiled as standalone test "common"; harmless warning when included as module.
#![recursion_limit = "256"]
#![allow(unused_attributes)]

#[path = "../fixtures/mod.rs"]
pub mod fixtures;
use fixtures::{OpenAiFixture, PermissionDecision, Session, TestSessionConfig};
use fs_err as fs;
use goose::config::base::CONFIG_YAML_NAME;
use goose::config::GooseMode;
use goose_test_support::{ExpectedSessionId, McpFixture, FAKE_CODE, TEST_MODEL};
use sacp::schema::{
    McpServer, McpServerHttp, ModelId, ModelInfo, SessionModelState, ToolCallStatus,
};

pub async fn run_config_mcp<S: Session>() {
    let temp_dir = tempfile::tempdir().unwrap();
    let expected_session_id = ExpectedSessionId::default();
    let prompt = "Use the get_code tool and output only its result.";
    let mcp = McpFixture::new(Some(expected_session_id.clone())).await;

    let config_yaml = format!(
        "GOOSE_MODEL: {TEST_MODEL}\nextensions:\n  mcp-fixture:\n    enabled: true\n    type: streamable_http\n    name: mcp-fixture\n    description: MCP fixture\n    uri: \"{}\"\n",
        mcp.url
    );
    fs::write(temp_dir.path().join(CONFIG_YAML_NAME), config_yaml).unwrap();

    let openai = OpenAiFixture::new(
        vec![
            (
                prompt.to_string(),
                include_str!("../test_data/openai_tool_call.txt"),
            ),
            (
                format!(r#""content":"{FAKE_CODE}""#),
                include_str!("../test_data/openai_tool_result.txt"),
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
    expected_session_id.set(session.id().0.to_string());

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.text, FAKE_CODE);
    expected_session_id.assert_matches(&session.id().0);
}

pub async fn run_permission_persistence<S: Session>() {
    let cases = vec![
        (
            PermissionDecision::AllowAlways,
            ToolCallStatus::Completed,
            "user:\n  always_allow:\n  - mcp-fixture__get_code\n  ask_before: []\n  never_allow: []\n",
        ),
        (PermissionDecision::AllowOnce, ToolCallStatus::Completed, ""),
        (
            PermissionDecision::RejectAlways,
            ToolCallStatus::Failed,
            "user:\n  always_allow: []\n  ask_before: []\n  never_allow:\n  - mcp-fixture__get_code\n",
        ),
        (PermissionDecision::RejectOnce, ToolCallStatus::Failed, ""),
        (PermissionDecision::Cancel, ToolCallStatus::Failed, ""),
    ];

    let temp_dir = tempfile::tempdir().unwrap();
    let prompt = "Use the get_code tool and output only its result.";
    let expected_session_id = ExpectedSessionId::default();
    let mcp = McpFixture::new(Some(expected_session_id.clone())).await;
    let openai = OpenAiFixture::new(
        vec![
            (
                prompt.to_string(),
                include_str!("../test_data/openai_tool_call.txt"),
            ),
            (
                format!(r#""content":"{FAKE_CODE}""#),
                include_str!("../test_data/openai_tool_result.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestSessionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        goose_mode: GooseMode::Approve,
        data_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let mut session = S::new(config, openai).await;
    expected_session_id.set(session.id().0.to_string());

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

pub async fn run_prompt_basic<S: Session>() {
    let expected_session_id = ExpectedSessionId::default();
    let openai = OpenAiFixture::new(
        vec![(
            r#"</info-msg>\nwhat is 1+1""#.into(),
            include_str!("../test_data/openai_basic.txt"),
        )],
        expected_session_id.clone(),
    )
    .await;

    let mut session = S::new(TestSessionConfig::default(), openai).await;
    expected_session_id.set(session.id().0.to_string());

    let output = session
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");
    expected_session_id.assert_matches(&session.id().0);
}

pub async fn run_prompt_codemode<S: Session>() {
    let expected_session_id = ExpectedSessionId::default();
    let prompt =
        "Search for getCode and textEditor tools. Use them to save the code to /tmp/result.txt.";
    let mcp = McpFixture::new(Some(expected_session_id.clone())).await;
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
                r#"Successfully wrote to /tmp/result.txt"#.into(),
                include_str!("../test_data/openai_builtin_final.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestSessionConfig {
        builtins: vec!["code_execution".to_string(), "developer".to_string()],
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        ..Default::default()
    };

    let _ = fs::remove_file("/tmp/result.txt");

    let mut session = S::new(config, openai).await;
    expected_session_id.set(session.id().0.to_string());

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    if matches!(output.tool_status, Some(ToolCallStatus::Failed)) || output.text.contains("error") {
        panic!("{}", output.text);
    }

    let result = fs::read_to_string("/tmp/result.txt").unwrap_or_default();
    assert_eq!(result, format!("{FAKE_CODE}\n"));
    expected_session_id.assert_matches(&session.id().0);
}

pub async fn run_prompt_image<S: Session>() {
    let expected_session_id = ExpectedSessionId::default();
    let mcp = McpFixture::new(Some(expected_session_id.clone())).await;
    let openai = OpenAiFixture::new(
        vec![
            (
                r#"</info-msg>\nUse the get_image tool and describe what you see in its result.""#
                    .into(),
                include_str!("../test_data/openai_image_tool_call.txt"),
            ),
            (
                r#""type":"image_url""#.into(),
                include_str!("../test_data/openai_image_tool_result.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestSessionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        ..Default::default()
    };
    let mut session = S::new(config, openai).await;
    expected_session_id.set(session.id().0.to_string());

    let output = session
        .prompt(
            "Use the get_image tool and describe what you see in its result.",
            PermissionDecision::Cancel,
        )
        .await;
    assert_eq!(output.text, "Hello Goose!\nThis is a test image.");
    expected_session_id.assert_matches(&session.id().0);
}

pub async fn run_prompt_mcp<S: Session>() {
    let expected_session_id = ExpectedSessionId::default();
    let mcp = McpFixture::new(Some(expected_session_id.clone())).await;
    let openai = OpenAiFixture::new(
        vec![
            (
                r#"</info-msg>\nUse the get_code tool and output only its result.""#.into(),
                include_str!("../test_data/openai_tool_call.txt"),
            ),
            (
                format!(r#""content":"{FAKE_CODE}""#),
                include_str!("../test_data/openai_tool_result.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestSessionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        ..Default::default()
    };
    let mut session = S::new(config, openai).await;
    expected_session_id.set(session.id().0.to_string());

    let output = session
        .prompt(
            "Use the get_code tool and output only its result.",
            PermissionDecision::Cancel,
        )
        .await;
    assert_eq!(output.text, FAKE_CODE);
    expected_session_id.assert_matches(&session.id().0);
}

pub async fn run_model_list<S: Session>() {
    let expected_session_id = ExpectedSessionId::default();
    let openai = OpenAiFixture::new(vec![], expected_session_id.clone()).await;

    let session = S::new(TestSessionConfig::default(), openai).await;
    expected_session_id.set(session.id().0.to_string());

    let models = session.models().unwrap();
    let expected = SessionModelState::new(
        ModelId::new(TEST_MODEL),
        [
            "gpt-5.2",
            "gpt-5.2-2025-12-11",
            "gpt-5.2-chat-latest",
            "gpt-5.2-codex",
            "gpt-5.2-pro",
            "gpt-5.2-pro-2025-12-11",
            "gpt-5.1",
            "gpt-5.1-2025-11-13",
            "gpt-5.1-chat-latest",
            "gpt-5.1-codex",
            "gpt-5.1-codex-max",
            "gpt-5.1-codex-mini",
            "gpt-5-pro",
            "gpt-5-pro-2025-10-06",
            "gpt-5-codex",
            "gpt-5",
            "gpt-5-2025-08-07",
            "gpt-5-chat-latest",
            "gpt-5-mini",
            "gpt-5-mini-2025-08-07",
            TEST_MODEL,
            "gpt-5-nano-2025-08-07",
            "codex-mini-latest",
            "o3",
            "o3-2025-04-16",
            "o4-mini",
            "o4-mini-2025-04-16",
            "gpt-4.1",
            "gpt-4.1-2025-04-14",
            "gpt-4.1-mini",
            "gpt-4.1-mini-2025-04-14",
            "gpt-4.1-nano",
            "gpt-4.1-nano-2025-04-14",
            "o1-pro",
            "o1-pro-2025-03-19",
            "o3-mini",
            "o3-mini-2025-01-31",
            "o1",
            "o1-2024-12-17",
            "gpt-4o",
            "gpt-4o-2024-05-13",
            "gpt-4o-2024-08-06",
            "gpt-4o-2024-11-20",
            "gpt-4o-mini",
            "gpt-4o-mini-2024-07-18",
            "o4-mini-deep-research",
            "o4-mini-deep-research-2025-06-26",
            "text-embedding-3-large",
            "text-embedding-3-small",
            "gpt-4",
            "gpt-4-0613",
            "gpt-4-turbo",
            "gpt-4-turbo-2024-04-09",
            "gpt-3.5-turbo",
            "gpt-3.5-turbo-0125",
            "gpt-3.5-turbo-1106",
            "text-embedding-ada-002",
        ]
        .iter()
        .map(|id| ModelInfo::new(ModelId::new(*id), *id))
        .collect(),
    );
    assert_eq!(*models, expected);
}

pub async fn run_set_model<S: Session>() {
    let expected_session_id = ExpectedSessionId::default();
    let openai = OpenAiFixture::new(
        vec![(
            r#""model":"o4-mini""#.into(),
            include_str!("../test_data/openai_basic.txt"),
        )],
        expected_session_id.clone(),
    )
    .await;

    let mut session = S::new(TestSessionConfig::default(), openai).await;
    expected_session_id.set(session.id().0.to_string());

    session.set_model("o4-mini").await;

    let output = session
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");
}
