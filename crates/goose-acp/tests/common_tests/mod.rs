// Required when compiled as standalone test "common"; harmless warning when included as module.
#![recursion_limit = "256"]
#![allow(unused_attributes)]

#[path = "../fixtures/mod.rs"]
pub mod fixtures;
use fixtures::{
    initialize_agent, AssertReadFs, AssertWriteFs, Connection, NoOpFs, OpenAiFixture,
    PermissionDecision, Session, TestConnectionConfig,
};
use fs_err as fs;
use goose::agents::platform_extensions::developer::edit::LocalFs;
use goose::config::base::CONFIG_YAML_NAME;
use goose::config::GooseMode;
use goose::providers::provider_registry::ProviderConstructor;
use goose_acp::server::GooseAcpAgent;
use goose_test_support::{ExpectedSessionId, McpFixture, FAKE_CODE, TEST_MODEL};
use sacp::schema::{
    McpServer, McpServerHttp, ModelId, ReadTextFileResponse, ToolCallStatus, WriteTextFileResponse,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub async fn run_config_mcp<C: Connection>() {
    let temp_dir = tempfile::tempdir().unwrap();
    let expected_session_id = ExpectedSessionId::default();
    let prompt = "Use the get_code tool and output only its result.";
    let mcp = McpFixture::new(Some(expected_session_id.clone())).await;

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
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.text, FAKE_CODE);
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_fs_read_text_file_false<C: Connection>() {
    let expected_session_id = ExpectedSessionId::default();
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

    let config = TestConnectionConfig {
        builtins: vec!["developer".to_string()],
        fs: Some(AssertReadFs::new(
            "/tmp/test_acp_read.txt",
            "test-read-content-12345",
        )),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.text, "test-read-content-12345");
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_fs_read_text_file_true<C: Connection>() {
    let expected_session_id = ExpectedSessionId::default();
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

    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();
    let config = TestConnectionConfig {
        builtins: vec!["developer".to_string()],
        fs: Some(NoOpFs::new()),
        read_text_file: Some(Arc::new(move |req| {
            assert_eq!(req.path.to_str().unwrap(), "/tmp/test_acp_read.txt");
            called_clone.store(true, Ordering::SeqCst);
            ReadTextFileResponse::new("test-read-content-12345")
        })),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    assert_eq!(output.text, "test-read-content-12345");
    assert!(called.load(Ordering::SeqCst), "read handler was not called");
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_fs_write_text_file_false<C: Connection>() {
    let expected_session_id = ExpectedSessionId::default();
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
        fs: Some(AssertWriteFs::new(
            "/tmp/test_acp_write.txt",
            "test-write-content-67890",
        )),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let output = session.prompt(prompt, PermissionDecision::AllowOnce).await;
    assert!(!output.text.is_empty());
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_fs_write_text_file_true<C: Connection>() {
    let expected_session_id = ExpectedSessionId::default();
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

    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();
    let config = TestConnectionConfig {
        builtins: vec!["developer".to_string()],
        fs: Some(NoOpFs::new()),
        write_text_file: Some(Arc::new(move |req| {
            assert_eq!(req.path.to_str().unwrap(), "/tmp/test_acp_write.txt");
            assert_eq!(req.content, "test-write-content-67890");
            called_clone.store(true, Ordering::SeqCst);
            WriteTextFileResponse::new()
        })),
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let output = session.prompt(prompt, PermissionDecision::AllowOnce).await;
    assert!(!output.text.is_empty());
    assert!(
        called.load(Ordering::SeqCst),
        "write handler was not called"
    );
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_initialize_without_provider() {
    let temp_dir = tempfile::tempdir().unwrap();

    let provider_factory: ProviderConstructor =
        Arc::new(|_, _| Box::pin(async { Err(anyhow::anyhow!("no provider configured")) }));

    let agent = Arc::new(
        GooseAcpAgent::new(
            provider_factory,
            vec![],
            Arc::new(LocalFs),
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
            GooseMode::Auto,
            false,
        )
        .await
        .unwrap(),
    );

    let resp = initialize_agent(agent).await;
    assert!(!resp.auth_methods.is_empty());
    assert!(resp
        .auth_methods
        .iter()
        .any(|m| &*m.id.0 == "goose-provider"));
}

pub async fn run_load_model<C: Connection>() {
    let expected_session_id = ExpectedSessionId::default();
    let openai = OpenAiFixture::new(
        vec![(
            r#""model":"o4-mini""#.into(),
            include_str!("../test_data/openai_basic.txt"),
        )],
        expected_session_id.clone(),
    )
    .await;

    let mut conn = C::new(TestConnectionConfig::default(), openai).await;
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    session.set_model("o4-mini").await;

    let output = session
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");

    let session_id = session.session_id().0.to_string();
    let (_, models) = conn.load_session(&session_id).await;
    assert_eq!(&*models.unwrap().current_model_id.0, "o4-mini");
}

pub async fn run_model_list<C: Connection>() {
    let expected_session_id = ExpectedSessionId::default();
    let openai = OpenAiFixture::new(vec![], expected_session_id.clone()).await;

    let mut conn = C::new(TestConnectionConfig::default(), openai).await;
    let (session, models) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let models = models.unwrap();
    assert!(!models.available_models.is_empty());
    assert_eq!(models.current_model_id, ModelId::new(TEST_MODEL));
}

pub async fn run_model_set<C: Connection>() {
    let expected_session_id = ExpectedSessionId::default();
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
    let (mut session_a, _) = conn.new_session().await;

    // Session B: switch to o4-mini
    let (mut session_b, _) = conn.new_session().await;
    session_b.set_model("o4-mini").await;

    // Prompt B — expects o4-mini
    expected_session_id.set(session_b.session_id().0.to_string());
    let output = session_b
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");

    // Prompt A — expects default TEST_MODEL (proves sessions are independent)
    expected_session_id.set(session_a.session_id().0.to_string());
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

    let config = TestConnectionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        goose_mode: GooseMode::Approve,
        data_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let mut conn = C::new(config, openai).await;
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

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
    let expected_session_id = ExpectedSessionId::default();
    let openai = OpenAiFixture::new(
        vec![(
            r#"</info-msg>\nwhat is 1+1""#.into(),
            include_str!("../test_data/openai_basic.txt"),
        )],
        expected_session_id.clone(),
    )
    .await;

    let mut conn = C::new(TestConnectionConfig::default(), openai).await;
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let output = session
        .prompt("what is 1+1", PermissionDecision::Cancel)
        .await;
    assert_eq!(output.text, "2");
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_prompt_codemode<C: Connection>() {
    let expected_session_id = ExpectedSessionId::default();
    let prompt =
        "Search for getCode and write tools. Use them to save the code to /tmp/result.txt.";
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
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let output = session.prompt(prompt, PermissionDecision::Cancel).await;
    if matches!(output.tool_status, Some(ToolCallStatus::Failed)) || output.text.contains("error") {
        panic!("{}", output.text);
    }

    let result = fs::read_to_string("/tmp/result.txt").unwrap_or_default();
    assert_eq!(result, FAKE_CODE);
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_prompt_image<C: Connection>() {
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

    let config = TestConnectionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let output = session
        .prompt(
            "Use the get_image tool and describe what you see in its result.",
            PermissionDecision::Cancel,
        )
        .await;
    assert_eq!(output.text, "Hello Goose!\nThis is a test image.");
    expected_session_id.assert_matches(&session.session_id().0);
}

pub async fn run_prompt_mcp<C: Connection>() {
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

    let config = TestConnectionConfig {
        mcp_servers: vec![McpServer::Http(McpServerHttp::new("mcp-fixture", &mcp.url))],
        ..Default::default()
    };
    let mut conn = C::new(config, openai).await;
    let (mut session, _) = conn.new_session().await;
    expected_session_id.set(session.session_id().0.to_string());

    let output = session
        .prompt(
            "Use the get_code tool and output only its result.",
            PermissionDecision::Cancel,
        )
        .await;
    assert_eq!(output.text, FAKE_CODE);
    expected_session_id.assert_matches(&session.session_id().0);
}
