use super::{
    map_permission_response, spawn_acp_server_in_process, PermissionDecision, PermissionMapping,
    Session, TestOutput, TestSessionConfig,
};
use async_trait::async_trait;
use goose::config::PermissionManager;
use sacp::schema::{
    ContentBlock, InitializeRequest, NewSessionRequest, NewSessionResponse, PromptRequest,
    ProtocolVersion, RequestPermissionRequest, SessionModelState, SessionNotification,
    SessionUpdate, StopReason, TextContent, ToolCallStatus,
};
use sacp::{ClientToAgent, JrConnectionCx};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

pub struct ClientToAgentSession {
    cx: JrConnectionCx<ClientToAgent>,
    session_id: sacp::schema::SessionId,
    new_session_response: NewSessionResponse,
    updates: Arc<Mutex<Vec<SessionNotification>>>,
    permission: Arc<Mutex<PermissionDecision>>,
    notify: Arc<Notify>,
    permission_manager: Arc<PermissionManager>,
    // Keep the OpenAI mock server alive for the lifetime of the session.
    _openai: super::OpenAiFixture,
    // Keep the temp dir alive so test data/permissions persist during the session.
    _temp_dir: Option<tempfile::TempDir>,
}

#[async_trait]
impl Session for ClientToAgentSession {
    async fn new(config: TestSessionConfig, openai: super::OpenAiFixture) -> Self {
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
        )
        .await;

        let updates = Arc::new(Mutex::new(Vec::new()));
        let notify = Arc::new(Notify::new());
        let permission = Arc::new(Mutex::new(PermissionDecision::Cancel));

        let (cx, session_id, new_session_response) = {
            let updates_clone = updates.clone();
            let notify_clone = notify.clone();
            let permission_clone = permission.clone();
            let mcp_servers_clone = config.mcp_servers.clone();

            let cx_holder: Arc<Mutex<Option<JrConnectionCx<ClientToAgent>>>> =
                Arc::new(Mutex::new(None));
            let session_id_holder: Arc<Mutex<Option<sacp::schema::SessionId>>> =
                Arc::new(Mutex::new(None));
            let response_holder: Arc<Mutex<Option<NewSessionResponse>>> =
                Arc::new(Mutex::new(None));

            let cx_holder_clone = cx_holder.clone();
            let session_id_holder_clone = session_id_holder.clone();
            let response_holder_clone = response_holder.clone();

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
                    .connect_to(transport)
                    .unwrap()
                    .run_until({
                        let mcp_servers = mcp_servers_clone;
                        let cx_holder = cx_holder_clone;
                        let session_id_holder = session_id_holder_clone;
                        move |cx: JrConnectionCx<ClientToAgent>| async move {
                            cx.send_request(InitializeRequest::new(ProtocolVersion::LATEST))
                                .block_task()
                                .await
                                .unwrap();

                            let work_dir = tempfile::tempdir().unwrap();
                            let response = cx
                                .send_request(
                                    NewSessionRequest::new(work_dir.path())
                                        .mcp_servers(mcp_servers),
                                )
                                .block_task()
                                .await
                                .unwrap();

                            *cx_holder.lock().unwrap() = Some(cx.clone());
                            *session_id_holder.lock().unwrap() = Some(response.session_id.clone());
                            *response_holder_clone.lock().unwrap() = Some(response);
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
            let session_id = session_id_holder.lock().unwrap().take().unwrap();
            let new_session_response = response_holder.lock().unwrap().take().unwrap();
            (cx, session_id, new_session_response)
        };

        Self {
            cx,
            session_id,
            new_session_response,
            updates,
            permission,
            notify,
            permission_manager,
            _openai: openai,
            _temp_dir: temp_dir,
        }
    }

    fn id(&self) -> &sacp::schema::SessionId {
        &self.session_id
    }

    fn models(&self) -> Option<&SessionModelState> {
        self.new_session_response.models.as_ref()
    }

    fn reset_openai(&self) {
        self._openai.reset();
    }

    fn reset_permissions(&self) {
        self.permission_manager.remove_extension("");
    }

    async fn prompt(&mut self, text: &str, decision: PermissionDecision) -> TestOutput {
        *self.permission.lock().unwrap() = decision;
        self.updates.lock().unwrap().clear();

        let response = self
            .cx
            .send_request(PromptRequest::new(
                self.id().clone(),
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
