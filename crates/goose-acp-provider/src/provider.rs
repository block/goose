use anyhow::{Context, Result};
use sacp::schema::{
    ContentBlock, ContentChunk, InitializeRequest, McpServer, NewSessionRequest, PermissionOption,
    PermissionOptionKind, PromptRequest, ProtocolVersion, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome, SessionId,
    SessionNotification, SessionUpdate, SetSessionModeRequest, StopReason, TextContent,
    ToolCallContent, ToolCallStatus,
};
use sacp::{ClientToAgent, JrConnectionCx};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex as TokioMutex};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

// ============================================================================
// Permission types (merged from provider.rs)
// ============================================================================

#[derive(Clone, Debug)]
pub struct PermissionMapping {
    pub allow_option_id: Option<String>,
    pub reject_option_id: Option<String>,
    pub rejected_tool_status: ToolCallStatus,
}

impl Default for PermissionMapping {
    fn default() -> Self {
        Self {
            allow_option_id: None,
            reject_option_id: None,
            rejected_tool_status: ToolCallStatus::Failed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionDecision {
    AllowAlways,
    AllowOnce,
    RejectAlways,
    RejectOnce,
    Cancel,
}

impl PermissionDecision {
    pub(crate) fn should_record_rejection(self) -> bool {
        matches!(
            self,
            PermissionDecision::RejectAlways
                | PermissionDecision::RejectOnce
                | PermissionDecision::Cancel
        )
    }
}

pub fn map_permission_response(
    mapping: &PermissionMapping,
    request: &RequestPermissionRequest,
    decision: PermissionDecision,
) -> RequestPermissionResponse {
    let selected_id = match decision {
        PermissionDecision::AllowAlways => select_option_id(
            &request.options,
            &mapping.allow_option_id,
            PermissionOptionKind::AllowAlways,
        )
        .or_else(|| {
            select_option_id(
                &request.options,
                &mapping.allow_option_id,
                PermissionOptionKind::AllowOnce,
            )
        }),
        PermissionDecision::AllowOnce => select_option_id(
            &request.options,
            &mapping.allow_option_id,
            PermissionOptionKind::AllowOnce,
        )
        .or_else(|| {
            select_option_id(
                &request.options,
                &mapping.allow_option_id,
                PermissionOptionKind::AllowAlways,
            )
        }),
        PermissionDecision::RejectAlways => select_option_id(
            &request.options,
            &mapping.reject_option_id,
            PermissionOptionKind::RejectAlways,
        )
        .or_else(|| {
            select_option_id(
                &request.options,
                &mapping.reject_option_id,
                PermissionOptionKind::RejectOnce,
            )
        }),
        PermissionDecision::RejectOnce => select_option_id(
            &request.options,
            &mapping.reject_option_id,
            PermissionOptionKind::RejectOnce,
        )
        .or_else(|| {
            select_option_id(
                &request.options,
                &mapping.reject_option_id,
                PermissionOptionKind::RejectAlways,
            )
        }),
        PermissionDecision::Cancel => None,
    };

    if let Some(option_id) = selected_id {
        RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
            SelectedPermissionOutcome::new(option_id),
        ))
    } else {
        RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled)
    }
}

fn select_option_id(
    options: &[PermissionOption],
    preferred_id: &Option<String>,
    kind: PermissionOptionKind,
) -> Option<String> {
    if let Some(preferred_id) = preferred_id {
        let preferred = sacp::schema::PermissionOptionId::new(preferred_id.clone());
        if options.iter().any(|opt| opt.option_id == preferred) {
            return Some(preferred_id.clone());
        }
    }

    options
        .iter()
        .find(|opt| opt.kind == kind)
        .map(|opt| opt.option_id.0.to_string())
}

// ============================================================================
// Config types
// ============================================================================

/// Session configuration shared between server and provider.
/// Use `Default::default()` for easy test setup.
#[derive(Clone, Debug, Default)]
pub struct AcpSessionConfig {
    pub mcp_servers: Vec<McpServer>,
    pub work_dir: PathBuf,
}

/// Provider configuration that embeds session config plus process spawning details.
#[derive(Clone, Debug)]
pub struct AcpProviderConfig {
    pub session: AcpSessionConfig,
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub permission_mapping: PermissionMapping,
    pub session_mode_id: Option<String>,
}

impl Default for AcpProviderConfig {
    fn default() -> Self {
        Self {
            session: AcpSessionConfig::default(),
            command: PathBuf::from("goose"),
            args: vec!["acp".to_string()],
            env: vec![],
            permission_mapping: PermissionMapping::default(),
            session_mode_id: None,
        }
    }
}

/// Internal client config used by AcpClient.
/// This is derived from AcpProviderConfig for spawning the process.
#[derive(Clone, Debug)]
pub(crate) struct AcpClientConfig {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub work_dir: PathBuf,
    pub mcp_servers: Vec<McpServer>,
    pub session_mode_id: Option<String>,
    pub permission_mapping: PermissionMapping,
}

impl From<AcpProviderConfig> for AcpClientConfig {
    fn from(config: AcpProviderConfig) -> Self {
        Self {
            command: config.command,
            args: config.args,
            env: config.env,
            work_dir: config.session.work_dir,
            mcp_servers: config.session.mcp_servers,
            session_mode_id: config.session_mode_id,
            permission_mapping: config.permission_mapping,
        }
    }
}

impl Default for AcpClientConfig {
    fn default() -> Self {
        Self {
            command: PathBuf::from("goose"),
            args: vec!["acp".to_string()],
            env: vec![],
            work_dir: std::env::current_dir().unwrap_or_default(),
            mcp_servers: vec![],
            session_mode_id: None,
            permission_mapping: PermissionMapping::default(),
        }
    }
}

// ============================================================================
// AcpClient
// ============================================================================

#[derive(Clone)]
pub struct AcpClient {
    tx: mpsc::Sender<ClientRequest>,
    permission_mapping: PermissionMapping,
    rejected_tool_calls: Arc<TokioMutex<HashSet<String>>>,
}

enum ClientRequest {
    NewSession {
        response_tx: oneshot::Sender<Result<SessionId>>,
    },
    Prompt {
        session_id: SessionId,
        content: Vec<ContentBlock>,
        response_tx: mpsc::Sender<AcpUpdate>,
    },
    Shutdown,
}

#[derive(Debug)]
pub enum AcpUpdate {
    Text(String),
    Thought(String),
    ToolCallStart {
        id: String,
        title: String,
        raw_input: Option<serde_json::Value>,
    },
    ToolCallComplete {
        id: String,
        status: ToolCallStatus,
        content: Vec<ToolCallContent>,
    },
    PermissionRequest {
        request: Box<RequestPermissionRequest>,
        response_tx: oneshot::Sender<RequestPermissionResponse>,
    },
    Complete(StopReason),
    Error(String),
}

impl AcpClient {
    /// Connect to an ACP agent using the provider config (spawns process).
    pub async fn connect(config: AcpProviderConfig) -> Result<Self> {
        Self::connect_internal(config.into()).await
    }

    /// Connect using internal client config.
    pub(crate) async fn connect_internal(config: AcpClientConfig) -> Result<Self> {
        let (tx, rx) = mpsc::channel(32);
        let (init_tx, init_rx) = oneshot::channel();
        let permission_mapping = config.permission_mapping.clone();
        let rejected_tool_calls = Arc::new(TokioMutex::new(HashSet::new()));

        tokio::spawn(run_client_loop(config, rx, init_tx));

        init_rx
            .await
            .context("ACP client initialization cancelled")??;

        Ok(Self {
            tx,
            permission_mapping,
            rejected_tool_calls,
        })
    }

    /// Connect with a custom transport (for testing).
    pub async fn connect_with_transport<R, W>(
        config: AcpProviderConfig,
        read: R,
        write: W,
    ) -> Result<Self>
    where
        R: futures::AsyncRead + Unpin + Send + 'static,
        W: futures::AsyncWrite + Unpin + Send + 'static,
    {
        Self::connect_with_transport_internal(config.into(), read, write).await
    }

    pub(crate) async fn connect_with_transport_internal<R, W>(
        config: AcpClientConfig,
        read: R,
        write: W,
    ) -> Result<Self>
    where
        R: futures::AsyncRead + Unpin + Send + 'static,
        W: futures::AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel(32);
        let (init_tx, init_rx) = oneshot::channel();
        let permission_mapping = config.permission_mapping.clone();
        let rejected_tool_calls = Arc::new(TokioMutex::new(HashSet::new()));
        let transport = sacp::ByteStreams::new(write, read);
        let init_tx = Arc::new(Mutex::new(Some(init_tx)));
        tokio::spawn(async move {
            if let Err(e) =
                run_protocol_loop_with_transport(config, transport, &mut rx, init_tx.clone()).await
            {
                tracing::error!("ACP protocol error: {e}");
            }
        });

        init_rx
            .await
            .context("ACP client initialization cancelled")??;

        Ok(Self {
            tx,
            permission_mapping,
            rejected_tool_calls,
        })
    }

    pub async fn new_session(&self) -> Result<SessionId> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(ClientRequest::NewSession { response_tx })
            .await
            .context("ACP client is unavailable")?;
        response_rx.await.context("ACP session/new cancelled")?
    }

    pub async fn prompt(
        &self,
        session_id: SessionId,
        content: Vec<ContentBlock>,
    ) -> Result<mpsc::Receiver<AcpUpdate>> {
        let (response_tx, response_rx) = mpsc::channel(64);
        self.tx
            .send(ClientRequest::Prompt {
                session_id,
                content,
                response_tx,
            })
            .await
            .context("ACP client is unavailable")?;
        Ok(response_rx)
    }

    pub async fn permission_response(
        &self,
        request: &RequestPermissionRequest,
        decision: PermissionDecision,
    ) -> RequestPermissionResponse {
        if decision.should_record_rejection() {
            self.rejected_tool_calls
                .lock()
                .await
                .insert(request.tool_call.tool_call_id.0.to_string());
        }

        map_permission_response(&self.permission_mapping, request, decision)
    }

    pub async fn tool_call_is_error(&self, tool_call_id: &str, status: ToolCallStatus) -> bool {
        let was_rejected = self.rejected_tool_calls.lock().await.remove(tool_call_id);

        match status {
            ToolCallStatus::Failed => true,
            ToolCallStatus::Completed => {
                was_rejected
                    && self.permission_mapping.rejected_tool_status == ToolCallStatus::Completed
            }
            _ => false,
        }
    }
}

impl Drop for AcpClient {
    fn drop(&mut self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(ClientRequest::Shutdown).await;
        });
    }
}

async fn run_client_loop(
    config: AcpClientConfig,
    mut rx: mpsc::Receiver<ClientRequest>,
    init_tx: oneshot::Sender<Result<()>>,
) {
    let init_tx = Arc::new(Mutex::new(Some(init_tx)));

    let child = match spawn_acp_process(&config).await {
        Ok(c) => c,
        Err(e) => {
            let message = e.to_string();
            send_init_result(&init_tx, Err(anyhow::anyhow!(message.clone())));
            tracing::error!("failed to spawn ACP process: {message}");
            return;
        }
    };

    if let Err(e) = run_protocol_loop_with_child(config, child, &mut rx, init_tx.clone()).await {
        let message = e.to_string();
        send_init_result(&init_tx, Err(anyhow::anyhow!(message.clone())));
        tracing::error!("ACP protocol error: {message}");
    }
}

async fn spawn_acp_process(config: &AcpClientConfig) -> Result<Child> {
    let mut cmd = Command::new(&config.command);
    cmd.args(&config.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);

    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    cmd.spawn().context("failed to spawn ACP process")
}

async fn run_protocol_loop_with_child(
    config: AcpClientConfig,
    mut child: Child,
    rx: &mut mpsc::Receiver<ClientRequest>,
    init_tx: Arc<Mutex<Option<oneshot::Sender<Result<()>>>>>,
) -> Result<()> {
    let stdin = child.stdin.take().context("no stdin")?;
    let stdout = child.stdout.take().context("no stdout")?;
    let transport = sacp::ByteStreams::new(stdin.compat_write(), stdout.compat());
    run_protocol_loop_with_transport(config, transport, rx, init_tx).await
}

async fn run_protocol_loop_with_transport<R, W>(
    config: AcpClientConfig,
    transport: sacp::ByteStreams<W, R>,
    rx: &mut mpsc::Receiver<ClientRequest>,
    init_tx: Arc<Mutex<Option<oneshot::Sender<Result<()>>>>>,
) -> Result<()>
where
    R: futures::AsyncRead + Unpin + Send + 'static,
    W: futures::AsyncWrite + Unpin + Send + 'static,
{
    let prompt_response_tx: Arc<Mutex<Option<mpsc::Sender<AcpUpdate>>>> =
        Arc::new(Mutex::new(None));

    ClientToAgent::builder()
        .on_receive_notification(
            {
                let prompt_response_tx = prompt_response_tx.clone();
                async move |notification: SessionNotification, _cx| {
                    if let Some(tx) = prompt_response_tx.lock().unwrap().as_ref() {
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
                                let _ = tx.try_send(AcpUpdate::ToolCallStart {
                                    id: tool_call.tool_call_id.0.to_string(),
                                    title: tool_call.title,
                                    raw_input: tool_call.raw_input,
                                });
                            }
                            SessionUpdate::ToolCallUpdate(update) => {
                                if let Some(status) = update.fields.status {
                                    let _ = tx.try_send(AcpUpdate::ToolCallComplete {
                                        id: update.tool_call_id.0.to_string(),
                                        status,
                                        content: update.fields.content.unwrap_or_default(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                }
            },
            sacp::on_receive_notification!(),
        )
        .on_receive_request(
            {
                let prompt_response_tx = prompt_response_tx.clone();
                async move |request: RequestPermissionRequest, request_cx, _connection_cx| {
                    let (response_tx, response_rx) = oneshot::channel();

                    let handler = prompt_response_tx.lock().unwrap().as_ref().cloned();
                    let tx = handler.ok_or_else(sacp::Error::internal_error)?;

                    if tx.is_closed() {
                        return Err(sacp::Error::internal_error());
                    }

                    tx.try_send(AcpUpdate::PermissionRequest {
                        request: Box::new(request),
                        response_tx,
                    })
                    .map_err(|_| sacp::Error::internal_error())?;

                    let response = response_rx.await.unwrap_or_else(|_| {
                        RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled)
                    });
                    request_cx.respond(response)
                }
            },
            sacp::on_receive_request!(),
        )
        .connect_to(transport)?
        .run_until({
            let prompt_response_tx = prompt_response_tx.clone();
            move |cx: JrConnectionCx<ClientToAgent>| {
                handle_requests(config, cx, rx, prompt_response_tx, init_tx.clone())
            }
        })
        .await?;

    Ok(())
}

async fn handle_requests(
    config: AcpClientConfig,
    cx: JrConnectionCx<ClientToAgent>,
    rx: &mut mpsc::Receiver<ClientRequest>,
    prompt_response_tx: Arc<Mutex<Option<mpsc::Sender<AcpUpdate>>>>,
    init_tx: Arc<Mutex<Option<oneshot::Sender<Result<()>>>>>,
) -> Result<(), sacp::Error> {
    cx.send_request(InitializeRequest::new(ProtocolVersion::LATEST))
        .block_task()
        .await
        .map_err(|err| {
            let message = format!("ACP initialize failed: {err}");
            send_init_result(&init_tx, Err(anyhow::anyhow!(message.clone())));
            sacp::Error::internal_error().data(message)
        })?;

    send_init_result(&init_tx, Ok(()));

    while let Some(request) = rx.recv().await {
        match request {
            ClientRequest::NewSession { response_tx } => {
                let session = cx
                    .send_request(
                        NewSessionRequest::new(config.work_dir.clone())
                            .mcp_servers(config.mcp_servers.clone()),
                    )
                    .block_task()
                    .await;

                let result = match session {
                    Ok(session) => {
                        let session_id = session.session_id.clone();
                        let mut result = Ok(session_id);

                        if let Some(mode_id) = config.session_mode_id.clone() {
                            let modes = match session.modes {
                                Some(modes) => Some(modes),
                                None => {
                                    result = Err(anyhow::anyhow!(
                                        "ACP agent did not advertise SessionModeState"
                                    ));
                                    None
                                }
                            };

                            if let (Some(modes), Ok(_)) = (modes, result.as_ref()) {
                                if modes.current_mode_id.0.as_ref() != mode_id.as_str() {
                                    let available: Vec<String> = modes
                                        .available_modes
                                        .iter()
                                        .map(|mode| mode.id.0.to_string())
                                        .collect();

                                    if !available.iter().any(|id| id == &mode_id) {
                                        result = Err(anyhow::anyhow!(
                                            "Requested mode '{}' not offered by agent. Available modes: {}",
                                            mode_id,
                                            available.join(", ")
                                        ));
                                    } else if let Err(err) = cx
                                        .send_request(SetSessionModeRequest::new(
                                            session.session_id.clone(),
                                            mode_id,
                                        ))
                                        .block_task()
                                        .await
                                    {
                                        result = Err(anyhow::anyhow!(
                                            "ACP agent rejected session/set_mode: {err}"
                                        ));
                                    }
                                }
                            }
                        }

                        result
                    }
                    Err(err) => Err(anyhow::anyhow!("ACP session/new failed: {err}")),
                };

                let _ = response_tx.send(result);
            }
            ClientRequest::Prompt {
                session_id,
                content,
                response_tx,
            } => {
                *prompt_response_tx.lock().unwrap() = Some(response_tx.clone());

                let response = cx
                    .send_request(PromptRequest::new(session_id, content))
                    .block_task()
                    .await;

                match response {
                    Ok(r) => {
                        let _ = response_tx.try_send(AcpUpdate::Complete(r.stop_reason));
                    }
                    Err(e) => {
                        let _ = response_tx.try_send(AcpUpdate::Error(e.to_string()));
                    }
                }

                *prompt_response_tx.lock().unwrap() = None;
            }
            ClientRequest::Shutdown => break,
        }
    }

    Ok(())
}

fn send_init_result(init_tx: &Arc<Mutex<Option<oneshot::Sender<Result<()>>>>>, result: Result<()>) {
    if let Some(tx) = init_tx.lock().unwrap().take() {
        let _ = tx.send(result);
    }
}

pub fn text_content(text: impl Into<String>) -> ContentBlock {
    ContentBlock::Text(TextContent::new(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sacp::schema::{PermissionOptionId, ToolCallId, ToolCallUpdate, ToolCallUpdateFields};
    use test_case::test_case;

    fn make_request(options: Vec<PermissionOption>) -> RequestPermissionRequest {
        let tool_call =
            ToolCallUpdate::new(ToolCallId::new("tool-1"), ToolCallUpdateFields::default());
        RequestPermissionRequest::new("session-1", tool_call, options)
    }

    fn option(id: &str, kind: PermissionOptionKind) -> PermissionOption {
        PermissionOption::new(
            PermissionOptionId::new(id.to_string()),
            id.to_string(),
            kind,
        )
    }

    #[test_case(
        Some("allow"),
        None,
        PermissionDecision::AllowOnce,
        "allow",
        true;
        "allow_uses_preferred_id"
    )]
    #[test_case(
        None,
        None,
        PermissionDecision::AllowAlways,
        "allow_always",
        false;
        "allow_always_prefers_kind"
    )]
    #[test_case(
        Some("missing"),
        None,
        PermissionDecision::AllowOnce,
        "allow_once",
        false;
        "allow_falls_back_to_kind"
    )]
    #[test_case(
        None,
        Some("reject"),
        PermissionDecision::RejectOnce,
        "reject",
        true;
        "reject_uses_preferred_id"
    )]
    #[test_case(
        None,
        Some("missing"),
        PermissionDecision::RejectOnce,
        "reject_once",
        false;
        "reject_falls_back_to_kind"
    )]
    fn test_permission_mapping(
        allow_option_id: Option<&str>,
        reject_option_id: Option<&str>,
        decision: PermissionDecision,
        expected_id: &str,
        include_preferred: bool,
    ) {
        let mut options = vec![
            option("allow_once", PermissionOptionKind::AllowOnce),
            option("allow_always", PermissionOptionKind::AllowAlways),
            option("reject_once", PermissionOptionKind::RejectOnce),
            option("reject", PermissionOptionKind::RejectAlways),
        ];

        if include_preferred {
            if let Some(preferred_allow) = allow_option_id {
                if !options
                    .iter()
                    .any(|opt| opt.option_id.0.as_ref() == preferred_allow)
                {
                    options.push(option(preferred_allow, PermissionOptionKind::AllowOnce));
                }
            }

            if let Some(preferred_reject) = reject_option_id {
                if !options
                    .iter()
                    .any(|opt| opt.option_id.0.as_ref() == preferred_reject)
                {
                    options.push(option(preferred_reject, PermissionOptionKind::RejectOnce));
                }
            }
        }

        let request = make_request(options);

        let mapping = PermissionMapping {
            allow_option_id: allow_option_id.map(|s| s.to_string()),
            reject_option_id: reject_option_id.map(|s| s.to_string()),
            rejected_tool_status: ToolCallStatus::Failed,
        };

        let response = map_permission_response(&mapping, &request, decision);
        match response.outcome {
            RequestPermissionOutcome::Selected(selected) => {
                assert_eq!(selected.option_id.0.as_ref(), expected_id);
            }
            _ => panic!("expected selected outcome"),
        }
    }

    #[test_case(PermissionDecision::Cancel; "cancelled")]
    fn test_permission_cancelled(decision: PermissionDecision) {
        let request = make_request(vec![option("allow_once", PermissionOptionKind::AllowOnce)]);
        let response = map_permission_response(&PermissionMapping::default(), &request, decision);
        assert!(matches!(
            response.outcome,
            RequestPermissionOutcome::Cancelled
        ));
    }
}
