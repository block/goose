mod common;

use async_trait::async_trait;
use common::{
    run_basic_completion, run_builtin_and_mcp, run_mcp_http_server, run_permission_persistence,
    run_test, spawn_acp_server_in_process, OpenAiFixture, Session, TestOutput, TestSessionConfig,
};
use futures::StreamExt;
use goose::config::PermissionManager;
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::model::ModelConfig;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};
use goose::providers::acp_agent::AcpProviderCore;
use goose_acp_provider::{
    AcpClient, AcpProviderConfig, AcpSessionConfig, PermissionDecision, PermissionMapping,
};
use sacp::schema::ToolCallStatus;
use std::sync::Arc;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

struct TestSession {
    provider: AcpProviderCore,
    session_id: sacp::schema::SessionId,
    permission_manager: Arc<PermissionManager>,
    // Keep the OpenAI mock server alive for the lifetime of the session.
    _openai: OpenAiFixture,
    // Keep the temp dir alive so test data/permissions persist during the session.
    _temp_dir: Option<tempfile::TempDir>,
}

#[async_trait]
impl Session for TestSession {
    async fn new(config: TestSessionConfig, openai: OpenAiFixture) -> Self {
        let (data_root, temp_dir) = match config.data_root.as_os_str().is_empty() {
            true => {
                let temp_dir = tempfile::tempdir().unwrap();
                (temp_dir.path().to_path_buf(), Some(temp_dir))
            }
            false => (config.data_root.clone(), None),
        };

        let (client_read, client_write, _handle, permission_manager) = spawn_acp_server_in_process(
            openai.uri(),
            &config.builtins,
            data_root.as_path(),
            config.goose_mode,
        )
        .await;

        let client_config = AcpProviderConfig {
            session: AcpSessionConfig {
                work_dir: data_root,
                mcp_servers: config.mcp_servers,
            },
            command: "unused".into(),
            args: vec![],
            env: vec![],
            session_mode_id: None,
            permission_mapping: PermissionMapping::default(),
        };

        let client = AcpClient::connect_with_transport(
            client_config,
            client_read.compat(),
            client_write.compat_write(),
        )
        .await
        .unwrap();

        let provider = AcpProviderCore::with_client(
            "acp-test".to_string(),
            ModelConfig::new("default").unwrap(),
            Arc::new(client),
            config.goose_mode,
        );

        let session_id = provider
            .new_session()
            .await
            .expect("missing ACP session_id");

        Self {
            provider,
            session_id,
            permission_manager,
            _openai: openai,
            _temp_dir: temp_dir,
        }
    }

    fn id(&self) -> &sacp::schema::SessionId {
        &self.session_id
    }

    fn reset_openai(&self) {
        self._openai.reset();
    }

    fn reset_permissions(&self) {
        self.permission_manager.remove_extension("");
    }

    async fn prompt(&mut self, prompt: &str, decision: PermissionDecision) -> TestOutput {
        let message = Message::user().with_text(prompt);
        let session_id = self.id().0.to_string();
        let mut stream = self
            .provider
            .stream(&session_id, "", &[message], &[])
            .await
            .unwrap();
        let mut text = String::new();
        let mut tool_error = false;
        let mut saw_tool = false;

        while let Some(item) = stream.next().await {
            let (msg, _) = item.unwrap();
            if let Some(msg) = msg {
                for content in msg.content {
                    match content {
                        MessageContent::Text(t) => {
                            text.push_str(&t.text);
                        }
                        MessageContent::ToolResponse(resp) => {
                            saw_tool = true;
                            if let Ok(result) = resp.tool_result {
                                tool_error |= result.is_error.unwrap_or(false);
                            }
                        }
                        MessageContent::ActionRequired(action) => {
                            if let ActionRequiredData::ToolConfirmation { id, .. } = action.data {
                                saw_tool = true;
                                if matches!(
                                    decision,
                                    PermissionDecision::RejectAlways
                                        | PermissionDecision::RejectOnce
                                        | PermissionDecision::Cancel
                                ) {
                                    tool_error = true;
                                }

                                let permission = match decision {
                                    PermissionDecision::AllowAlways => Permission::AlwaysAllow,
                                    PermissionDecision::AllowOnce => Permission::AllowOnce,
                                    PermissionDecision::RejectAlways => Permission::AlwaysDeny,
                                    PermissionDecision::RejectOnce => Permission::DenyOnce,
                                    PermissionDecision::Cancel => Permission::Cancel,
                                };

                                let confirmation = PermissionConfirmation {
                                    principal_type: PrincipalType::Tool,
                                    permission,
                                };

                                let handled = self
                                    .provider
                                    .handle_permission_confirmation(&id, &confirmation)
                                    .await;
                                assert!(handled);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let tool_status = if saw_tool {
            Some(if tool_error {
                ToolCallStatus::Failed
            } else {
                ToolCallStatus::Completed
            })
        } else {
            None
        };

        TestOutput { text, tool_status }
    }
}

#[test]
fn test_provider_basic_completion() {
    run_test(async { run_basic_completion::<TestSession>().await });
}

#[test]
fn test_provider_with_mcp_http_server() {
    run_test(async { run_mcp_http_server::<TestSession>().await });
}

#[test]
fn test_provider_with_builtin_and_mcp() {
    run_test(async { run_builtin_and_mcp::<TestSession>().await });
}

#[test]
fn test_permission_persistence() {
    run_test(async { run_permission_persistence::<TestSession>().await });
}
