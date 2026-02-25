use super::{
    spawn_acp_server_in_process, Connection, OpenAiFixture, PermissionDecision, Session,
    TestConnectionConfig, TestOutput,
};
use async_trait::async_trait;
use futures::StreamExt;
use goose::acp::{AcpProvider, AcpProviderConfig, PermissionMapping};
use goose::config::PermissionManager;
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::model::ModelConfig;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};
use goose::providers::base::Provider;
use goose_test_support::TEST_MODEL;
use sacp::schema::{AuthMethod, SessionModelState, ToolCallStatus};
use std::sync::Arc;
use tokio::sync::Mutex;

#[allow(dead_code)]
pub struct ClientToProviderConnection {
    provider: Arc<Mutex<AcpProvider>>,
    permission_manager: Arc<PermissionManager>,
    auth_methods: Vec<AuthMethod>,
    _openai: OpenAiFixture,
    _temp_dir: Option<tempfile::TempDir>,
}

#[allow(dead_code)]
pub struct ClientToProviderSession {
    provider: Arc<Mutex<AcpProvider>>,
    session_id: sacp::schema::SessionId,
    permission: PermissionDecision,
}

#[async_trait]
impl Connection for ClientToProviderConnection {
    type Session = ClientToProviderSession;

    async fn new(config: TestConnectionConfig, openai: OpenAiFixture) -> Self {
        let (data_root, temp_dir) = match config.data_root.as_os_str().is_empty() {
            true => {
                let temp_dir = tempfile::tempdir().unwrap();
                (temp_dir.path().to_path_buf(), Some(temp_dir))
            }
            false => (config.data_root.clone(), None),
        };

        let goose_mode = config.goose_mode;
        let mcp_servers = config.mcp_servers;

        let (transport, _handle, permission_manager) = spawn_acp_server_in_process(
            openai.uri(),
            &config.builtins,
            data_root.as_path(),
            goose_mode,
            config.provider_factory,
        )
        .await;

        let provider_config = AcpProviderConfig {
            command: "unused".into(),
            args: vec![],
            env: vec![],
            env_remove: vec![],
            work_dir: data_root,
            mcp_servers,
            session_mode_id: None,
            permission_mapping: PermissionMapping::default(),
        };

        let provider = AcpProvider::connect_with_transport(
            "acp-test".to_string(),
            ModelConfig::new(TEST_MODEL).unwrap(),
            goose_mode,
            provider_config,
            transport.incoming,
            transport.outgoing,
        )
        .await
        .unwrap();

        let auth_methods = provider.auth_methods().to_vec();

        Self {
            provider: Arc::new(Mutex::new(provider)),
            permission_manager,
            auth_methods,
            _openai: openai,
            _temp_dir: temp_dir,
        }
    }

    async fn new_session(&mut self) -> (ClientToProviderSession, Option<SessionModelState>) {
        let (session_id, models) = self
            .provider
            .lock()
            .await
            .new_session()
            .await
            .expect("missing ACP session_id");

        let session = ClientToProviderSession {
            provider: Arc::clone(&self.provider),
            session_id,
            permission: PermissionDecision::Cancel,
        };
        (session, models)
    }

    async fn load_session(
        &mut self,
        _session_id: &str,
    ) -> (ClientToProviderSession, Option<SessionModelState>) {
        unimplemented!("provider sessions do not support load_session")
    }

    fn auth_methods(&self) -> &[AuthMethod] {
        &self.auth_methods
    }

    fn reset_openai(&self) {
        self._openai.reset();
    }

    fn reset_permissions(&self) {
        self.permission_manager.remove_extension("");
    }
}

#[async_trait]
impl Session for ClientToProviderSession {
    fn session_id(&self) -> &sacp::schema::SessionId {
        &self.session_id
    }

    async fn prompt(&mut self, prompt: &str, decision: PermissionDecision) -> TestOutput {
        self.permission = decision;
        let message = Message::user().with_text(prompt);
        let session_id = self.session_id.0.to_string();
        let provider = self.provider.lock().await;
        let model_config = provider.get_model_config();
        let mut stream = provider
            .stream(&model_config, &session_id, "", &[message], &[])
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
                                    self.permission,
                                    PermissionDecision::RejectAlways
                                        | PermissionDecision::RejectOnce
                                        | PermissionDecision::Cancel
                                ) {
                                    tool_error = true;
                                }

                                let permission = match self.permission {
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

                                let handled = provider
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

    async fn set_model(&self, model_id: &str) {
        self.provider
            .lock()
            .await
            .set_model(&self.session_id, model_id)
            .await
            .unwrap();
    }
}
