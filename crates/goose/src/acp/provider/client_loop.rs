use super::*;

pub struct AcpClientLoop {
    config: AcpProviderConfig,
    goose_mode: Arc<Mutex<GooseMode>>,
    prompt_response_tx: Arc<Mutex<Option<mpsc::Sender<AcpUpdate>>>>,
    pending_tool_updates: Arc<Mutex<HashMap<String, AccumulatedToolCall>>>,
}

impl AcpClientLoop {
    pub(super) fn new(
        config: AcpProviderConfig,
        goose_mode: Arc<Mutex<GooseMode>>,
        pending_tool_updates: Arc<Mutex<HashMap<String, AccumulatedToolCall>>>,
    ) -> Self {
        Self {
            config,
            goose_mode,
            prompt_response_tx: Arc::new(Mutex::new(None)),
            pending_tool_updates,
        }
    }

    pub(super) async fn spawn(
        self,
        mut rx: mpsc::Receiver<ClientRequest>,
        init_tx: oneshot::Sender<Result<InitializeResponse>>,
    ) {
        let child = match spawn_acp_process(&self.config).await {
            Ok(c) => c,
            Err(e) => {
                let _ = init_tx.send(Err(anyhow::anyhow!("{e}")));
                tracing::error!("failed to spawn ACP process: {e}");
                return;
            }
        };

        match self.run_with_child(child, &mut rx, init_tx).await {
            Ok(()) => tracing::debug!("ACP protocol loop exited cleanly"),
            Err(e) => tracing::error!(error = %e, "ACP protocol loop error"),
        }
    }

    async fn run_with_child(
        self,
        mut child: Child,
        rx: &mut mpsc::Receiver<ClientRequest>,
        init_tx: oneshot::Sender<Result<InitializeResponse>>,
    ) -> Result<()> {
        let stdin = child.stdin.take().context("no stdin")?;
        let stdout = child.stdout.take().context("no stdout")?;
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(forward_child_stderr(stderr));
        }
        let transport =
            agent_client_protocol::ByteStreams::new(stdin.compat_write(), stdout.compat());
        let result = self.run(transport, rx, init_tx).await;
        let _ = child.kill().await;
        let _ = child.wait().await;
        result
    }

    pub(super) async fn run(
        self,
        transport: impl agent_client_protocol::ConnectTo<Client> + 'static,
        rx: &mut mpsc::Receiver<ClientRequest>,
        init_tx: oneshot::Sender<Result<InitializeResponse>>,
    ) -> Result<()> {
        let AcpClientLoop {
            config,
            goose_mode,
            prompt_response_tx,
            pending_tool_updates,
        } = self;
        let notification_callback = config.notification_callback.clone();
        let reverse_modes = reverse_mode_mapping(&config.mode_mapping);

        Client
            .builder()
            .on_receive_notification(
                {
                    let prompt_response_tx = prompt_response_tx.clone();
                    let reverse_modes = reverse_modes.clone();
                    let goose_mode = goose_mode.clone();
                    let pending_tool_updates = pending_tool_updates.clone();
                    async move |notification: SessionNotification, _cx| {
                        if let Some(ref cb) = notification_callback {
                            cb(notification.clone());
                        }
                        match &notification.update {
                            SessionUpdate::CurrentModeUpdate(update) => {
                                if let Some(mode) = resolve_mode(
                                    &reverse_modes,
                                    update.current_mode_id.0.as_ref(),
                                    &goose_mode,
                                ) {
                                    if let Ok(mut guard) = goose_mode.lock() {
                                        *guard = mode;
                                    }
                                }
                            }
                            SessionUpdate::ConfigOptionUpdate(update) => {
                                for opt in &update.config_options {
                                    if opt.category == Some(SessionConfigOptionCategory::Mode) {
                                        if let SessionConfigKind::Select(sel) = &opt.kind {
                                            if let Some(mode) = resolve_mode(
                                                &reverse_modes,
                                                sel.current_value.0.as_ref(),
                                                &goose_mode,
                                            ) {
                                                if let Ok(mut guard) = goose_mode.lock() {
                                                    *guard = mode;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        if let Some(tx) = prompt_response_tx
                            .lock()
                            .ok()
                            .as_ref()
                            .and_then(|g| g.as_ref())
                        {
                            match notification.update {
                                SessionUpdate::AgentMessageChunk(ContentChunk {
                                    content: ContentBlock::Text(TextContent { text, .. }),
                                    ..
                                }) => {
                                    let _ = tx.try_send(AcpUpdate::Text(text));
                                }
                                SessionUpdate::AgentThoughtChunk(ContentChunk {
                                    content: ContentBlock::Text(TextContent { text, .. }),
                                    ..
                                }) => {
                                    let _ = tx.try_send(AcpUpdate::Thought(text));
                                }
                                SessionUpdate::ToolCall(tool_call) => {
                                    let id = tool_call.tool_call_id.0.to_string();
                                    let initial_status = tool_call.status;
                                    let synchronous_terminal = matches!(
                                        initial_status,
                                        ToolCallStatus::Completed | ToolCallStatus::Failed
                                    );
                                    // Seed the buffer; drain immediately if the call is
                                    // already terminal (synchronous tool, no follow-up).
                                    let synchronous_accumulated =
                                        if let Ok(mut buffer) = pending_tool_updates.lock() {
                                            let entry = buffer.entry(id.clone()).or_default();
                                            if let Some(raw_output) = tool_call.raw_output.clone() {
                                                entry.raw_output = Some(raw_output);
                                            }
                                            entry.content.extend(tool_call.content.clone());
                                            if synchronous_terminal {
                                                buffer.remove(&id)
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        };
                                    // ACP carries no canonical tool name to clients — only
                                    // `title` (display) and `kind` (category). We pass `title`
                                    // for renderer affordance, surface `kind` separately via
                                    // tool_meta for stable categorization, and the
                                    // goose.external_dispatch marker keeps `name` off the
                                    // agent loop's routing/auth paths.
                                    let _ = tx.try_send(AcpUpdate::ToolCallStart {
                                        id: id.clone(),
                                        name: tool_call.title.clone(),
                                        kind: tool_call.kind,
                                        raw_input: tool_call.raw_input.clone(),
                                    });
                                    if let Some(accumulated) = synchronous_accumulated {
                                        let content = if accumulated.content.is_empty() {
                                            None
                                        } else {
                                            Some(accumulated.content)
                                        };
                                        let _ = tx.try_send(AcpUpdate::ToolCallComplete {
                                            id,
                                            raw_output: accumulated.raw_output,
                                            content,
                                            is_error: matches!(
                                                initial_status,
                                                ToolCallStatus::Failed
                                            ),
                                        });
                                    }
                                }
                                SessionUpdate::ToolCallUpdate(update) => {
                                    let id = update.tool_call_id.0.to_string();
                                    // Merge patch-like fields; only emit on terminal status.
                                    let terminal_status = update.fields.status.filter(|s| {
                                        matches!(
                                            s,
                                            ToolCallStatus::Completed | ToolCallStatus::Failed
                                        )
                                    });
                                    let accumulated = if let Ok(mut buffer) =
                                        pending_tool_updates.lock()
                                    {
                                        let entry = buffer.entry(id.clone()).or_default();
                                        if let Some(raw_output) = update.fields.raw_output.clone() {
                                            entry.raw_output = Some(raw_output);
                                        }
                                        if let Some(content) = update.fields.content.clone() {
                                            entry.content.extend(content);
                                        }
                                        if terminal_status.is_some() {
                                            buffer.remove(&id)
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    if let (Some(accumulated), Some(status)) =
                                        (accumulated, terminal_status)
                                    {
                                        let content = if accumulated.content.is_empty() {
                                            None
                                        } else {
                                            Some(accumulated.content)
                                        };
                                        let _ = tx.try_send(AcpUpdate::ToolCallComplete {
                                            id,
                                            raw_output: accumulated.raw_output,
                                            content,
                                            is_error: matches!(status, ToolCallStatus::Failed),
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                        Ok(())
                    }
                },
                agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_request(
                {
                    let prompt_response_tx = prompt_response_tx.clone();
                    async move |request: RequestPermissionRequest, responder, _connection_cx| {
                        let (response_tx, response_rx) = oneshot::channel();

                        let handler = prompt_response_tx
                            .lock()
                            .ok()
                            .as_ref()
                            .and_then(|g| g.as_ref().cloned());
                        let tx =
                            handler.ok_or_else(agent_client_protocol::Error::internal_error)?;

                        if tx.is_closed() {
                            return Err(agent_client_protocol::Error::internal_error());
                        }

                        tx.try_send(AcpUpdate::PermissionRequest {
                            request: Box::new(request),
                            response_tx,
                        })
                        .map_err(|_| agent_client_protocol::Error::internal_error())?;

                        let response = response_rx.await.unwrap_or_else(|_| {
                            RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled)
                        });
                        responder.respond(response)
                    }
                },
                agent_client_protocol::on_receive_request!(),
            )
            .connect_with(transport, async move |cx: ConnectionTo<Agent>| {
                handle_requests(config, goose_mode, cx, rx, prompt_response_tx, init_tx).await
            })
            .await?;

        Ok(())
    }
}

/// Forwards an ACP child's stderr to tracing line by line.
///
/// Lines longer than `MAX_LINE_LEN` are flushed in chunks so a child that
/// emits unbounded output without newlines (e.g. carriage-return progress
/// bars or binary data) cannot cause unbounded memory growth.
async fn forward_child_stderr(mut stderr: tokio::process::ChildStderr) {
    const MAX_LINE_LEN: usize = 8192;
    const READ_CHUNK: usize = 1024;

    let mut line: Vec<u8> = Vec::with_capacity(256);
    let mut chunk = [0u8; READ_CHUNK];
    loop {
        match stderr.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                for &b in &chunk[..n] {
                    if b == b'\n' {
                        emit_stderr_line(&mut line);
                    } else {
                        line.push(b);
                        if line.len() >= MAX_LINE_LEN {
                            emit_stderr_line(&mut line);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!(target: "acp::child::stderr", error = %e, "stderr read error");
                break;
            }
        }
    }
    emit_stderr_line(&mut line);
}

fn emit_stderr_line(line: &mut Vec<u8>) {
    if line.is_empty() {
        return;
    }
    let trimmed = line.strip_suffix(b"\r").unwrap_or(line);
    tracing::info!(target: "acp::child::stderr", "{}", String::from_utf8_lossy(trimmed));
    line.clear();
}

async fn spawn_acp_process(config: &AcpProviderConfig) -> Result<Child> {
    let mut cmd = Command::new(&config.command);
    cmd.args(&config.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    for key in &config.env_remove {
        cmd.env_remove(key);
    }

    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    configure_subprocess(&mut cmd);
    cmd.spawn().context("failed to spawn ACP process")
}

fn log_undelivered<E: std::fmt::Debug>(result: Result<(), E>, method: &str) {
    if let Err(e) = result {
        tracing::debug!(method, error = ?e, "response not delivered");
    }
}

async fn handle_requests(
    config: AcpProviderConfig,
    goose_mode: Arc<Mutex<GooseMode>>,
    cx: ConnectionTo<Agent>,
    rx: &mut mpsc::Receiver<ClientRequest>,
    prompt_response_tx: Arc<Mutex<Option<mpsc::Sender<AcpUpdate>>>>,
    init_tx: oneshot::Sender<Result<InitializeResponse>>,
) -> Result<(), agent_client_protocol::Error> {
    let mut init_tx = Some(init_tx);

    let client_capabilities = ClientCapabilities::new();
    let init_response: InitializeResponse = cx
        .send_request(
            InitializeRequest::new(ProtocolVersion::LATEST)
                .client_capabilities(client_capabilities),
        )
        .block_task()
        .await
        .map_err(|err| {
            let message = format!("ACP {} failed: {err}", AGENT_METHOD_NAMES.initialize);
            if let Some(tx) = init_tx.take() {
                let _ = tx.send(Err(anyhow::anyhow!(message.clone())));
            }
            agent_client_protocol::Error::internal_error().data(message)
        })?;

    let supports_close = init_response
        .agent_capabilities
        .session_capabilities
        .close
        .is_some();
    let mcp_capabilities = init_response.agent_capabilities.mcp_capabilities.clone();
    if let Some(tx) = init_tx.take() {
        log_undelivered(tx.send(Ok(init_response)), AGENT_METHOD_NAMES.initialize);
    }

    let mut session_ids: Vec<SessionId> = Vec::new();

    while let Some(request) = rx.recv().await {
        match request {
            ClientRequest::NewSession { response_tx } => {
                let mcp_servers = filter_supported_servers(&config.mcp_servers, &mcp_capabilities);
                let session = cx
                    .send_request(
                        NewSessionRequest::new(config.work_dir.clone()).mcp_servers(mcp_servers),
                    )
                    .block_task()
                    .await;
                let result = match session {
                    Ok(session) => {
                        session_ids.push(session.session_id.clone());
                        apply_session_mode(&config, &goose_mode, &cx, session).await
                    }
                    Err(err) => Err(anyhow::anyhow!(
                        "ACP {} failed: {err}",
                        AGENT_METHOD_NAMES.session_new
                    )),
                };
                log_undelivered(response_tx.send(result), AGENT_METHOD_NAMES.session_new);
            }
            ClientRequest::SetMode {
                session_id,
                mode_id,
                response_tx,
            } => {
                let result: Result<()> = cx
                    .send_request(SetSessionModeRequest::new(session_id, mode_id))
                    .block_task()
                    .await
                    .map(|_| ())
                    .map_err(anyhow::Error::from);
                log_undelivered(
                    response_tx.send(result),
                    AGENT_METHOD_NAMES.session_set_mode,
                );
            }
            ClientRequest::SetConfigOption {
                session_id,
                config_id,
                value,
                response_tx,
            } => {
                let value_id = agent_client_protocol::schema::SessionConfigValueId::new(value);
                let req = SetSessionConfigOptionRequest::new(session_id, config_id, value_id);
                let result: Result<()> = cx
                    .send_request(req)
                    .block_task()
                    .await
                    .map(|_| ())
                    .map_err(anyhow::Error::from);
                log_undelivered(
                    response_tx.send(result),
                    AGENT_METHOD_NAMES.session_set_config_option,
                );
            }
            ClientRequest::Prompt {
                session_id,
                content,
                response_tx,
            } => {
                *prompt_response_tx.lock().unwrap() = Some(response_tx.clone());

                let response: Result<PromptResponse, _> = cx
                    .send_request(PromptRequest::new(session_id, content))
                    .block_task()
                    .await;

                match response {
                    Ok(r) => {
                        log_undelivered(
                            response_tx.try_send(AcpUpdate::Complete(r.stop_reason, r.usage)),
                            AGENT_METHOD_NAMES.session_prompt,
                        );
                    }
                    Err(e) => {
                        log_undelivered(
                            response_tx.try_send(AcpUpdate::Error(e.to_string())),
                            AGENT_METHOD_NAMES.session_prompt,
                        );
                    }
                }

                *prompt_response_tx.lock().unwrap() = None;
            }
        }
    }

    if supports_close {
        for session_id in session_ids {
            if let Err(e) = cx
                .send_request(CloseSessionRequest::new(session_id.clone()))
                .block_task()
                .await
            {
                tracing::debug!(method = AGENT_METHOD_NAMES.session_close, session_id = %session_id, error = %e, "failed on shutdown");
            }
        }
    }

    Ok(())
}
