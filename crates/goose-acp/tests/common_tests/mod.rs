// Required when compiled as standalone test "common"; harmless warning when included as module.
#![recursion_limit = "256"]
#![allow(unused_attributes)]

#[path = "../fixtures/mod.rs"]
pub mod fixtures;
use fixtures::{
    assert_notifications, Connection, FsFixture, Notification, OpenAiFixture, PermissionDecision,
    Session, SessionResult, TerminalCall, TerminalFixture, TestConnectionConfig,
};
use fs_err as fs;
use goose::config::base::CONFIG_YAML_NAME;
use goose::config::GooseMode;
use goose::providers::provider_registry::ProviderConstructor;
use goose_test_support::{McpFixture, FAKE_CODE, TEST_IMAGE_B64, TEST_MODEL};
use sacp::schema::{McpServer, McpServerHttp, ModelId, SessionModeId, ToolCallStatus, ToolKind};
use std::sync::Arc;

const SHELL_TEST_CONTENT: &str = "test-shell-content-98765";

pub async fn run_config_mcp<C: Connection>() {
    let temp_dir = tempfile::tempdir().unwrap();
    let expected_session_id = C::expected_session_id();
    let prompt = "Use the get_code tool and output only its result.";
    let mcp = McpFixture::new(expected_session_id.clone()).await;

    let config_yaml = format!(
        "GOOSE_MODEL: {TEST_MODEL}\nGOOSE_PROVIDER: openai\nextensions:\n  mcp-fixture:\n    enabled: true\n    type: streamable_http\n    name: mcp-fixture\n    description: MCP fixture\n    uri: \"{}\"\n",
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

    let config = TestConnectionConfig {
        data_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.text, FAKE_CODE);
    assert_notifications(
        &session.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallContent("content".into()),
            Notification::ToolCallStatus(ToolCallStatus::Completed),
            Notification::AgentMessage,
        ],
    );
    expected_session_id.assert_matches(&session.session_id().0);
}

// Also proves developer loaded from config.yaml (not CLI args) gets ACP fs delegation.
pub async fn run_fs_read_text_file_true<C: Connection>() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_yaml = format!(
        "GOOSE_MODEL: {TEST_MODEL}\nGOOSE_PROVIDER: openai\nextensions:\n  developer:\n    enabled: true\n    type: platform\n    name: developer\n    description: Developer\n    display_name: Developer\n    bundled: true\n    available_tools: []\n"
    );
    fs::write(temp_dir.path().join(CONFIG_YAML_NAME), config_yaml).unwrap();

    let expected_session_id = C::expected_session_id();
    let prompt = "Use the read tool to read /tmp/test_acp_read.txt and output only its contents.";
    let openai = OpenAiFixture::new(
        vec![
            (
                prompt.to_string(),
                include_str!("../test_data/openai_fs_read_tool_call.txt"),
            ),
            (
                r#""content":"test-read-content-12345""#.into(),
                include_str!("../test_data/openai_fs_read_tool_result.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let fs = FsFixture::new();
    let config = TestConnectionConfig {
        read_text_file: Some(fs.read_handler("/tmp/test_acp_read.txt", "test-read-content-12345")),
        data_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.text, "test-read-content-12345");
    assert_notifications(
        &session.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallKind(ToolKind::Read),
            Notification::ToolCallStatus(ToolCallStatus::Completed),
            Notification::AgentMessage,
        ],
    );
    fs.assert_called();
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_fs_write_text_file_false<C: Connection>() {
    let _ = fs::remove_file("/tmp/test_acp_write.txt");

    let expected_session_id = C::expected_session_id();
    let prompt =
        "Use the write tool to write 'test-write-content-67890' to /tmp/test_acp_write.txt";
    let openai = OpenAiFixture::new(
        vec![
            (
                prompt.to_string(),
                include_str!("../test_data/openai_fs_write_tool_call.txt"),
            ),
            (
                r#"Created /tmp/test_acp_write.txt"#.into(),
                include_str!("../test_data/openai_fs_write_tool_result.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestConnectionConfig {
        builtins: vec!["developer".to_string()],
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session.prompt(prompt, PermissionDecision::AllowOnce).await;
    assert!(!output.text.is_empty());
    assert_eq!(
        fs::read_to_string("/tmp/test_acp_write.txt").unwrap(),
        "test-write-content-67890"
    );
    assert_notifications(
        &session.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallContent("content".into()),
            Notification::ToolCallStatus(ToolCallStatus::Completed),
            Notification::AgentMessage,
        ],
    );
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_fs_write_text_file_true<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let prompt =
        "Use the write tool to write 'test-write-content-67890' to /tmp/test_acp_write.txt";
    let openai = OpenAiFixture::new(
        vec![
            (
                prompt.to_string(),
                include_str!("../test_data/openai_fs_write_tool_call.txt"),
            ),
            (
                r#"Created /tmp/test_acp_write.txt"#.into(),
                include_str!("../test_data/openai_fs_write_tool_result.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let fs = FsFixture::new();
    let config = TestConnectionConfig {
        builtins: vec!["developer".to_string()],
        write_text_file: Some(
            fs.write_handler("/tmp/test_acp_write.txt", "test-write-content-67890"),
        ),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session.prompt(prompt, PermissionDecision::AllowOnce).await;
    assert!(!output.text.is_empty());
    assert_notifications(
        &session.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallKind(ToolKind::Edit),
            Notification::ToolCallContent("diff".into()),
            Notification::ToolCallStatus(ToolCallStatus::Completed),
            Notification::AgentMessage,
        ],
    );
    fs.assert_called();
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_initialize_doesnt_hit_provider<C: Connection>() {
    let provider_factory: ProviderConstructor =
        Arc::new(|_, _| Box::pin(async { Err(anyhow::anyhow!("no provider configured")) }));

    let openai = OpenAiFixture::new(vec![], C::expected_session_id()).await;
    let config = TestConnectionConfig {
        provider_factory: Some(provider_factory),
        ..Default::default()
    };

    let conn = C::new(config, openai).await;
    assert!(!conn.auth_methods().is_empty());
    assert!(conn
        .auth_methods()
        .iter()
        .any(|m| &*m.id.0 == "goose-provider"));
}

pub async fn run_load_mode<C: Connection>() {
    let temp_dir = tempfile::tempdir().unwrap();
    let expected_session_id = C::expected_session_id();
    let prompt = "Use the get_code tool and output only its result.";
    let mcp = McpFixture::new(expected_session_id.clone()).await;

    let config_yaml = format!(
        "GOOSE_MODEL: {TEST_MODEL}\nGOOSE_PROVIDER: openai\nextensions:\n  mcp-fixture:\n    enabled: true\n    type: streamable_http\n    name: mcp-fixture\n    description: MCP fixture\n    uri: \"{}\"\n",
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

    let config = TestConnectionConfig {
        data_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;

    let SessionResult { session, modes, .. } = conn.new_session().await;
    assert_eq!(
        modes.unwrap().current_mode_id,
        SessionModeId::new(<&str>::from(GooseMode::default()))
    );
    let session_id = session.session_id().0.to_string();
    conn.set_mode(&session_id, <&str>::from(GooseMode::Approve))
        .await
        .unwrap();

    let SessionResult {
        session: mut loaded,
        modes,
        ..
    } = conn.load_session(&session_id, vec![]).await;
    assert_eq!(
        modes.unwrap().current_mode_id,
        SessionModeId::new(<&str>::from(GooseMode::Approve))
    );

    // Approve mode + Cancel = permission denied → tool fails
    expected_session_id.set(&loaded.session_id().0);
    let output = loaded.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.tool_status.unwrap(), ToolCallStatus::Failed);
    assert_notifications(
        &loaded.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallContent("content".into()),
            Notification::ToolCallStatus(ToolCallStatus::Failed),
            Notification::AgentMessage,
        ],
    );
}

pub async fn run_load_model<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let openai = OpenAiFixture::new(
        vec![(
            r#""model":"o4-mini""#.into(),
            include_str!("../test_data/openai_basic.txt"),
        )],
        expected_session_id.clone(),
    )
    .await;

    let mut conn = C::new(TestConnectionConfig::default(), openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let session_id = session.session_id().0.to_string();
    conn.set_model(&session_id, "o4-mini").await.unwrap();

    let output = session
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");

    let SessionResult { models, .. } = conn.load_session(&session_id, vec![]).await;
    assert_eq!(&*models.unwrap().current_model_id.0, "o4-mini");
}

pub async fn run_load_session_mcp<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let prompt = "Use the get_code tool and output only its result.";
    let mcp = McpFixture::new(expected_session_id.clone()).await;
    let mcp_url = mcp.url.clone();

    // Two rounds of tool call + tool result: one for new session, one for loaded session.
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

    let mcp_servers = vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp_url))];

    let config = TestConnectionConfig {
        mcp_servers: mcp_servers.clone(),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    // First prompt: tool should work in the new session.
    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.text, FAKE_CODE, "tool call failed in new session");

    // Load the same session with MCP servers re-specified.
    let session_id = session.session_id().0.to_string();
    let SessionResult {
        session: mut loaded_session,
        ..
    } = conn.load_session(&session_id, mcp_servers).await;

    // Second prompt: tool should work in the loaded session.
    let output = loaded_session
        .prompt(prompt, PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, FAKE_CODE, "tool call failed in loaded session");
}

pub async fn run_mode_set<C: Connection>() {
    let temp_dir = tempfile::tempdir().unwrap();
    let expected_session_id = C::expected_session_id();
    let prompt = "Use the get_code tool and output only its result.";
    let mcp = McpFixture::new(expected_session_id.clone()).await;

    let config_yaml = format!(
        "GOOSE_MODEL: {TEST_MODEL}\nGOOSE_PROVIDER: openai\nextensions:\n  mcp-fixture:\n    enabled: true\n    type: streamable_http\n    name: mcp-fixture\n    description: MCP fixture\n    uri: \"{}\"\n",
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

    let config = TestConnectionConfig {
        data_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;

    let SessionResult {
        session: mut session_a,
        ..
    } = conn.new_session().await;

    let SessionResult {
        session: mut session_b,
        ..
    } = conn.new_session().await;
    conn.set_mode(&session_b.session_id().0, <&str>::from(GooseMode::Approve))
        .await
        .unwrap();

    // Approve mode + Cancel = permission denied → tool fails
    expected_session_id.set(&session_b.session_id().0);
    let output = session_b.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.tool_status.unwrap(), ToolCallStatus::Failed);
    assert_notifications(
        &session_b.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallContent("content".into()),
            Notification::ToolCallStatus(ToolCallStatus::Failed),
            Notification::AgentMessage,
        ],
    );

    // Auto mode ignores Cancel — tool succeeds without permission prompt
    conn.reset_openai();
    expected_session_id.set(&session_a.session_id().0);
    let output = session_a.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.text, FAKE_CODE);
    assert_notifications(
        &session_a.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallContent("content".into()),
            Notification::ToolCallStatus(ToolCallStatus::Completed),
            Notification::AgentMessage,
        ],
    );
}

pub async fn run_mode_set_error<C: Connection>(
    mode_id: &str,
    session_id_override: Option<&str>,
    expected: sacp::Error,
) {
    let openai = OpenAiFixture::new(vec![], C::expected_session_id()).await;
    let mut conn = C::new(TestConnectionConfig::default(), openai).await;
    let SessionResult { session, .. } = conn.new_session().await;

    let target_session_id = session_id_override
        .map(str::to_string)
        .unwrap_or_else(|| session.session_id().0.to_string());

    let err = conn
        .set_mode(&target_session_id, mode_id)
        .await
        .unwrap_err();

    let sacp_err = err.downcast::<sacp::Error>().unwrap();
    assert_eq!(sacp_err, expected);
}

#[macro_export]
macro_rules! tests_mode_set_error {
    ($conn:ty) => {
        #[test_case::test_case("not_a_mode", None, sacp::Error::invalid_params().data("Invalid mode: not_a_mode") ; "invalid mode")]
        #[test_case::test_case("auto", Some("nonexistent-session-id"), sacp::Error::invalid_params().data("Session not found: nonexistent-session-id") ; "session not found")]
        fn test_mode_set_error(
            mode_id: &'static str,
            session_id: Option<&'static str>,
            expected: sacp::Error,
        ) {
            common_tests::fixtures::run_test(async move {
                common_tests::run_mode_set_error::<$conn>(
                    mode_id, session_id, expected,
                )
                .await
            });
        }
    };
}

pub async fn run_model_list<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let openai = OpenAiFixture::new(vec![], expected_session_id.clone()).await;

    let mut conn = C::new(TestConnectionConfig::default(), openai).await;
    let SessionResult {
        session, models, ..
    } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let models = models.unwrap();
    assert!(!models.available_models.is_empty());
    assert_eq!(models.current_model_id, ModelId::new(TEST_MODEL));
}

pub async fn run_model_set<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let openai = OpenAiFixture::new(
        vec![
            // Session B prompt with switched model
            (
                r#""model":"o4-mini""#.into(),
                include_str!("../test_data/openai_basic.txt"),
            ),
            // Session A prompt with default model
            (
                format!(r#""model":"{TEST_MODEL}""#),
                include_str!("../test_data/openai_basic.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let mut conn = C::new(TestConnectionConfig::default(), openai).await;

    // Session A: default model
    let SessionResult {
        session: mut session_a,
        ..
    } = conn.new_session().await;

    // Session B: switch to o4-mini
    let SessionResult {
        session: mut session_b,
        ..
    } = conn.new_session().await;
    conn.set_model(&session_b.session_id().0, "o4-mini")
        .await
        .unwrap();

    // Prompt B — expects o4-mini
    expected_session_id.set(&session_b.session_id().0);
    let output = session_b
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");

    // Prompt A — expects default TEST_MODEL (proves sessions are independent)
    expected_session_id.set(&session_a.session_id().0);
    let output = session_a
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");
}

pub async fn run_permission_persistence<C: Connection>() {
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
    let expected_session_id = C::expected_session_id();
    let mcp = McpFixture::new(expected_session_id.clone()).await;
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

    let config = TestConnectionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        goose_mode: GooseMode::Approve,
        data_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    for (decision, expected_status, expected_yaml) in cases {
        conn.reset_openai();
        conn.reset_permissions();
        let _ = fs::remove_file(temp_dir.path().join("permission.yaml"));
        let output = session.prompt(prompt, decision).await;

        assert_eq!(output.tool_status.unwrap(), expected_status);
        assert_eq!(
            fs::read_to_string(temp_dir.path().join("permission.yaml")).unwrap_or_default(),
            expected_yaml,
        );
    }
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_prompt_basic<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let openai = OpenAiFixture::new(
        vec![(
            r#"</info-msg>\nwhat is 1+1""#.into(),
            include_str!("../test_data/openai_basic.txt"),
        )],
        expected_session_id.clone(),
    )
    .await;

    let mut conn = C::new(TestConnectionConfig::default(), openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");
    assert_notifications(&session.notifications(), &[Notification::AgentMessage]);
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_prompt_codemode<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let prompt =
        "Search for getCode and write tools. Use them to save the code to /tmp/result.txt.";
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
                r#"Created /tmp/result.txt"#.into(),
                include_str!("../test_data/openai_builtin_final.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestConnectionConfig {
        builtins: vec!["code_execution".to_string(), "developer".to_string()],
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        ..Default::default()
    };

    let _ = fs::remove_file("/tmp/result.txt");

    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    if matches!(output.tool_status, Some(ToolCallStatus::Failed)) || output.text.contains("error") {
        panic!("{}", output.text);
    }

    let result = fs::read_to_string("/tmp/result.txt").unwrap_or_default();
    assert_eq!(result, FAKE_CODE);
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_prompt_image<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let mcp = McpFixture::new(expected_session_id.clone()).await;
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

    let config = TestConnectionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session
        .prompt(
            "Use the get_image tool and describe what you see in its result.",
            PermissionDecision::Cancel,
        )
        .await;
    assert_eq!(output.text, "Hello Goose!\nThis is a test image.");
    assert_notifications(
        &session.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallContent("content".into()),
            Notification::ToolCallStatus(ToolCallStatus::Completed),
            Notification::AgentMessage,
        ],
    );
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_prompt_image_attachment<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let openai = OpenAiFixture::new(
        vec![(
            r#""type":"image_url""#.into(),
            include_str!("../test_data/openai_image_attachment.txt"),
        )],
        expected_session_id.clone(),
    )
    .await;

    let mut conn = C::new(TestConnectionConfig::default(), openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session
        .prompt_with_image(
            "Describe what you see in this image",
            TEST_IMAGE_B64,
            "image/png",
            PermissionDecision::Cancel,
        )
        .await;
    assert!(output.text.contains("Hello Goose!"));
    assert_notifications(&session.notifications(), &[Notification::AgentMessage]);
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_prompt_mcp<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let mcp = McpFixture::new(expected_session_id.clone()).await;
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

    let config = TestConnectionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session
        .prompt(
            "Use the get_code tool and output only its result.",
            PermissionDecision::Cancel,
        )
        .await;
    assert_eq!(output.text, FAKE_CODE);
    assert_notifications(
        &session.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallContent("content".into()),
            Notification::ToolCallStatus(ToolCallStatus::Completed),
            Notification::AgentMessage,
        ],
    );
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_prompt_skill<C: Connection>() {
    let cwd = tempfile::tempdir().unwrap();
    let skill_dir = cwd.path().join(".agents/skills/test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: skill-loaded-in-acp-session\n---\nTest instructions\n",
    )
    .unwrap();

    let expected_session_id = C::expected_session_id();
    let openai = OpenAiFixture::new(
        vec![(
            "skill-loaded-in-acp-session".to_string(),
            include_str!("../test_data/openai_basic.txt"),
        )],
        expected_session_id.clone(),
    )
    .await;

    let config = TestConnectionConfig {
        builtins: vec!["summon".to_string()],
        cwd: Some(cwd),
        ..Default::default()
    };

    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_shell_terminal_false<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let prompt = format!("Run the command echo {SHELL_TEST_CONTENT} and output only its result.");
    let openai = OpenAiFixture::new(
        vec![
            (
                prompt.clone(),
                include_str!("../test_data/openai_shell_tool_call.txt"),
            ),
            (
                SHELL_TEST_CONTENT.into(),
                include_str!("../test_data/openai_shell_tool_result.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let config = TestConnectionConfig {
        builtins: vec!["developer".to_string()],
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session.prompt(&prompt, PermissionDecision::AllowOnce).await;
    assert!(!output.text.is_empty());
    assert_notifications(
        &session.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallContent("content".into()),
            Notification::ToolCallStatus(ToolCallStatus::Completed),
            Notification::AgentMessage,
        ],
    );
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_shell_terminal_true<C: Connection>() {
    let expected_session_id = C::expected_session_id();
    let prompt = format!("Run the command echo {SHELL_TEST_CONTENT} and output only its result.");
    let openai = OpenAiFixture::new(
        vec![
            (
                prompt.clone(),
                include_str!("../test_data/openai_shell_tool_call.txt"),
            ),
            (
                SHELL_TEST_CONTENT.into(),
                include_str!("../test_data/openai_shell_tool_result.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    let command = format!("echo {SHELL_TEST_CONTENT}");
    let output_text = format!("{SHELL_TEST_CONTENT}\n");
    let tid = String::from("term-1");
    let terminal = TerminalFixture::new(vec![
        TerminalCall::Create(command.clone(), tid.clone()),
        TerminalCall::WaitForExit(tid.clone(), 0),
        TerminalCall::Output(tid.clone(), output_text.clone(), 0),
        TerminalCall::Release(tid),
    ]);
    let config = TestConnectionConfig {
        builtins: vec!["developer".to_string()],
        terminal: Some(terminal.clone()),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let SessionResult { mut session, .. } = conn.new_session().await;
    expected_session_id.set(&session.session_id().0);

    let output = session.prompt(&prompt, PermissionDecision::AllowOnce).await;
    assert_eq!(output.tool_status, Some(ToolCallStatus::Completed));
    assert_notifications(
        &session.notifications(),
        &[
            Notification::ToolCall,
            Notification::ToolCallKind(ToolKind::Execute),
            Notification::ToolCallContent("terminal".into()),
            Notification::ToolCallStatus(ToolCallStatus::Completed),
            Notification::AgentMessage,
        ],
    );
    terminal.assert_called();
    expected_session_id.assert_matches(&session.session_id().0);
}
