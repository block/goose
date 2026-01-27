mod common;

use async_trait::async_trait;
use common::{
    run_basic_completion, run_builtin_and_mcp, run_mcp_http_server, run_permission_persistence,
    run_test, spawn_acp_server_in_process, OpenAiFixture, Session, TestOutput, TestSessionConfig,
};
use goose::config::PermissionManager;
use goose_acp_provider::{map_permission_response, PermissionDecision, PermissionMapping};
use sacp::schema::{
    ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest, ProtocolVersion,
    RequestPermissionRequest, SessionNotification, SessionUpdate, StopReason, TextContent,
    ToolCallStatus,
};
use sacp::{ClientToAgent, JrConnectionCx};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

struct TestSession {
    cx: JrConnectionCx<ClientToAgent>,
    session_id: sacp::schema::SessionId,
    updates: Arc<Mutex<Vec<SessionNotification>>>,
    permission: Arc<Mutex<PermissionDecision>>,
    notify: Arc<Notify>,
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

        let updates = Arc::new(Mutex::new(Vec::new()));
        let notify = Arc::new(Notify::new());
        let permission = Arc::new(Mutex::new(PermissionDecision::Cancel));

        let transport = sacp::ByteStreams::new(client_write.compat_write(), client_read.compat());

        let (cx, session_id) = {
            let updates_clone = updates.clone();
            let notify_clone = notify.clone();
            let permission_clone = permission.clone();
            let mcp_servers_clone = config.mcp_servers.clone();

            let cx_holder: Arc<Mutex<Option<JrConnectionCx<ClientToAgent>>>> =
                Arc::new(Mutex::new(None));
            let session_id_holder: Arc<Mutex<Option<sacp::schema::SessionId>>> =
                Arc::new(Mutex::new(None));

            let cx_holder_clone = cx_holder.clone();
            let session_id_holder_clone = session_id_holder.clone();

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
                            let session = cx
                                .send_request(
                                    NewSessionRequest::new(work_dir.path())
                                        .mcp_servers(mcp_servers),
                                )
                                .block_task()
                                .await
                                .unwrap();

                            *cx_holder.lock().unwrap() = Some(cx.clone());
                            *session_id_holder.lock().unwrap() = Some(session.session_id);
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
            (cx, session_id)
        };

        Self {
            cx,
            session_id,
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

#[test]
fn test_acp_basic_completion() {
    run_test(async { run_basic_completion::<TestSession>().await });
}

#[test]
fn test_acp_with_mcp_http_server() {
    run_test(async { run_mcp_http_server::<TestSession>().await });
}

#[test]
fn test_acp_with_builtin_and_mcp() {
    run_test(async { run_builtin_and_mcp::<TestSession>().await });
}

#[test]
fn test_permission_persistence() {
    run_test(async { run_permission_persistence::<TestSession>().await });
}
