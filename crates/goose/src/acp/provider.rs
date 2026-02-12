use anyhow::{Context, Result};
use async_stream::try_stream;
use rmcp::model::{CallToolRequestParams, CallToolResult, Content, Role, Tool};
use sacp::schema::{
    ContentBlock, ContentChunk, EnvVariable, HttpHeader, InitializeRequest, McpCapabilities,
    McpServer, McpServerHttp, McpServerStdio, NewSessionRequest, NewSessionResponse, PromptRequest,
    ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SessionId, SessionModelState, SessionNotification, SessionUpdate, SetSessionModeRequest,
    StopReason, TextContent, ToolCallContent, ToolCallStatus,
};
use sacp::{ClientToAgent, JrConnectionCx};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex as TokioMutex};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::acp::{map_permission_response, PermissionDecision, PermissionMapping};
use crate::config::{ExtensionConfig, GooseMode};
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::permission::permission_confirmation::PrincipalType;
use crate::permission::{Permission, PermissionConfirmation};
use crate::providers::base::{MessageStream, PermissionRouting, Provider, ProviderUsage, Usage};
use crate::providers::errors::ProviderError;
use crate::session::Session;

#[derive(Clone, Debug)]
pub struct AcpProviderConfig {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub work_dir: PathBuf,
    pub mcp_servers: Vec<McpServer>,
    pub session_mode_id: Option<String>,
    pub permission_mapping: PermissionMapping,
}

enum ClientRequest {
    NewSession {
        response_tx: oneshot::Sender<Result<(SessionId, Option<SessionModelState>)>>,
    },
    SetModel {
        session_id: SessionId,
        model_id: String,
        response_tx: oneshot::Sender<Result<()>>,
    },
    Prompt {
        session_id: SessionId,
        content: Vec<ContentBlock>,
        response_tx: mpsc::Sender<AcpUpdate>,
    },
    Shutdown,
}

#[derive(Debug)]
enum AcpUpdate {
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

pub struct AcpProvider {
    name: String,
    model: ModelConfig,
    goose_mode: GooseMode,
    tx: mpsc::Sender<ClientRequest>,
    permission_mapping: PermissionMapping,
    rejected_tool_calls: Arc<TokioMutex<HashSet<String>>>,
    pending_confirmations:
        Arc<TokioMutex<HashMap<String, oneshot::Sender<PermissionConfirmation>>>>,
    sessions: Arc<TokioMutex<HashMap<String, Session>>>,
    goose_to_acp_id: Arc<TokioMutex<HashMap<String, String>>>,
}

impl std::fmt::Debug for AcpProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcpProvider")
            .field("name", &self.name)
            .field("model", &self.model)
            .finish()
    }
}

impl AcpProvider {
    pub async fn connect(
        name: String,
        model: ModelConfig,
        goose_mode: GooseMode,
        config: AcpProviderConfig,
    ) -> Result<Self> {
        let (tx, rx) = mpsc::channel(32);
        let (init_tx, init_rx) = oneshot::channel();
        let permission_mapping = config.permission_mapping.clone();
        let rejected_tool_calls = Arc::new(TokioMutex::new(HashSet::new()));

        tokio::spawn(run_client_loop(config, rx, init_tx));

        init_rx
            .await
            .context("ACP client initialization cancelled")??;

        Ok(Self::new_with_runtime(
            name,
            model,
            goose_mode,
            tx,
            permission_mapping,
            rejected_tool_calls,
        ))
    }

    pub async fn connect_with_transport<R, W>(
        name: String,
        model: ModelConfig,
        goose_mode: GooseMode,
        config: AcpProviderConfig,
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

        Ok(Self::new_with_runtime(
            name,
            model,
            goose_mode,
            tx,
            permission_mapping,
            rejected_tool_calls,
        ))
    }

    fn new_with_runtime(
        name: String,
        model: ModelConfig,
        goose_mode: GooseMode,
        tx: mpsc::Sender<ClientRequest>,
        permission_mapping: PermissionMapping,
        rejected_tool_calls: Arc<TokioMutex<HashSet<String>>>,
    ) -> Self {
        Self {
            name,
            model,
            goose_mode,
            tx,
            permission_mapping,
            rejected_tool_calls,
            pending_confirmations: Arc::new(TokioMutex::new(HashMap::new())),
            sessions: Arc::new(TokioMutex::new(HashMap::new())),
            goose_to_acp_id: Arc::new(TokioMutex::new(HashMap::new())),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn model(&self) -> ModelConfig {
        self.model.clone()
    }

    pub fn permission_routing(&self) -> PermissionRouting {
        PermissionRouting::ActionRequired
    }

    pub async fn new_session(&self) -> Result<(SessionId, Option<SessionModelState>)> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(ClientRequest::NewSession { response_tx })
            .await
            .context("ACP client is unavailable")?;
        response_rx.await.context("ACP session/new cancelled")?
    }

    pub async fn set_model(&self, session_id: &SessionId, model_id: &str) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(ClientRequest::SetModel {
                session_id: session_id.clone(),
                model_id: model_id.to_string(),
                response_tx,
            })
            .await
            .context("ACP client is unavailable")?;
        response_rx
            .await
            .context("ACP session/set_model cancelled")?
    }

    pub async fn handle_permission_confirmation(
        &self,
        request_id: &str,
        confirmation: &PermissionConfirmation,
    ) -> bool {
        let mut pending = self.pending_confirmations.lock().await;
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(confirmation.clone());
            return true;
        }
        false
    }

    pub async fn complete_with_model(
        &self,
        session_id: &str,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let stream = self.stream(session_id, system, messages, tools).await?;

        use futures::StreamExt;
        tokio::pin!(stream);

        let mut text = String::new();
        let mut last_error: Option<ProviderError> = None;
        while let Some(result) = stream.next().await {
            match result {
                Ok((Some(msg), _)) => {
                    for item in msg.content {
                        if let MessageContent::Text(t) = item {
                            text.push_str(&t.text);
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                }
                _ => {}
            }
        }

        if text.is_empty() {
            return Err(last_error.map(classify_error).unwrap_or_else(|| {
                ProviderError::RequestFailed(
                    "No response received from ACP agent".to_string(),
                )
            }));
        }

        let message = Message::assistant().with_text(text);

        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), Usage::default()),
        ))
    }

    pub async fn stream(
        &self,
        session_id: &str,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let prompt_blocks = messages_to_prompt(messages);
        let mut rx = self
            .prompt(SessionId::new(session_id.to_string()), prompt_blocks)
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to send ACP prompt: {e}")))?;

        let pending_confirmations = self.pending_confirmations.clone();
        let rejected_tool_calls = self.rejected_tool_calls.clone();
        let permission_mapping = self.permission_mapping.clone();
        let goose_mode = self.goose_mode;

        Ok(Box::pin(try_stream! {
            while let Some(update) = rx.recv().await {
                match update {
                    AcpUpdate::Text(text) => {
                        let message = Message::assistant().with_text(text);
                        yield (Some(message), None);
                    }
                    AcpUpdate::Thought(text) => {
                        let message = Message::assistant()
                            .with_thinking(text, "")
                            .with_visibility(true, false);
                        yield (Some(message), None);
                    }
                    AcpUpdate::ToolCallStart { id, title, raw_input } => {
                        let arguments = raw_input
                            .and_then(|v| v.as_object().cloned())
                            .unwrap_or_default();

                        let tool_call = CallToolRequestParams {
                            meta: None,
                            task: None,
                            name: title.into(),
                            arguments: Some(arguments),
                        };
                        let message = Message::assistant().with_tool_request(id.clone(), Ok(tool_call));
                        yield (Some(message), None);
                    }
                    AcpUpdate::ToolCallComplete { id, status, content } => {
                        let result_text = tool_call_content_to_text(&content);
                        let is_error = tool_call_is_error(&rejected_tool_calls, &permission_mapping, &id, status).await;

                        let call_result = CallToolResult {
                            content: if result_text.is_empty() {
                                content_blocks_to_rmcp(&content)
                            } else {
                                vec![Content::text(result_text)]
                            },
                            structured_content: None,
                            is_error: Some(is_error),
                            meta: None,
                        };

                        let message = Message::assistant().with_tool_response(id, Ok(call_result));
                        yield (Some(message), None);
                    }
                    AcpUpdate::PermissionRequest { request, response_tx } => {
                        if let Some(decision) = permission_decision_from_mode(goose_mode) {
                            let response = permission_response(&permission_mapping, &rejected_tool_calls, &request, decision).await;
                            let _ = response_tx.send(response);
                            continue;
                        }

                        let request_id = request.tool_call.tool_call_id.0.to_string();
                        let (tx, rx) = oneshot::channel();

                        pending_confirmations
                            .lock()
                            .await
                            .insert(request_id.clone(), tx);

                        if let Some(action_required) = build_action_required_message(&request) {
                            yield (Some(action_required), None);
                        }

                        let confirmation = rx.await.unwrap_or(PermissionConfirmation {
                            principal_type: PrincipalType::Tool,
                            permission: Permission::Cancel,
                        });

                        pending_confirmations.lock().await.remove(&request_id);

                        let decision = permission_decision_from_confirmation(&confirmation);
                        let response = permission_response(&permission_mapping, &rejected_tool_calls, &request, decision).await;
                        let _ = response_tx.send(response);
                    }
                    AcpUpdate::Complete(_reason) => {
                        break;
                    }
                    AcpUpdate::Error(e) => {
                        Err(ProviderError::RequestFailed(e))?;
                    }
                }
            }
        }))
    }

    pub async fn ensure_session(&self, session_id: Option<&str>) -> Result<String, ProviderError> {
        let goose_id = session_id.ok_or_else(|| {
            ProviderError::RequestFailed("ACP session_id is required".to_string())
        })?;

        if let Some(acp_id) = self.goose_to_acp_id.lock().await.get(goose_id) {
            return Ok(acp_id.clone());
        }

        if self.sessions.lock().await.contains_key(goose_id) {
            return Ok(goose_id.to_string());
        }

        let (acp_id, _models) = self.new_session().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to create ACP session: {e}"))
        })?;

        self.goose_to_acp_id
            .lock()
            .await
            .insert(goose_id.to_string(), acp_id.0.to_string());

        Ok(acp_id.0.to_string())
    }

    async fn prompt(
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
}

#[async_trait::async_trait]
impl Provider for AcpProvider {
    fn get_name(&self) -> &str {
        self.name()
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model()
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn permission_routing(&self) -> PermissionRouting {
        AcpProvider::permission_routing(self)
    }

    async fn handle_permission_confirmation(
        &self,
        request_id: &str,
        confirmation: &PermissionConfirmation,
    ) -> bool {
        AcpProvider::handle_permission_confirmation(self, request_id, confirmation).await
    }

    async fn complete_with_model(
        &self,
        session_id: Option<&str>,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let session_id = self.ensure_session(session_id).await?;
        AcpProvider::complete_with_model(self, &session_id, model_config, system, messages, tools)
            .await
    }

    async fn stream(
        &self,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let session_id = self.ensure_session(Some(session_id)).await?;
        AcpProvider::stream(self, &session_id, system, messages, tools).await
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let (_session_id, models) = self.new_session().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to create ACP session for model list: {e}"))
        })?;
        Ok(models
            .map(|state| {
                state
                    .available_models
                    .iter()
                    .map(|m| m.model_id.0.to_string())
                    .collect()
            })
            .unwrap_or_default())
    }
}

impl Drop for AcpProvider {
    fn drop(&mut self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(ClientRequest::Shutdown).await;
        });
    }
}

async fn run_client_loop(
    config: AcpProviderConfig,
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

async fn spawn_acp_process(config: &AcpProviderConfig) -> Result<Child> {
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
    config: AcpProviderConfig,
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
    config: AcpProviderConfig,
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
    config: AcpProviderConfig,
    cx: JrConnectionCx<ClientToAgent>,
    rx: &mut mpsc::Receiver<ClientRequest>,
    prompt_response_tx: Arc<Mutex<Option<mpsc::Sender<AcpUpdate>>>>,
    init_tx: Arc<Mutex<Option<oneshot::Sender<Result<()>>>>>,
) -> Result<(), sacp::Error> {
    let init_response = cx
        .send_request(InitializeRequest::new(ProtocolVersion::LATEST))
        .block_task()
        .await
        .map_err(|err| {
            let message = format!("ACP initialize failed: {err}");
            send_init_result(&init_tx, Err(anyhow::anyhow!(message.clone())));
            sacp::Error::internal_error().data(message)
        })?;

    send_init_result(&init_tx, Ok(()));

    let mcp_capabilities = init_response.agent_capabilities.mcp_capabilities;

    while let Some(request) = rx.recv().await {
        match request {
            ClientRequest::NewSession { response_tx } => {
                handle_new_session_request(&config, &cx, &mcp_capabilities, response_tx).await;
            }
            ClientRequest::SetModel {
                session_id,
                model_id,
                response_tx,
            } => {
                let msg = sacp::UntypedMessage::new(
                    "session/set_model",
                    serde_json::json!({
                        "sessionId": session_id.0,
                        "modelId": model_id
                    }),
                )
                .unwrap();
                let result = cx
                    .send_request(msg)
                    .block_task()
                    .await
                    .map(|_| ())
                    .map_err(|e| anyhow::anyhow!("ACP session/set_model failed: {e}"));
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

async fn handle_new_session_request(
    config: &AcpProviderConfig,
    cx: &JrConnectionCx<ClientToAgent>,
    mcp_capabilities: &McpCapabilities,
    response_tx: oneshot::Sender<Result<(SessionId, Option<SessionModelState>)>>,
) {
    let mcp_servers = filter_supported_servers(&config.mcp_servers, mcp_capabilities);
    let session = cx
        .send_request(NewSessionRequest::new(config.work_dir.clone()).mcp_servers(mcp_servers))
        .block_task()
        .await;

    let result = match session {
        Ok(session) => apply_session_mode(config, cx, session).await,
        Err(err) => Err(anyhow::anyhow!("ACP session/new failed: {err}")),
    };

    let _ = response_tx.send(result);
}

async fn apply_session_mode(
    config: &AcpProviderConfig,
    cx: &JrConnectionCx<ClientToAgent>,
    session: NewSessionResponse,
) -> Result<(SessionId, Option<SessionModelState>)> {
    let session_id = session.session_id.clone();
    let models = session.models.clone();
    let mut result = Ok((session_id, models));

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

/// Converts extension configs to MCP servers at provider construction time.
///
/// This function handles the first stage of the MCP server pipeline:
/// 1. `ExtensionConfig` → `McpServer` conversion happens here during provider construction
/// 2. `filter_supported_servers()` filters by agent capabilities at session creation time
///
/// Skips SSE extensions (migrate to streamable_http) and unknown extension types.
pub fn extension_configs_to_mcp_servers(configs: &[ExtensionConfig]) -> Vec<McpServer> {
    let mut servers = Vec::new();

    for config in configs {
        match config {
            ExtensionConfig::StreamableHttp {
                name, uri, headers, ..
            } => {
                let http_headers = headers
                    .iter()
                    .map(|(key, value)| HttpHeader::new(key, value))
                    .collect();
                servers.push(McpServer::Http(
                    McpServerHttp::new(name, uri).headers(http_headers),
                ));
            }
            ExtensionConfig::Stdio {
                name,
                cmd,
                args,
                envs,
                ..
            } => {
                let env_vars = envs
                    .get_env()
                    .into_iter()
                    .map(|(key, value)| EnvVariable::new(key, value))
                    .collect();

                servers.push(McpServer::Stdio(
                    McpServerStdio::new(name, cmd)
                        .args(args.clone())
                        .env(env_vars),
                ));
            }
            ExtensionConfig::Sse { name, .. } => {
                tracing::debug!(name, "skipping SSE extension, migrate to streamable_http");
            }
            _ => {}
        }
    }

    servers
}

fn filter_supported_servers(
    servers: &[McpServer],
    capabilities: &McpCapabilities,
) -> Vec<McpServer> {
    servers
        .iter()
        .filter(|server| match server {
            McpServer::Http(http) => {
                if !capabilities.http {
                    tracing::debug!(
                        name = http.name,
                        "skipping HTTP server, agent lacks capability"
                    );
                    false
                } else {
                    true
                }
            }
            McpServer::Sse(sse) => {
                tracing::debug!(name = sse.name, "skipping SSE server, unsupported");
                false
            }
            _ => true,
        })
        .cloned()
        .collect()
}

fn send_init_result(init_tx: &Arc<Mutex<Option<oneshot::Sender<Result<()>>>>>, result: Result<()>) {
    if let Some(tx) = init_tx.lock().unwrap().take() {
        let _ = tx.send(result);
    }
}

async fn permission_response(
    mapping: &PermissionMapping,
    rejected_tool_calls: &Arc<TokioMutex<HashSet<String>>>,
    request: &RequestPermissionRequest,
    decision: PermissionDecision,
) -> RequestPermissionResponse {
    if decision.should_record_rejection() {
        rejected_tool_calls
            .lock()
            .await
            .insert(request.tool_call.tool_call_id.0.to_string());
    }

    map_permission_response(mapping, request, decision)
}

async fn tool_call_is_error(
    rejected_tool_calls: &Arc<TokioMutex<HashSet<String>>>,
    mapping: &PermissionMapping,
    tool_call_id: &str,
    status: ToolCallStatus,
) -> bool {
    let was_rejected = rejected_tool_calls.lock().await.remove(tool_call_id);

    match status {
        ToolCallStatus::Failed => true,
        ToolCallStatus::Completed => {
            was_rejected && mapping.rejected_tool_status == ToolCallStatus::Completed
        }
        _ => false,
    }
}

fn text_content(text: impl Into<String>) -> ContentBlock {
    ContentBlock::Text(TextContent::new(text))
}

fn messages_to_prompt(messages: &[Message]) -> Vec<ContentBlock> {
    let mut content_blocks = Vec::new();

    let last_user = messages
        .iter()
        .rev()
        .find(|m| m.role == Role::User && m.is_agent_visible());

    if let Some(message) = last_user {
        for content in &message.content {
            if let MessageContent::Text(text) = content {
                content_blocks.push(text_content(text.text.clone()));
            }
        }
    }

    content_blocks
}

fn build_action_required_message(request: &RequestPermissionRequest) -> Option<Message> {
    let tool_title = request
        .tool_call
        .fields
        .title
        .clone()
        .unwrap_or_else(|| "Tool".to_string());

    let arguments = request
        .tool_call
        .fields
        .raw_input
        .as_ref()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    let prompt = request
        .tool_call
        .fields
        .content
        .as_ref()
        .and_then(|content| {
            content.iter().find_map(|c| match c {
                ToolCallContent::Content(val) => match &val.content {
                    ContentBlock::Text(text) => Some(text.text.clone()),
                    _ => None,
                },
                _ => None,
            })
        });

    Some(
        Message::assistant()
            .with_action_required(
                request.tool_call.tool_call_id.0.to_string(),
                tool_title,
                arguments,
                prompt,
            )
            .user_only(),
    )
}

fn permission_decision_from_confirmation(
    confirmation: &PermissionConfirmation,
) -> PermissionDecision {
    match confirmation.permission {
        Permission::AlwaysAllow => PermissionDecision::AllowAlways,
        Permission::AllowOnce => PermissionDecision::AllowOnce,
        Permission::DenyOnce => PermissionDecision::RejectOnce,
        Permission::AlwaysDeny => PermissionDecision::RejectAlways,
        Permission::Cancel => PermissionDecision::Cancel,
    }
}

fn permission_decision_from_mode(goose_mode: GooseMode) -> Option<PermissionDecision> {
    match goose_mode {
        GooseMode::Auto => Some(PermissionDecision::AllowOnce),
        GooseMode::Chat => Some(PermissionDecision::RejectOnce),
        GooseMode::Approve | GooseMode::SmartApprove => None,
    }
}

fn tool_call_content_to_text(content: &[ToolCallContent]) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            ToolCallContent::Content(val) => match &val.content {
                ContentBlock::Text(text) => Some(text.text.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn content_blocks_to_rmcp(content: &[ToolCallContent]) -> Vec<Content> {
    content
        .iter()
        .filter_map(|c| match c {
            ToolCallContent::Content(val) => match &val.content {
                ContentBlock::Text(text) => Some(Content::text(text.text.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn classify_error(err: ProviderError) -> ProviderError {
    let msg = err.to_string();
    if msg.contains("context window")
        || msg.contains("too long")
        || msg.contains("too many tokens")
    {
        return ProviderError::ContextLengthExceeded(msg);
    }
    err
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::extension::Envs;
    use test_case::test_case;

    #[test_case(
        ExtensionConfig::Stdio {
            name: "github".into(),
            description: String::new(),
            cmd: "/path/to/github-mcp-server".into(),
            args: vec!["stdio".into()],
            envs: Envs::new([("GITHUB_PERSONAL_ACCESS_TOKEN".into(), "ghp_xxxxxxxxxxxx".into())].into()),
            env_keys: vec![],
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        },
        vec![
            McpServer::Stdio(
                McpServerStdio::new("github", "/path/to/github-mcp-server")
                    .args(vec!["stdio".into()])
                    .env(vec![EnvVariable::new("GITHUB_PERSONAL_ACCESS_TOKEN", "ghp_xxxxxxxxxxxx")])
            )
        ]
        ; "stdio_converts_to_mcpserver_stdio"
    )]
    #[test_case(
        ExtensionConfig::StreamableHttp {
            name: "github".into(),
            description: String::new(),
            uri: "https://api.githubcopilot.com/mcp/".into(),
            envs: Envs::default(),
            env_keys: vec![],
            headers: HashMap::from([("Authorization".into(), "Bearer ghp_xxxxxxxxxxxx".into())]),
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        },
        vec![
            McpServer::Http(
                McpServerHttp::new("github", "https://api.githubcopilot.com/mcp/")
                    .headers(vec![HttpHeader::new("Authorization", "Bearer ghp_xxxxxxxxxxxx")])
            )
        ]
        ; "streamable_http_converts_to_mcpserver_http_when_capable"
    )]
    fn test_extension_configs_to_mcp_servers(config: ExtensionConfig, expected: Vec<McpServer>) {
        let result = extension_configs_to_mcp_servers(&[config]);
        assert_eq!(result.len(), expected.len(), "server count mismatch");
        for (a, e) in result.iter().zip(expected.iter()) {
            match (a, e) {
                (McpServer::Stdio(actual), McpServer::Stdio(expected)) => {
                    assert_eq!(actual.name, expected.name);
                    assert_eq!(actual.command, expected.command);
                    assert_eq!(actual.args, expected.args);
                    assert_eq!(actual.env.len(), expected.env.len());
                }
                (McpServer::Http(actual), McpServer::Http(expected)) => {
                    assert_eq!(actual.name, expected.name);
                    assert_eq!(actual.url, expected.url);
                    assert_eq!(actual.headers.len(), expected.headers.len());
                }
                _ => panic!("server type mismatch"),
            }
        }
    }

    #[test]
    fn test_sse_skips() {
        let config = ExtensionConfig::Sse {
            name: "test-sse".into(),
            description: String::new(),
            uri: Some("https://example.com/sse".into()),
        };
        let result = extension_configs_to_mcp_servers(&[config]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_supported_servers_skips_http_without_capability() {
        let config = ExtensionConfig::StreamableHttp {
            name: "github".into(),
            description: String::new(),
            uri: "https://api.githubcopilot.com/mcp/".into(),
            envs: Envs::default(),
            env_keys: vec![],
            headers: HashMap::from([("Authorization".into(), "Bearer ghp_xxxxxxxxxxxx".into())]),
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        };

        let servers = extension_configs_to_mcp_servers(&[config]);
        let filtered = filter_supported_servers(&servers, &McpCapabilities::default());
        assert!(filtered.is_empty());
    }
}
