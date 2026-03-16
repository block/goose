use super::{
    spawn_acp_server_in_process, Connection, OpenAiFixture, PermissionDecision, Session,
    SessionResult, TestConnectionConfig, TestOutput,
};
use async_trait::async_trait;
use futures::StreamExt;
use goose::acp::{AcpProvider, AcpProviderConfig, PermissionMapping};
use goose::config::goose_mode::GooseMode;
use goose::config::PermissionManager;
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::model::ModelConfig;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};
use goose::providers::base::Provider;
use goose::providers::errors::ProviderError;
use goose_test_support::{ExpectedSessionId, IgnoreSessionId, TEST_MODEL};
use sacp::schema::{AuthMethod, McpServer, SessionUpdate, ToolCallStatus};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type NotificationSink = Arc<std::sync::Mutex<Vec<SessionUpdate>>>;

#[allow(dead_code)]
pub struct ClientToProviderConnection {
    provider: Arc<Mutex<AcpProvider>>,
    permission_manager: Arc<PermissionManager>,
    auth_methods: Vec<AuthMethod>,
    session_counter: usize,
    notification_sink: NotificationSink,
    _openai: OpenAiFixture,
    _temp_dir: Option<tempfile::TempDir>,
    _cwd: Option<tempfile::TempDir>,
}

#[allow(dead_code)]
pub struct ClientToProviderSession {
    provider: Arc<Mutex<AcpProvider>>,
    session_id: sacp::schema::SessionId,
    notification_sink: NotificationSink,
}

impl ClientToProviderSession {
    #[allow(dead_code)]
    async fn send_message(&mut self, message: Message, decision: PermissionDecision) -> TestOutput {
        let session_id = self.session_id.0.clone();
        let provider = self.provider.lock().await;
        self.notification_sink.lock().unwrap().clear();
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

    fn expected_session_id() -> Arc<dyn ExpectedSessionId> {
        Arc::new(IgnoreSessionId)
    }

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

        let cwd_path = config
            .cwd
            .as_ref()
            .map(|td| td.path().to_path_buf())
            .unwrap_or(data_root);

        let notification_sink: NotificationSink = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink_clone = notification_sink.clone();
        let provider_config = AcpProviderConfig {
            command: "unused".into(),
            args: vec![],
            env: vec![],
            env_remove: vec![],
            work_dir: cwd_path,
            mcp_servers,
            session_mode_id: None,
            permission_mapping: PermissionMapping::default(),
            notification_callback: Some(Arc::new(move |n| {
                sink_clone.lock().unwrap().push(n.update.clone());
            })),
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
            notification_sink,
            _openai: openai,
            _temp_dir: temp_dir,
            _cwd: config.cwd,
        }
    }

    async fn new_session(&mut self) -> SessionResult<ClientToProviderSession> {
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
            session_id: sacp::schema::SessionId::new(goose_id),
            notification_sink: self.notification_sink.clone(),
        };
        SessionResult {
            session,
            models: response.models,
            modes: response.modes,
        }
    }

    async fn load_session(
        &mut self,
        _session_id: &str,
        _mcp_servers: Vec<McpServer>,
    ) -> SessionResult<ClientToProviderSession> {
        unimplemented!("TODO: implement load_session in ACP provider")
    }

    async fn set_mode(&self, session_id: &str, mode_id: &str) -> anyhow::Result<()> {
        let mode = GooseMode::from_str(mode_id).map_err(|_| {
            sacp::Error::invalid_params().data(format!("Invalid mode: {}", mode_id))
        })?;
        self.provider
            .lock()
            .await
            .update_mode(session_id, mode)
            .await
            .map_err(|e| match e {
                ProviderError::RequestFailed(msg) => sacp::Error::invalid_params().data(msg),
                other => sacp::Error::internal_error().data(other.to_string()),
            })?;
        Ok(())
    }

    async fn set_model(&self, session_id: &str, model_id: &str) -> anyhow::Result<()> {
        let provider = self.provider.lock().await;
        let response = provider.ensure_session(Some(session_id)).await?;
        provider
            .send_untyped(
                "session/set_model",
                serde_json::json!({ "sessionId": response.session_id, "modelId": model_id }),
            )
            .await?;
        Ok(())
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

    fn notifications(&self) -> Vec<super::Notification> {
        let updates = self.notification_sink.lock().unwrap();
        super::to_notifications(&updates)
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
}
