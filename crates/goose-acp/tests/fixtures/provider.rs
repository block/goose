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
use sacp::schema::{AuthMethod, McpServer, SessionModelState, ToolCallStatus};
use std::sync::Arc;
use tokio::sync::Mutex;

#[allow(dead_code)]
pub struct ClientToProviderConnection {
    provider: Arc<Mutex<AcpProvider>>,
    permission_manager: Arc<PermissionManager>,
    auth_methods: Vec<AuthMethod>,
    session_counter: usize,
    _openai: OpenAiFixture,
    _temp_dir: Option<tempfile::TempDir>,
}

#[allow(dead_code)]
pub struct ClientToProviderSession {
    provider: Arc<Mutex<AcpProvider>>,
    acp_session_id: sacp::schema::SessionId,
    session_id: String,
}

impl ClientToProviderSession {
    #[allow(dead_code)]
    async fn send_message(&mut self, message: Message, decision: PermissionDecision) -> TestOutput {
        let session_id = self.session_id.clone();
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
                                tool_error |= decision.should_record_rejection();

                                let confirmation = PermissionConfirmation {
                                    principal_type: PrincipalType::Tool,
                                    permission: Permission::from(decision),
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
            session_counter: 0,
            _openai: openai,
            _temp_dir: temp_dir,
        }
    }

    async fn new_session(&mut self) -> (ClientToProviderSession, Option<SessionModelState>) {
        // Tests like run_model_set call new_session() multiple times on the same
        // connection, so each needs a distinct key to avoid returning a cached session.
        self.session_counter += 1;
        let goose_id = format!("test-session-{}", self.session_counter);
        let response = self
            .provider
            .lock()
            .await
            .ensure_session(Some(&goose_id))
            .await
            .unwrap();

        let session = ClientToProviderSession {
            provider: Arc::clone(&self.provider),
            acp_session_id: response.session_id,
            session_id: goose_id,
        };
        (session, response.models)
    }

    async fn load_session(
        &mut self,
        _session_id: &str,
        _mcp_servers: Vec<McpServer>,
    ) -> (ClientToProviderSession, Option<SessionModelState>) {
        unimplemented!("TODO: implement load_session in ACP provider")
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
        &self.acp_session_id
    }

    async fn prompt(&mut self, prompt: &str, decision: PermissionDecision) -> TestOutput {
        self.send_message(Message::user().with_text(prompt), decision)
            .await
    }

    async fn prompt_with_image(
        &mut self,
        prompt: &str,
        image_b64: &str,
        mime_type: &str,
        decision: PermissionDecision,
    ) -> TestOutput {
        let message = Message::user()
            .with_image(image_b64, mime_type)
            .with_text(prompt);
        self.send_message(message, decision).await
    }

    async fn set_model(&self, model_id: &str) {
        self.provider
            .lock()
            .await
            .set_model(&self.acp_session_id, model_id)
            .await
            .unwrap();
    }
}
