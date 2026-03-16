use super::{
    map_permission_response, spawn_acp_server_in_process, Connection, PermissionDecision,
    PermissionMapping, Session, SessionResult, TestConnectionConfig, TestOutput,
};
use async_trait::async_trait;
use goose::config::PermissionManager;
use goose_test_support::{EnforceSessionId, ExpectedSessionId};
use sacp::schema::{
    AuthMethod, ClientCapabilities, ContentBlock, CreateTerminalRequest, FileSystemCapability,
    ImageContent, InitializeRequest, KillTerminalCommandRequest, LoadSessionRequest, McpServer,
    NewSessionRequest, PromptRequest, ProtocolVersion, ReadTextFileRequest, ReleaseTerminalRequest,
    RequestPermissionRequest, SessionNotification, SessionUpdate, StopReason,
    TerminalOutputRequest, TextContent, ToolCallStatus, WaitForTerminalExitRequest,
    WriteTextFileRequest,
};
use sacp::{ClientToAgent, JrConnectionCx};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

pub struct ClientToAgentConnection {
    cx: JrConnectionCx<ClientToAgent>,
    // MCP servers from config, consumed by the first new_session call.
    pending_mcp_servers: Vec<McpServer>,
    cwd: Option<tempfile::TempDir>,
    updates: Arc<Mutex<Vec<SessionNotification>>>,
    permission: Arc<Mutex<PermissionDecision>>,
    notify: Arc<Notify>,
    permission_manager: Arc<PermissionManager>,
    auth_methods: Vec<AuthMethod>,
    _openai: super::OpenAiFixture,
    _temp_dir: Option<tempfile::TempDir>,
}

pub struct ClientToAgentSession {
    cx: JrConnectionCx<ClientToAgent>,
    session_id: sacp::schema::SessionId,
    updates: Arc<Mutex<Vec<SessionNotification>>>,
    permission: Arc<Mutex<PermissionDecision>>,
    notify: Arc<Notify>,
}

impl ClientToAgentSession {
    async fn send_prompt(
        &mut self,
        content: Vec<ContentBlock>,
        decision: PermissionDecision,
    ) -> TestOutput {
        *self.permission.lock().unwrap() = decision;
        self.updates.lock().unwrap().clear();

        let response = self
            .cx
            .send_request(PromptRequest::new(self.session_id.clone(), content))
            .block_task()
            .await
            .unwrap();

        assert_eq!(response.stop_reason, StopReason::EndTurn);

        let mut updates_len = self.updates.lock().unwrap().len();
        while updates_len == 0 {
            self.notify.notified().await;
            updates_len = self.updates.lock().unwrap().len();
        }

        let text = collect_agent_text(&self.updates);
        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        let mut tool_status = extract_tool_status(&self.updates);
        while tool_status.is_none() && tokio::time::Instant::now() < deadline {
            tokio::task::yield_now().await;
            tool_status = extract_tool_status(&self.updates);
        }

        TestOutput { text, tool_status }
    }
}

impl ClientToAgentConnection {
    #[allow(dead_code)]
    pub fn cx(&self) -> &JrConnectionCx<ClientToAgent> {
        &self.cx
    }
}

#[async_trait]
impl Connection for ClientToAgentConnection {
    type Session = ClientToAgentSession;

    fn expected_session_id() -> Arc<dyn ExpectedSessionId> {
        Arc::new(EnforceSessionId::default())
    }

    async fn new(config: TestConnectionConfig, openai: super::OpenAiFixture) -> Self {
        let (data_root, temp_dir) = match config.data_root.as_os_str().is_empty() {
            true => {
                let temp_dir = tempfile::tempdir().unwrap();
                (temp_dir.path().to_path_buf(), Some(temp_dir))
            }
            false => (config.data_root.clone(), None),
        };

        let (transport, _handle, permission_manager) = spawn_acp_server_in_process(
            openai.uri(),
            &config.builtins,
            data_root.as_path(),
            config.goose_mode,
            config.provider_factory,
        )
        .await;

        let updates = Arc::new(Mutex::new(Vec::new()));
        let notify = Arc::new(Notify::new());
        let permission = Arc::new(Mutex::new(PermissionDecision::Cancel));

        let mut fs_cap = FileSystemCapability::default();
        if config.read_text_file.is_some() {
            fs_cap = fs_cap.read_text_file(true);
        }
        if config.write_text_file.is_some() {
            fs_cap = fs_cap.write_text_file(true);
        }

        let (cx, auth_methods) = {
            let updates_clone = updates.clone();
            let notify_clone = notify.clone();
            let permission_clone = permission.clone();
            let read_handler = config.read_text_file;
            let write_handler = config.write_text_file;
            let terminal = config.terminal;

            let cx_holder: Arc<Mutex<Option<JrConnectionCx<ClientToAgent>>>> =
                Arc::new(Mutex::new(None));
            let cx_holder_clone = cx_holder.clone();
            let auth_holder: Arc<Mutex<Vec<AuthMethod>>> = Arc::new(Mutex::new(Vec::new()));
            let auth_holder_clone = auth_holder.clone();

            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

            tokio::spawn(async move {
                let permission_mapping = PermissionMapping::default();

                let result = ClientToAgent::builder()
                    .on_receive_notification(
                        {
                            let updates = updates_clone.clone();
                            let notify = notify_clone.clone();
                            async move |notification: SessionNotification, _cx| {
                                updates.lock().unwrap().push(notification);
                                notify.notify_waiters();
                                Ok(())
                            }
                        },
                        sacp::on_receive_notification!(),
                    )
                    .on_receive_request(
                        {
                            let permission = permission_clone.clone();
                            async move |req: RequestPermissionRequest,
                                        request_cx,
                                        _connection_cx| {
                                let decision = *permission.lock().unwrap();
                                let response =
                                    map_permission_response(&permission_mapping, &req, decision);
                                request_cx.respond(response)
                            }
                        },
                        sacp::on_receive_request!(),
                    )
                    .on_receive_request(
                        async move |req: ReadTextFileRequest, request_cx, _cx| match read_handler {
                            Some(ref rh) => match rh(&req) {
                                Ok(resp) => request_cx.respond(resp),
                                Err(msg) => request_cx.respond_with_internal_error(msg),
                            },
                            None => request_cx.respond_with_error(sacp::Error::method_not_found()),
                        },
                        sacp::on_receive_request!(),
                    )
                    .on_receive_request(
                        async move |req: WriteTextFileRequest, request_cx, _cx| match write_handler
                        {
                            Some(ref wh) => match wh(&req) {
                                Ok(resp) => request_cx.respond(resp),
                                Err(msg) => request_cx.respond_with_internal_error(msg),
                            },
                            None => request_cx.respond_with_error(sacp::Error::method_not_found()),
                        },
                        sacp::on_receive_request!(),
                    )
                    .on_receive_request(
                        {
                            let t = terminal.clone();
                            async move |req: CreateTerminalRequest, request_cx, _cx| match t {
                                Some(ref f) => {
                                    if req.command == f.command {
                                        f.record_call(Ok(()));
                                    } else {
                                        f.record_call(Err(format!(
                                            "expected command {}, got {}",
                                            f.command, req.command
                                        )));
                                    }
                                    request_cx.respond(f.create_response())
                                }
                                None => {
                                    request_cx.respond_with_error(sacp::Error::method_not_found())
                                }
                            }
                        },
                        sacp::on_receive_request!(),
                    )
                    .on_receive_request(
                        {
                            let t = terminal.clone();
                            async move |_req: WaitForTerminalExitRequest, request_cx, _cx| match t {
                                Some(ref f) => request_cx.respond(f.wait_response()),
                                None => {
                                    request_cx.respond_with_error(sacp::Error::method_not_found())
                                }
                            }
                        },
                        sacp::on_receive_request!(),
                    )
                    .on_receive_request(
                        {
                            let t = terminal.clone();
                            async move |_req: TerminalOutputRequest, request_cx, _cx| match t {
                                Some(ref f) => request_cx.respond(f.output_response()),
                                None => {
                                    request_cx.respond_with_error(sacp::Error::method_not_found())
                                }
                            }
                        },
                        sacp::on_receive_request!(),
                    )
                    .on_receive_request(
                        {
                            let t = terminal.clone();
                            async move |_req: ReleaseTerminalRequest, request_cx, _cx| match t {
                                Some(ref f) => request_cx.respond(f.release_response()),
                                None => {
                                    request_cx.respond_with_error(sacp::Error::method_not_found())
                                }
                            }
                        },
                        sacp::on_receive_request!(),
                    )
                    .on_receive_request(
                        {
                            let t = terminal.clone();
                            async move |_req: KillTerminalCommandRequest, request_cx, _cx| match t {
                                Some(ref f) => request_cx.respond(f.kill_response()),
                                None => {
                                    request_cx.respond_with_error(sacp::Error::method_not_found())
                                }
                            }
                        },
                        sacp::on_receive_request!(),
                    )
                    .connect_to(transport)
                    .unwrap()
                    .run_until({
                        let cx_holder = cx_holder_clone;
                        let auth_holder = auth_holder_clone;
                        move |cx: JrConnectionCx<ClientToAgent>| async move {
                            let resp = cx
                                .send_request(
                                    InitializeRequest::new(ProtocolVersion::LATEST)
                                        .client_capabilities(
                                            ClientCapabilities::new()
                                                .fs(fs_cap)
                                                .terminal(terminal.is_some()),
                                        ),
                                )
                                .block_task()
                                .await
                                .unwrap();

                            *auth_holder.lock().unwrap() = resp.auth_methods;
                            *cx_holder.lock().unwrap() = Some(cx.clone());
                            let _ = ready_tx.send(());

                            std::future::pending::<Result<(), sacp::Error>>().await
                        }
                    })
                    .await;

                if let Err(e) = result {
                    tracing::error!("SACP client error: {e}");
                }
            });

            ready_rx.await.unwrap();
            let cx = cx_holder.lock().unwrap().take().unwrap();
            let auth = std::mem::take(&mut *auth_holder.lock().unwrap());
            (cx, auth)
        };

        Self {
            cx,
            pending_mcp_servers: config.mcp_servers,
            cwd: config.cwd,
            updates,
            permission,
            notify,
            permission_manager,
            auth_methods,
            _openai: openai,
            _temp_dir: temp_dir,
        }
    }

    async fn new_session(&mut self) -> SessionResult<ClientToAgentSession> {
        let work_dir = self
            .cwd
            .take()
            .unwrap_or_else(|| tempfile::tempdir().unwrap());
        let mcp_servers = std::mem::take(&mut self.pending_mcp_servers);
        let response = self
            .cx
            .send_request(NewSessionRequest::new(work_dir.path()).mcp_servers(mcp_servers))
            .block_task()
            .await
            .unwrap();
        let session = ClientToAgentSession {
            cx: self.cx.clone(),
            session_id: response.session_id.clone(),
            updates: self.updates.clone(),
            permission: self.permission.clone(),
            notify: self.notify.clone(),
        };
        SessionResult {
            session,
            models: response.models,
            modes: response.modes,
        }
    }

    async fn load_session(
        &mut self,
        session_id: &str,
        mcp_servers: Vec<McpServer>,
    ) -> SessionResult<ClientToAgentSession> {
        self.updates.lock().unwrap().clear();
        let work_dir = tempfile::tempdir().unwrap();
        let session_id = sacp::schema::SessionId::new(session_id.to_string());
        let response = self
            .cx
            .send_request(
                LoadSessionRequest::new(session_id.clone(), work_dir.path())
                    .mcp_servers(mcp_servers),
            )
            .block_task()
            .await
            .unwrap();
        let session = ClientToAgentSession {
            cx: self.cx.clone(),
            session_id,
            updates: self.updates.clone(),
            permission: self.permission.clone(),
            notify: self.notify.clone(),
        };
        SessionResult {
            session,
            models: response.models,
            modes: response.modes,
        }
    }

    async fn set_mode(&self, session_id: &str, mode_id: &str) -> anyhow::Result<()> {
        let msg = sacp::UntypedMessage::new(
            "session/set_mode",
            serde_json::json!({ "sessionId": session_id, "modeId": mode_id }),
        )?;
        self.cx
            .send_request(msg)
            .block_task()
            .await
            .map(|_| ())
            .map_err(|e| e.into())
    }

    async fn set_model(&self, session_id: &str, model_id: &str) -> anyhow::Result<()> {
        let msg = sacp::UntypedMessage::new(
            "session/set_model",
            serde_json::json!({ "sessionId": session_id, "modelId": model_id }),
        )?;
        self.cx
            .send_request(msg)
            .block_task()
            .await
            .map(|_| ())
            .map_err(|e| e.into())
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
impl Session for ClientToAgentSession {
    fn session_id(&self) -> &sacp::schema::SessionId {
        &self.session_id
    }

    async fn prompt(&mut self, text: &str, decision: PermissionDecision) -> TestOutput {
        self.send_prompt(vec![ContentBlock::Text(TextContent::new(text))], decision)
            .await
    }

    async fn prompt_with_image(
        &mut self,
        text: &str,
        image_b64: &str,
        mime_type: &str,
        decision: PermissionDecision,
    ) -> TestOutput {
        self.send_prompt(
            vec![
                ContentBlock::Image(ImageContent::new(image_b64, mime_type)),
                ContentBlock::Text(TextContent::new(text)),
            ],
            decision,
        )
        .await
    }
}

fn collect_agent_text(updates: &Arc<Mutex<Vec<SessionNotification>>>) -> String {
    let guard = updates.lock().unwrap();
    let mut text = String::new();

    for notification in guard.iter() {
        if let SessionUpdate::AgentMessageChunk(chunk) = &notification.update {
            if let ContentBlock::Text(t) = &chunk.content {
                text.push_str(&t.text);
            }
        }
    }

    text
}

fn extract_tool_status(updates: &Arc<Mutex<Vec<SessionNotification>>>) -> Option<ToolCallStatus> {
    let guard = updates.lock().unwrap();
    guard.iter().find_map(|notification| {
        if let SessionUpdate::ToolCallUpdate(update) = &notification.update {
            return update.fields.status;
        }
        None
    })
}
