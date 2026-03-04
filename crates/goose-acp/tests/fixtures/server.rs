use super::{
    map_permission_response, spawn_acp_server_in_process, Connection, PermissionDecision,
    PermissionMapping, Session, TestConnectionConfig, TestOutput,
};
use async_trait::async_trait;
use goose::config::PermissionManager;
use sacp::schema::{
    ClientCapabilities, ContentBlock, FileSystemCapability, InitializeRequest, LoadSessionRequest,
    McpServer, NewSessionRequest, PromptRequest, ProtocolVersion, ReadTextFileRequest,
    ReadTextFileResponse, RequestPermissionRequest, SessionModelState, SessionNotification,
    SessionUpdate, StopReason, TextContent, ToolCallStatus, WriteTextFileRequest,
    WriteTextFileResponse,
};
use sacp::{ClientToAgent, JrConnectionCx};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

pub struct ClientToAgentConnection {
    cx: JrConnectionCx<ClientToAgent>,
    // MCP servers from config, consumed by the first new_session call.
    pending_mcp_servers: Vec<McpServer>,
    updates: Arc<Mutex<Vec<SessionNotification>>>,
    permission: Arc<Mutex<PermissionDecision>>,
    permission_requests: Arc<Mutex<Vec<RequestPermissionRequest>>>,
    read_requests: Arc<Mutex<Vec<ReadTextFileRequest>>>,
    write_requests: Arc<Mutex<Vec<WriteTextFileRequest>>>,
    notify: Arc<Notify>,
    permission_manager: Arc<PermissionManager>,
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

impl ClientToAgentConnection {
    #[allow(dead_code)]
    pub fn cx(&self) -> &JrConnectionCx<ClientToAgent> {
        &self.cx
    }

    pub fn permission_request_count(&self) -> usize {
        self.permission_requests.lock().unwrap().len()
    }

    pub fn read_requests(&self) -> Vec<ReadTextFileRequest> {
        self.read_requests.lock().unwrap().clone()
    }

    pub fn write_requests(&self) -> Vec<WriteTextFileRequest> {
        self.write_requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl Connection for ClientToAgentConnection {
    type Session = ClientToAgentSession;

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
        let permission_requests = Arc::new(Mutex::new(Vec::new()));
        let read_requests = Arc::new(Mutex::new(Vec::new()));
        let write_requests = Arc::new(Mutex::new(Vec::new()));
        let fs_read_text_file = config.fs_read_text_file;
        let fs_write_text_file = config.fs_write_text_file;

        let cx = {
            let updates_clone = updates.clone();
            let notify_clone = notify.clone();
            let permission_clone = permission.clone();
            let permission_requests_clone = permission_requests.clone();
            let read_requests_clone = read_requests.clone();
            let write_requests_clone = write_requests.clone();

            let cx_holder: Arc<Mutex<Option<JrConnectionCx<ClientToAgent>>>> =
                Arc::new(Mutex::new(None));
            let cx_holder_clone = cx_holder.clone();

            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

            tokio::spawn(async move {
                let permission_mapping = PermissionMapping;

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
                            let permission_requests = permission_requests_clone.clone();
                            async move |req: RequestPermissionRequest,
                                        request_cx,
                                        _connection_cx| {
                                permission_requests.lock().unwrap().push(req.clone());
                                let decision = *permission.lock().unwrap();
                                let response =
                                    map_permission_response(&permission_mapping, &req, decision);
                                request_cx.respond(response)
                            }
                        },
                        sacp::on_receive_request!(),
                    )
                    .on_receive_request(
                        {
                            let read_requests = read_requests_clone.clone();
                            async move |req: ReadTextFileRequest, request_cx, _connection_cx| {
                                read_requests.lock().unwrap().push(req.clone());
                                let content = std::fs::read_to_string(&req.path).map_err(|e| {
                                    sacp::Error::internal_error().data(format!(
                                        "Failed to read {}: {}",
                                        req.path.display(),
                                        e
                                    ))
                                })?;
                                request_cx.respond(ReadTextFileResponse::new(content))
                            }
                        },
                        sacp::on_receive_request!(),
                    )
                    .on_receive_request(
                        {
                            let write_requests = write_requests_clone.clone();
                            async move |req: WriteTextFileRequest, request_cx, _connection_cx| {
                                write_requests.lock().unwrap().push(req.clone());
                                if let Some(parent) = req.path.parent() {
                                    if !parent.as_os_str().is_empty() {
                                        std::fs::create_dir_all(parent).map_err(|e| {
                                            sacp::Error::internal_error().data(format!(
                                                "Failed to create {}: {}",
                                                parent.display(),
                                                e
                                            ))
                                        })?;
                                    }
                                }
                                std::fs::write(&req.path, &req.content).map_err(|e| {
                                    sacp::Error::internal_error().data(format!(
                                        "Failed to write {}: {}",
                                        req.path.display(),
                                        e
                                    ))
                                })?;
                                request_cx.respond(WriteTextFileResponse::new())
                            }
                        },
                        sacp::on_receive_request!(),
                    )
                    .connect_to(transport)
                    .unwrap()
                    .run_until({
                        let cx_holder = cx_holder_clone;
                        move |cx: JrConnectionCx<ClientToAgent>| async move {
                            cx.send_request(
                                InitializeRequest::new(ProtocolVersion::LATEST)
                                    .client_capabilities(
                                        ClientCapabilities::new().fs(FileSystemCapability::new()
                                            .read_text_file(fs_read_text_file)
                                            .write_text_file(fs_write_text_file)),
                                    ),
                            )
                            .block_task()
                            .await
                            .unwrap();

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
            cx
        };

        Self {
            cx,
            pending_mcp_servers: config.mcp_servers,
            updates,
            permission,
            permission_requests,
            read_requests,
            write_requests,
            notify,
            permission_manager,
            _openai: openai,
            _temp_dir: temp_dir,
        }
    }

    async fn new_session(&mut self) -> (ClientToAgentSession, Option<SessionModelState>) {
        let work_dir = tempfile::tempdir().unwrap();
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
        (session, response.models)
    }

    async fn load_session(
        &mut self,
        session_id: &str,
    ) -> (ClientToAgentSession, Option<SessionModelState>) {
        self.updates.lock().unwrap().clear();
        let work_dir = tempfile::tempdir().unwrap();
        let session_id = sacp::schema::SessionId::new(session_id.to_string());
        let response = self
            .cx
            .send_request(LoadSessionRequest::new(session_id.clone(), work_dir.path()))
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
        (session, response.models)
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
        *self.permission.lock().unwrap() = decision;
        self.updates.lock().unwrap().clear();

        let response = self
            .cx
            .send_request(PromptRequest::new(
                self.session_id.clone(),
                vec![ContentBlock::Text(TextContent::new(text))],
            ))
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

    // HACK: sacp doesn't support session/set_model yet, so we send it as untyped JSON.
    async fn set_model(&self, model_id: &str) {
        let msg = sacp::UntypedMessage::new(
            "session/set_model",
            serde_json::json!({
                "sessionId": self.session_id.0,
                "modelId": model_id
            }),
        )
        .unwrap();
        self.cx.send_request(msg).block_task().await.unwrap();
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
