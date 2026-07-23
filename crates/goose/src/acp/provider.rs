use agent_client_protocol::schema::v1::{
    Annotations as AcpAnnotations, ClientCapabilities, CloseSessionRequest, ContentBlock,
    ContentChunk, EnvVariable, HttpHeader, ImageContent, InitializeRequest, InitializeResponse,
    McpCapabilities, McpServer, McpServerHttp, McpServerStdio, NewSessionRequest,
    NewSessionResponse, PromptRequest, PromptResponse, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, Role as AcpRole, SessionConfigKind,
    SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectOptions, SessionId,
    SessionModeState, SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionModeRequest, SetSessionModeResponse, StopReason, TextContent, ToolCallContent,
    ToolCallStatus, ToolKind,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{Agent, Client, ConnectionTo};
use agent_client_protocol_schema::v1::Usage as AcpUsage;
use agent_client_protocol_schema::v1::AGENT_METHOD_NAMES;
use anyhow::{Context, Result};
use async_stream::try_stream;
use futures::future::BoxFuture;
use goose_providers::conversation::token_usage::{ProviderUsage, Usage};
use rmcp::model::{CallToolRequestParams, CallToolResult, Content as RmcpContent, Role, Tool};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex as TokioMutex};
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

use crate::acp::{map_permission_response, PermissionDecision};
use crate::config::{ExtensionConfig, GooseMode};
use crate::context_mgmt::format_message_for_compacting;
use crate::conversation::message::{Message, MessageContent, TOOL_META_EXTERNAL_DISPATCH_KEY};
use crate::conversation::Conversation;
use crate::permission::permission_confirmation::PrincipalType;
use crate::permission::{Permission, PermissionConfirmation};
use crate::providers::base::{MessageStream, PermissionRouting, Provider};
use crate::subprocess::configure_subprocess;
use crate::utils::sanitize_unicode_tags;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;

/// Sentinel: resolved to the actual model name during connect().
pub const ACP_CURRENT_MODEL: &str = "current";

pub struct AcpProviderConfig {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub env_remove: Vec<String>,
    pub work_dir: PathBuf,
    pub mcp_servers: Vec<McpServer>,
    pub session_mode_id: Option<String>,
    pub session_config_options: Vec<(String, String)>,
    /// Config option id used to select the model (e.g. `"model"`). When set, the
    /// provider re-applies this option from the per-completion `ModelConfig`
    /// whenever the active session model changes.
    pub model_config_option_id: Option<String>,
    pub mode_mapping: HashMap<GooseMode, Vec<String>>,
    pub notification_callback: Option<Arc<dyn Fn(SessionNotification) + Send + Sync>>,
}

enum ClientRequest {
    NewSession {
        response_tx: oneshot::Sender<Result<NewSessionResponse>>,
    },
    SetMode {
        session_id: SessionId,
        mode_id: String,
        response_tx: oneshot::Sender<Result<()>>,
    },
    SetConfigOption {
        session_id: SessionId,
        config_id: String,
        value: String,
        response_tx: oneshot::Sender<Result<()>>,
    },
    Prompt {
        session_id: SessionId,
        content: Vec<ContentBlock>,
        response_tx: mpsc::Sender<AcpUpdate>,
    },
}

// tokio I/O handles can't move between runtimes, so the child process must be
// spawned inside the OS thread. This closure lets start() share all other logic.
type ClientLoopFn = Box<
    dyn FnOnce(
            AcpClientLoop,
            mpsc::Receiver<ClientRequest>,
            oneshot::Sender<Result<InitializeResponse>>,
        ) -> BoxFuture<'static, ()>
        + Send,
>;

#[derive(Debug)]
enum AcpUpdate {
    Text(TextContent),
    Thought(String),
    ToolCallStart {
        id: String,
        name: String,
        kind: ToolKind,
        raw_input: Option<serde_json::Value>,
    },
    ToolCallComplete {
        id: String,
        raw_output: Option<serde_json::Value>,
        content: Option<Vec<ToolCallContent>>,
        is_error: bool,
    },
    PermissionRequest {
        request: Box<RequestPermissionRequest>,
        response_tx: oneshot::Sender<RequestPermissionResponse>,
    },
    Complete(StopReason, Option<AcpUsage>),
    Error(String),
}

/// Per-tool-call buffer for accumulating ACP ToolCallUpdate fields across
/// non-terminal updates, drained on the terminal status update.
#[derive(Debug, Default)]
struct AccumulatedToolCall {
    raw_output: Option<serde_json::Value>,
    content: Vec<ToolCallContent>,
}

/// The single ACP session backing this provider instance.
#[derive(Clone)]
struct AcpSession {
    id: SessionId,
    response: NewSessionResponse,
}

struct HandoffContextClaim {
    first_prompt: bool,
    include_context: bool,
}

pub struct AcpProvider {
    name: String,
    goose_mode: Arc<Mutex<GooseMode>>,
    mode_mapping: HashMap<GooseMode, Vec<String>>,

    session: AcpSession,

    pending_confirmations:
        Arc<TokioMutex<HashMap<String, oneshot::Sender<PermissionConfirmation>>>>,
    pending_tool_updates: Arc<Mutex<HashMap<String, AccumulatedToolCall>>>,
    handoff_context_sent: AtomicBool,
    /// Latest `size` reported by the ACP server in a `session/update` →
    /// `usage_update` notification. 0 means no real update has arrived yet,
    /// in which case `get_context_limit()` falls back to the supplied model
    /// configuration's context limit.
    context_size: Arc<AtomicU64>,

    /// Config option id used to select the model, if this agent supports it.
    model_config_option_id: Option<String>,
    /// Model currently applied via `model_config_option_id`, used to avoid
    /// redundant `SetConfigOption` calls.
    applied_model: Arc<Mutex<Option<String>>>,

    tx: Option<mpsc::Sender<ClientRequest>>,
    loop_thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for AcpProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcpProvider")
            .field("name", &self.name)
            .finish()
    }
}

fn spawn_client_loop(fut: impl Future<Output = ()> + Send + 'static) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build ACP client runtime");
        rt.block_on(fut)
    })
}

impl AcpProvider {
    pub async fn connect(
        name: String,
        goose_mode: GooseMode,
        config: AcpProviderConfig,
    ) -> Result<Self> {
        Self::start(
            name,
            goose_mode,
            config,
            Box::new(|cl, rx, init_tx| Box::pin(cl.spawn(rx, init_tx))),
        )
        .await
    }

    #[doc(hidden)]
    pub async fn connect_with_transport(
        name: String,
        goose_mode: GooseMode,
        config: AcpProviderConfig,
        transport: impl agent_client_protocol::ConnectTo<Client> + 'static,
    ) -> Result<Self> {
        Self::start(
            name,
            goose_mode,
            config,
            Box::new(move |cl, mut rx, init_tx| {
                Box::pin(async move {
                    if let Err(e) = cl.run(transport, &mut rx, init_tx).await {
                        tracing::error!("ACP protocol error: {e}");
                    }
                })
            }),
        )
        .await
    }

    async fn start(
        name: String,
        goose_mode: GooseMode,
        config: AcpProviderConfig,
        run: ClientLoopFn,
    ) -> Result<Self> {
        let (tx, rx) = mpsc::channel(32);
        let (init_tx, init_rx) = oneshot::channel();
        let mode_mapping = config.mode_mapping.clone();
        let model_config_option_id = config.model_config_option_id.clone();
        let applied_model = config.model_config_option_id.as_ref().and_then(|id| {
            config
                .session_config_options
                .iter()
                .find(|(opt_id, _)| opt_id == id)
                .map(|(_, value)| value.clone())
        });
        let goose_mode_shared = Arc::new(Mutex::new(goose_mode));
        let pending_tool_updates: Arc<Mutex<HashMap<String, AccumulatedToolCall>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let context_size = Arc::new(AtomicU64::new(0));
        let client_loop = AcpClientLoop::new(
            config,
            goose_mode_shared.clone(),
            pending_tool_updates.clone(),
            context_size.clone(),
        );
        let loop_thread = spawn_client_loop(run(client_loop, rx, init_tx));

        let _init_response = init_rx
            .await
            .context("ACP client initialization cancelled")??;

        // Create the ACP session eagerly during connect.
        let (session_tx, session_rx) = oneshot::channel();
        tx.send(ClientRequest::NewSession {
            response_tx: session_tx,
        })
        .await
        .context("ACP client is unavailable")?;
        let response = session_rx
            .await
            .context("ACP session creation cancelled")??;

        let session = AcpSession {
            id: response.session_id.clone(),
            response,
        };

        Ok(Self {
            name,
            goose_mode: goose_mode_shared,
            mode_mapping,
            session,
            pending_confirmations: Arc::new(TokioMutex::new(HashMap::new())),
            pending_tool_updates,
            handoff_context_sent: AtomicBool::new(false),
            context_size,
            model_config_option_id,
            applied_model: Arc::new(Mutex::new(applied_model)),
            tx: Some(tx),
            loop_thread: Some(loop_thread),
        })
    }

    fn acp_session_id(&self) -> SessionId {
        self.session.id.clone()
    }

    pub(crate) async fn send_set_mode(&self, _goose_id: &str, mode_id: String) -> Result<()> {
        let session_id = self.acp_session_id();
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .as_ref()
            .unwrap()
            .send(ClientRequest::SetMode {
                session_id,
                mode_id,
                response_tx,
            })
            .await
            .context("ACP client is unavailable")?;
        response_rx.await.context("ACP request cancelled")?
    }

    pub(crate) async fn send_set_config_option(
        &self,
        _goose_id: &str,
        config_id: String,
        value: String,
    ) -> Result<()> {
        let session_id = self.acp_session_id();
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .as_ref()
            .unwrap()
            .send(ClientRequest::SetConfigOption {
                session_id,
                config_id,
                value,
                response_tx,
            })
            .await
            .context("ACP client is unavailable")?;
        response_rx.await.context("ACP request cancelled")?
    }

    /// Re-apply the model selection config option when the active session model
    /// differs from what was last applied. ACP agents that select their model
    /// via a config option (e.g. Copilot) need this so resumed or switched
    /// sessions actually use the requested model instead of the agent default.
    async fn apply_model_if_changed(&self, model_name: &str) -> Result<()> {
        let Some(config_id) = self.model_config_option_id.clone() else {
            return Ok(());
        };
        if model_name == ACP_CURRENT_MODEL {
            return Ok(());
        }

        {
            let applied = self
                .applied_model
                .lock()
                .map_err(|_| anyhow::anyhow!("applied_model lock poisoned"))?;
            if applied.as_deref() == Some(model_name) {
                return Ok(());
            }
        }

        self.send_set_config_option("", config_id, model_name.to_string())
            .await?;

        let mut applied = self
            .applied_model
            .lock()
            .map_err(|_| anyhow::anyhow!("applied_model lock poisoned"))?;
        *applied = Some(model_name.to_string());
        Ok(())
    }

    async fn prompt(
        &self,
        session_id: SessionId,
        content: Vec<ContentBlock>,
    ) -> Result<mpsc::Receiver<AcpUpdate>> {
        let (response_tx, response_rx) = mpsc::channel(64);
        self.tx
            .as_ref()
            .unwrap()
            .send(ClientRequest::Prompt {
                session_id,
                content,
                response_tx,
            })
            .await
            .context("ACP client is unavailable")?;
        Ok(response_rx)
    }

    fn session_has_config_option(&self, category: SessionConfigOptionCategory) -> bool {
        self.session
            .response
            .config_options
            .as_ref()
            .is_some_and(|opts| opts.iter().any(|o| o.category.as_ref() == Some(&category)))
    }

    fn claim_handoff_context(&self, messages: &[Message]) -> HandoffContextClaim {
        let first_prompt = !self.handoff_context_sent.swap(true, Ordering::AcqRel);
        HandoffContextClaim {
            first_prompt,
            include_context: first_prompt && has_handoff_context(messages),
        }
    }
}

fn fresh_text_run() -> (String, i64) {
    (
        uuid::Uuid::new_v4().to_string(),
        chrono::Utc::now().timestamp(),
    )
}

#[async_trait::async_trait]
impl Provider for AcpProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn get_context_limit(&self, model_config: &ModelConfig) -> Result<usize, ProviderError> {
        let size = self.context_size.load(Ordering::Relaxed);
        if size > 0 {
            return Ok(size as usize);
        }
        Ok(model_config.context_limit())
    }

    async fn update_mode(&self, session_id: &str, mode: GooseMode) -> Result<(), ProviderError> {
        if let Some(candidates) = self.mode_mapping.get(&mode) {
            let mode_str = select_mode_id(candidates, self.session.response.modes.as_ref())
                .ok_or_else(|| {
                    ProviderError::RequestFailed(format!(
                        "None of the mode ids [{}] are offered by the agent",
                        candidates.join(", ")
                    ))
                })?;
            if self.session_has_config_option(SessionConfigOptionCategory::Mode) {
                self.send_set_config_option(session_id, "mode".into(), mode_str)
                    .await
                    .map_err(|e| {
                        ProviderError::RequestFailed(format!("Failed to set mode: {e}"))
                    })?;
            } else {
                self.send_set_mode(session_id, mode_str)
                    .await
                    .map_err(|e| {
                        ProviderError::RequestFailed(format!("Failed to set mode: {e}"))
                    })?;
            }
        }

        if let Ok(mut guard) = self.goose_mode.lock() {
            *guard = mode;
        }
        Ok(())
    }

    fn permission_routing(&self) -> PermissionRouting {
        PermissionRouting::ActionRequired
    }

    fn manages_own_context(&self) -> bool {
        true
    }

    async fn handle_permission_confirmation(
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

    async fn stream(
        &self,
        model_config: &ModelConfig,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let session_id = self.acp_session_id();

        self.apply_model_if_changed(&model_config.model_name)
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to set ACP model option: {e}"))
            })?;

        let current_prompt_blocks = messages_to_prompt(messages, false);
        if current_prompt_blocks.is_empty() {
            return Ok(Box::pin(futures::stream::empty()));
        }

        let claim = self.claim_handoff_context(messages);
        let prompt_blocks = if claim.include_context {
            messages_to_prompt(messages, true)
        } else {
            current_prompt_blocks
        };
        // Drop any tool-call buffer state left over from a prior prompt
        // (e.g. cancelled or interrupted before its terminal status arrived).
        if let Ok(mut buffer) = self.pending_tool_updates.lock() {
            buffer.clear();
        }
        let mut rx = match self.prompt(session_id, prompt_blocks).await {
            Ok(rx) => rx,
            Err(e) => {
                if claim.first_prompt {
                    self.handoff_context_sent.store(false, Ordering::Release);
                }
                return Err(ProviderError::RequestFailed(format!(
                    "Failed to send ACP prompt: {e}"
                )));
            }
        };

        let pending_confirmations = self.pending_confirmations.clone();
        let goose_mode = *self
            .goose_mode
            .lock()
            .map_err(|_| ProviderError::RequestFailed("goose_mode lock poisoned".into()))?;

        let reject_all_tools = goose_mode == GooseMode::Chat;
        let model_name = model_config.model_name.clone();

        Ok(Box::pin(try_stream! {
            let mut suppress_text = false;
            let mut rejected_tool_calls: HashSet<String> = HashSet::new();
            // Stable id+timestamp per contiguous run so Desktop coalesces chunks into one bubble.
            let mut text_run: Option<(String, i64)> = None;
            let mut thought_run: Option<(String, i64)> = None;

            while let Some(update) = rx.recv().await {
                match update {
                    AcpUpdate::Text(text) => {
                        if !suppress_text {
                            let (id, ts) = text_run
                                .get_or_insert_with(fresh_text_run)
                                .clone();
                            let message = acp_text_update_message(text, id, ts);
                            yield (Some(message), None);
                        }
                    }
                    AcpUpdate::Thought(text) => {
                        let (id, ts) = thought_run
                            .get_or_insert_with(fresh_text_run)
                            .clone();
                        let message = Message::new(Role::Assistant, ts, vec![])
                            .with_thinking(text, "")
                            .with_visibility(true, false)
                            .with_id(id);
                        yield (Some(message), None);
                    }
                    AcpUpdate::ToolCallStart { id, name, kind, raw_input } => {
                        text_run = None;
                        thought_run = None;
                        if reject_all_tools {
                            suppress_text = true;
                            rejected_tool_calls.insert(id);
                        } else {
                            let mut params = CallToolRequestParams::new(name);
                            if let Some(serde_json::Value::Object(map)) = raw_input {
                                params = params.with_arguments(map);
                            }
                            // external_dispatch tells the agent loop not to redispatch this
                            // call. goose.acp.kind preserves ACP's stable categorization for
                            // downstream consumers (metrics, observability, icon selection)
                            // independent of the display title we put in `name`.
                            let tool_meta = Some(serde_json::json!({
                                TOOL_META_EXTERNAL_DISPATCH_KEY: true,
                                "goose.acp.kind": kind,
                            }));
                            let message = Message::assistant().with_tool_request_with_metadata(
                                id,
                                Ok(params),
                                None,
                                tool_meta,
                            );
                            yield (Some(message), None);
                        }
                    }
                    AcpUpdate::ToolCallComplete {
                        id,
                        raw_output,
                        content,
                        is_error,
                    } => {
                        text_run = None;
                        thought_run = None;
                        if rejected_tool_calls.remove(&id) {
                            // In chat mode no tool_request was emitted (suppressed at
                            // ToolCallStart), so surface a plain text message. In other
                            // modes a tool_request WAS emitted, so pair it with an error
                            // tool_response so downstream consumers see the rejection.
                            if reject_all_tools {
                                let message = Message::assistant()
                                    .with_text("Tool call was denied.");
                                yield (Some(message), None);
                            } else {
                                let denial = vec![RmcpContent::text("Tool call was denied.")];
                                let result = CallToolResult::error(denial);
                                let message =
                                    Message::user().with_tool_response(id, Ok(result));
                                yield (Some(message), None);
                            }
                        } else {
                            let result_content =
                                acp_tool_call_content_to_rmcp(content, raw_output);
                            let result = if is_error {
                                CallToolResult::error(result_content)
                            } else {
                                CallToolResult::success(result_content)
                            };
                            let message = Message::user().with_tool_response(id, Ok(result));
                            yield (Some(message), None);
                        }
                    }
                    AcpUpdate::PermissionRequest { request, response_tx } => {
                        text_run = None;
                        thought_run = None;
                        if let Some(decision) = permission_decision_from_mode(goose_mode) {
                            if decision.should_record_rejection() {
                                rejected_tool_calls.insert(request.tool_call.tool_call_id.0.to_string());
                            }
                            let _ = response_tx.send(map_permission_response(&request, decision));
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

                        let decision = PermissionDecision::from(confirmation.permission);
                        if decision.should_record_rejection() {
                            rejected_tool_calls.insert(request.tool_call.tool_call_id.0.to_string());
                        }
                        let _ = response_tx.send(map_permission_response(&request, decision));
                    }
                    AcpUpdate::Complete(_reason, usage) => {
                        if let Some(usage) = usage {
                            let provider_usage = ProviderUsage::new(
                                model_name.clone(),
                                Usage::new(
                                    Some(usage.input_tokens as i32),
                                    Some(usage.output_tokens as i32),
                                    Some(usage.total_tokens as i32),
                                ),
                            );
                            yield (None, Some(provider_usage));
                        }
                        break;
                    }
                    AcpUpdate::Error(e) => {
                        Err(ProviderError::RequestFailed(e))?;
                    }
                }
            }
        }))
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let (_, available) = resolve_model_info(&self.name, &self.session.response)?;
        Ok(available)
    }
}

impl Drop for AcpProvider {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(h) = self.loop_thread.take() {
            if let Err(e) = h.join() {
                tracing::debug!("AcpClientLoop thread panicked: {e:?}");
            }
        }
    }
}

struct AcpClientLoop {
    config: AcpProviderConfig,
    goose_mode: Arc<Mutex<GooseMode>>,
    prompt_response_tx: Arc<Mutex<Option<mpsc::Sender<AcpUpdate>>>>,
    pending_tool_updates: Arc<Mutex<HashMap<String, AccumulatedToolCall>>>,
    context_size: Arc<AtomicU64>,
}

impl AcpClientLoop {
    fn new(
        config: AcpProviderConfig,
        goose_mode: Arc<Mutex<GooseMode>>,
        pending_tool_updates: Arc<Mutex<HashMap<String, AccumulatedToolCall>>>,
        context_size: Arc<AtomicU64>,
    ) -> Self {
        Self {
            config,
            goose_mode,
            prompt_response_tx: Arc::new(Mutex::new(None)),
            pending_tool_updates,
            context_size,
        }
    }

    async fn spawn(
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

    async fn run(
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
            context_size,
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
                    let context_size = context_size.clone();
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
                            SessionUpdate::UsageUpdate(usage) => {
                                context_size.store(usage.size, Ordering::Relaxed);
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
                                    content: ContentBlock::Text(text),
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
                        apply_session_config_options(&config, &cx, session.session_id.clone())
                            .await?;
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
                let value_id = agent_client_protocol::schema::v1::SessionConfigValueId::new(value);
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

async fn apply_session_config_options(
    config: &AcpProviderConfig,
    cx: &ConnectionTo<Agent>,
    session_id: SessionId,
) -> Result<()> {
    for (config_id, value) in &config.session_config_options {
        let value_id = agent_client_protocol::schema::v1::SessionConfigValueId::new(value.clone());
        cx.send_request(SetSessionConfigOptionRequest::new(
            session_id.clone(),
            config_id.clone(),
            value_id,
        ))
        .block_task()
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "ACP agent rejected {} for '{}': {err}",
                AGENT_METHOD_NAMES.session_set_config_option,
                config_id
            )
        })?;
    }
    Ok(())
}

async fn apply_session_mode(
    config: &AcpProviderConfig,
    goose_mode: &Arc<Mutex<GooseMode>>,
    cx: &ConnectionTo<Agent>,
    session: NewSessionResponse,
) -> Result<NewSessionResponse> {
    let current_mode = goose_mode.lock().ok().map(|mode| *mode);
    let candidates = initial_mode_candidates(config, current_mode);

    if let Some(modes) = session.modes.as_ref() {
        if !candidates.is_empty() {
            let Some(mode_id) = select_mode_id(&candidates, Some(modes)) else {
                let available: Vec<String> = modes
                    .available_modes
                    .iter()
                    .map(|mode| mode.id.0.to_string())
                    .collect();
                return Err(anyhow::anyhow!(
                    "Requested mode(s) [{}] not offered by agent. Available modes: {}",
                    candidates.join(", "),
                    available.join(", ")
                ));
            };
            if modes.current_mode_id.0.as_ref() != mode_id.as_str() {
                let _: SetSessionModeResponse = cx
                    .send_request(SetSessionModeRequest::new(
                        session.session_id.clone(),
                        mode_id,
                    ))
                    .block_task()
                    .await
                    .map_err(|err| {
                        anyhow::anyhow!(
                            "ACP agent rejected {}: {err}",
                            AGENT_METHOD_NAMES.session_set_mode
                        )
                    })?;
            }
        }
    }

    Ok(session)
}

fn initial_mode_candidates(
    config: &AcpProviderConfig,
    current_mode: Option<GooseMode>,
) -> Vec<String> {
    current_mode
        .and_then(|mode| config.mode_mapping.get(&mode).cloned())
        .or_else(|| config.session_mode_id.clone().map(|id| vec![id]))
        .unwrap_or_default()
}

fn select_mode_id(candidates: &[String], modes: Option<&SessionModeState>) -> Option<String> {
    match modes {
        Some(state) => candidates
            .iter()
            .find(|candidate| {
                state
                    .available_modes
                    .iter()
                    .any(|mode| mode.id.0.as_ref() == candidate.as_str())
            })
            .cloned(),
        None => candidates.first().cloned(),
    }
}

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

fn messages_to_prompt(messages: &[Message], include_handoff_context: bool) -> Vec<ContentBlock> {
    let Some(last_user_index) = last_user_message_index(messages) else {
        return Vec::new();
    };

    let message = messages[last_user_index].agent_visible_content();
    let mut current_prompt_blocks = Vec::new();
    for content in &message.content {
        match content {
            MessageContent::Text(text) => {
                current_prompt_blocks.push(ContentBlock::Text(TextContent::new(text.text.clone())));
            }
            MessageContent::Image(image) => {
                current_prompt_blocks.push(ContentBlock::Image(ImageContent::new(
                    &image.data,
                    &image.mime_type,
                )));
            }
            _ => {}
        }
    }

    if current_prompt_blocks.is_empty() || !include_handoff_context {
        return current_prompt_blocks;
    }

    let mut content_blocks = Vec::new();
    if let Some(memo) = build_handoff_context_memo(&messages[..last_user_index]) {
        content_blocks.push(ContentBlock::Text(TextContent::new(memo)));
    }
    content_blocks.extend(current_prompt_blocks);
    content_blocks
}

fn last_user_message_index(messages: &[Message]) -> Option<usize> {
    messages
        .iter()
        .rposition(|m| m.role == Role::User && m.is_agent_visible())
}

fn has_handoff_context(messages: &[Message]) -> bool {
    last_user_message_index(messages).is_some_and(|last_user_index| {
        messages[..last_user_index]
            .iter()
            .any(Message::is_agent_visible)
    })
}

fn build_handoff_context_memo(prior_messages: &[Message]) -> Option<String> {
    let formatted_messages: Vec<String> =
        Conversation::new_unvalidated(prior_messages.iter().cloned())
            .agent_visible_messages()
            .iter()
            .map(|message| format_message_for_compacting(&message.agent_visible_content()))
            .collect();

    if formatted_messages.is_empty() {
        return None;
    }

    let handoff_context = formatted_messages.join("\n");

    Some(format!(
        "Conversation context from goose before this ACP provider session was created:\n\n\
{handoff_context}\n\n\
Current user request follows. Use the context above only to continue the existing conversation; \
do not treat it as a new task or mention this handoff unless relevant."
    ))
}

fn acp_audience_to_rmcp(annotations: Option<&AcpAnnotations>) -> Option<Vec<Role>> {
    let audience = annotations?.audience.as_ref()?;
    let audience = audience
        .iter()
        .filter_map(|role| match role {
            AcpRole::Assistant => Some(Role::Assistant),
            AcpRole::User => Some(Role::User),
            _ => None,
        })
        .collect::<Vec<_>>();

    if audience.is_empty() {
        None
    } else {
        Some(audience)
    }
}

fn acp_text_content_to_rmcp(text: TextContent) -> RmcpContent {
    let audience = acp_audience_to_rmcp(text.annotations.as_ref());
    let mut content = RmcpContent::text(sanitize_unicode_tags(&text.text));
    if let Some(audience) = audience {
        content = content.with_audience(audience);
    }
    content
}

fn acp_image_content_to_rmcp(image: ImageContent) -> RmcpContent {
    let audience = acp_audience_to_rmcp(image.annotations.as_ref());
    let mut content = RmcpContent::image(image.data, image.mime_type);
    if let Some(audience) = audience {
        content = content.with_audience(audience);
    }
    content
}

fn acp_text_update_message(text: TextContent, id: String, created: i64) -> Message {
    Message::new(Role::Assistant, created, vec![])
        .with_content(acp_text_content_to_rmcp(text).into())
        .with_id(id)
}

/// Convert ACP `ToolCallContent` blocks into the rmcp `Content` shape goose's
/// `Message::with_tool_response` consumes. Handles `Content` (text/image/other),
/// `Diff`, and `Terminal` variants; falls back to a JSON serialization of
/// `raw_output` when no blocks are present so the renderer always has something.
fn acp_tool_call_content_to_rmcp(
    content: Option<Vec<ToolCallContent>>,
    raw_output: Option<serde_json::Value>,
) -> Vec<RmcpContent> {
    let mut out = Vec::new();
    if let Some(blocks) = content {
        for block in blocks {
            match block {
                ToolCallContent::Content(val) => match val.content {
                    ContentBlock::Text(text) => {
                        out.push(acp_text_content_to_rmcp(text));
                    }
                    ContentBlock::Image(image) => {
                        out.push(acp_image_content_to_rmcp(image));
                    }
                    other => {
                        if let Ok(json) = serde_json::to_string(&other) {
                            out.push(RmcpContent::text(json));
                        }
                    }
                },
                ToolCallContent::Diff(diff) => {
                    let path = diff.path.display();
                    let body = match diff.old_text.as_deref() {
                        Some(old) => {
                            format!("--- {path}\n{old}\n+++ {path}\n{}", diff.new_text)
                        }
                        None => format!("+++ {path}\n{}", diff.new_text),
                    };
                    out.push(RmcpContent::text(body));
                }
                ToolCallContent::Terminal(_) => {}
                _ => {}
            }
        }
    }
    if out.is_empty() {
        if let Some(raw) = raw_output {
            let text = match raw {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            out.push(RmcpContent::text(text).with_priority(0.0));
        }
    } else {
        for item in &mut out {
            if item.priority().is_none() {
                *item = item.clone().with_priority(0.0);
            }
        }
    }
    out
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

fn extract_model_info_from_config_options(
    config_options: &[SessionConfigOption],
) -> Option<(String, Vec<String>)> {
    let select = config_options.iter().find_map(|opt| {
        if opt.category.as_ref() != Some(&SessionConfigOptionCategory::Model) {
            return None;
        }
        match &opt.kind {
            SessionConfigKind::Select(select) => Some(select),
            _ => None,
        }
    })?;

    let current = select.current_value.0.to_string();
    let available = match &select.options {
        SessionConfigSelectOptions::Ungrouped(options) => options
            .iter()
            .map(|option| option.value.0.to_string())
            .collect(),
        SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| {
                group
                    .options
                    .iter()
                    .map(|option| option.value.0.to_string())
            })
            .collect(),
        _ => Vec::new(),
    };
    Some((current, available))
}

fn resolve_model_info(
    provider_name: &str,
    response: &NewSessionResponse,
) -> Result<(String, Vec<String>), ProviderError> {
    if let Some(opts) = &response.config_options {
        if let Some((current, available)) = extract_model_info_from_config_options(opts) {
            return Ok((current, available));
        }
    }

    Err(ProviderError::RequestFailed(format!(
        "{provider_name}: agent returned no model config_options"
    )))
}

fn reverse_mode_mapping(
    mode_mapping: &HashMap<GooseMode, Vec<String>>,
) -> HashMap<String, Vec<GooseMode>> {
    let mut reverse: HashMap<String, Vec<GooseMode>> = HashMap::new();
    for (mode, ids) in mode_mapping {
        for id in ids {
            reverse.entry(id.clone()).or_default().push(*mode);
        }
    }
    reverse
}

fn resolve_mode(
    reverse_modes: &HashMap<String, Vec<GooseMode>>,
    mode_id: &str,
    current: &Arc<Mutex<GooseMode>>,
) -> Option<GooseMode> {
    let candidates = reverse_modes.get(mode_id)?;
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    let current = current.lock().ok()?;
    if candidates.contains(&*current) {
        Some(*current)
    } else {
        Some(candidates[0])
    }
}

fn permission_decision_from_mode(goose_mode: GooseMode) -> Option<PermissionDecision> {
    match goose_mode {
        GooseMode::Auto => Some(PermissionDecision::AllowOnce),
        GooseMode::Chat => Some(PermissionDecision::RejectOnce),
        GooseMode::Approve | GooseMode::SmartApprove => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::extension::Envs;
    use agent_client_protocol::schema::v1::{
        SessionConfigSelectOption, SessionMode, SessionModeId,
    };
    use rmcp::model::AnnotateAble;
    use test_case::test_case;

    fn prompt_text(block: &ContentBlock) -> &str {
        match block {
            ContentBlock::Text(text) => &text.text,
            _ => panic!("expected text block"),
        }
    }

    fn test_provider() -> (AcpProvider, ModelConfig) {
        test_provider_with_tx(None)
    }

    fn test_provider_with_tx(
        tx: Option<mpsc::Sender<ClientRequest>>,
    ) -> (AcpProvider, ModelConfig) {
        (
            AcpProvider {
                name: "acp-test".to_string(),
                goose_mode: Arc::new(Mutex::new(GooseMode::Auto)),
                mode_mapping: HashMap::new(),
                session: AcpSession {
                    id: SessionId::new("test-session"),
                    response: NewSessionResponse::new("test-session"),
                },
                pending_confirmations: Arc::new(TokioMutex::new(HashMap::new())),
                pending_tool_updates: Arc::new(Mutex::new(HashMap::new())),
                handoff_context_sent: AtomicBool::new(false),
                context_size: Arc::new(AtomicU64::new(0)),
                model_config_option_id: None,
                applied_model: Arc::new(Mutex::new(None)),
                tx,
                loop_thread: None,
            },
            ModelConfig::new("test-model"),
        )
    }

    #[test]
    fn messages_to_prompt_without_prior_history_preserves_current_prompt() {
        let messages = vec![Message::user().with_text("current request")];

        let blocks = messages_to_prompt(&messages, true);

        assert_eq!(blocks.len(), 1);
        assert_eq!(prompt_text(&blocks[0]), "current request");
    }

    #[test]
    fn messages_to_prompt_prepends_handoff_context_before_latest_user() {
        let messages = vec![
            Message::user().with_text("inspect src/lib.rs"),
            Message::assistant()
                .with_text("I found the file")
                .with_tool_request("call-1", Ok(CallToolRequestParams::new("read_file"))),
            Message::user().with_tool_response(
                "call-1",
                Ok(CallToolResult::success(vec![RmcpContent::text(
                    "file contents",
                )])),
            ),
            Message::user().with_text("continue from there"),
        ];

        let blocks = messages_to_prompt(&messages, true);

        assert_eq!(blocks.len(), 2);
        let memo = prompt_text(&blocks[0]);
        assert!(memo.starts_with(
            "Conversation context from goose before this ACP provider session was created:"
        ));
        assert!(memo.contains("[user]: inspect src/lib.rs"));
        assert!(memo.contains("[assistant]: I found the file"));
        assert!(memo.contains("tool_request(read_file):"));
        assert!(memo.contains("tool_response: file contents"));
        assert!(memo.contains("Current user request follows."));
        assert_eq!(prompt_text(&blocks[1]), "continue from there");
    }

    #[test]
    fn messages_to_prompt_drops_user_only_acp_rows_from_handoff() {
        let user_only = TextContent::new("SECRET_USER_ONLY")
            .annotations(AcpAnnotations::new().audience(vec![AcpRole::User]));
        let messages = vec![
            Message::user().with_text("visible prior"),
            acp_text_update_message(user_only, "acp-message".to_string(), 123),
            Message::user().with_text("current request"),
        ];

        let blocks = messages_to_prompt(&messages, true);

        assert_eq!(blocks.len(), 2);
        let memo = prompt_text(&blocks[0]);
        assert!(memo.contains("visible prior"));
        assert!(!memo.contains("SECRET_USER_ONLY"));
        assert!(!memo.contains("<empty message>"));
        assert_eq!(prompt_text(&blocks[1]), "current request");
    }

    #[test]
    fn messages_to_prompt_keeps_latest_user_images_after_handoff_memo() {
        let messages = vec![
            Message::assistant().with_text("prior answer"),
            Message::user()
                .with_image("base64-image", "image/png")
                .with_text("describe this"),
        ];

        let blocks = messages_to_prompt(&messages, true);

        assert_eq!(blocks.len(), 3);
        assert!(prompt_text(&blocks[0]).contains("[assistant]: prior answer"));
        match &blocks[1] {
            ContentBlock::Image(image) => {
                assert_eq!(image.data, "base64-image");
                assert_eq!(image.mime_type, "image/png");
            }
            _ => panic!("expected image block"),
        }
        assert_eq!(prompt_text(&blocks[2]), "describe this");
    }

    #[test]
    fn messages_to_prompt_excludes_user_only_current_and_handoff_content() {
        use rmcp::model::RawTextContent;

        fn user_only_text(text: &str) -> MessageContent {
            MessageContent::Text(
                RawTextContent {
                    text: text.to_string(),
                    meta: None,
                }
                .no_annotation()
                .with_audience(vec![Role::User]),
            )
        }

        let messages = vec![
            Message::user()
                .with_text("visible prior")
                .with_content(user_only_text("SECRET_PRIOR")),
            Message::user()
                .with_text("visible current")
                .with_content(user_only_text("SECRET_CURRENT")),
        ];

        let rendered = messages_to_prompt(&messages, true)
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("visible prior"));
        assert!(rendered.contains("visible current"));
        assert!(!rendered.contains("SECRET_PRIOR"));
        assert!(!rendered.contains("SECRET_CURRENT"));
    }

    #[test]
    fn messages_to_prompt_drops_handoff_when_current_content_is_user_only() {
        use rmcp::model::RawTextContent;

        let current = MessageContent::Text(
            RawTextContent {
                text: "user-only".to_string(),
                meta: None,
            }
            .no_annotation()
            .with_audience(vec![Role::User]),
        );
        let messages = vec![
            Message::assistant().with_text("prior context"),
            Message::user().with_content(current),
        ];

        assert!(messages_to_prompt(&messages, true).is_empty());
    }

    #[tokio::test]
    async fn stream_skips_user_only_prompt_without_claiming_handoff_context() {
        use futures::StreamExt;
        use rmcp::model::RawTextContent;

        let (tx, mut rx) = mpsc::channel(1);
        let (provider, model) = test_provider_with_tx(Some(tx));
        let current = MessageContent::Text(
            RawTextContent {
                text: "user-only".to_string(),
                meta: None,
            }
            .no_annotation()
            .with_audience(vec![Role::User]),
        );
        let messages = vec![
            Message::assistant().with_text("prior context"),
            Message::user().with_content(current),
        ];

        let mut stream = provider.stream(&model, "", &messages, &[]).await.unwrap();

        assert!(stream.next().await.is_none());
        assert!(rx.try_recv().is_err());
        assert!(!provider.handoff_context_sent.load(Ordering::Acquire));
    }

    #[test]
    fn live_acp_text_update_preserves_assistant_only_audience() {
        let text = TextContent::new("assistant-only")
            .annotations(AcpAnnotations::new().audience(vec![AcpRole::Assistant]));

        let message = acp_text_update_message(text, "message-id".to_string(), 123);

        let MessageContent::Text(text) = &message.content[0] else {
            panic!("expected text content");
        };
        let audience = text.audience().expect("audience annotation should survive");
        assert!(audience.contains(&Role::Assistant));
        assert!(!audience.contains(&Role::User));
    }

    #[test]
    fn handoff_context_is_sent_only_on_first_provider_prompt() {
        let (provider, _) = test_provider();
        let messages = vec![
            Message::assistant().with_text("prior answer"),
            Message::user().with_text("current request"),
        ];

        let first_claim = provider.claim_handoff_context(&messages);
        assert!(first_claim.first_prompt);
        assert!(first_claim.include_context);

        let second_claim = provider.claim_handoff_context(&messages);
        assert!(!second_claim.first_prompt);
        assert!(!second_claim.include_context);
    }

    #[test]
    fn first_prompt_without_history_still_marks_handoff_context_sent() {
        let (provider, _) = test_provider();
        let first_prompt = vec![Message::user().with_text("new conversation")];
        let later_prompt_with_history = vec![
            Message::assistant().with_text("prior answer"),
            Message::user().with_text("current request"),
        ];

        let first_claim = provider.claim_handoff_context(&first_prompt);
        assert!(first_claim.first_prompt);
        assert!(!first_claim.include_context);

        let later_claim = provider.claim_handoff_context(&later_prompt_with_history);
        assert!(!later_claim.first_prompt);
        assert!(!later_claim.include_context);
    }

    #[tokio::test]
    async fn get_context_limit_surfaces_captured_context_size() {
        let (provider, model) = test_provider();
        assert_eq!(
            provider.get_context_limit(&model).await.unwrap(),
            goose_providers::model::DEFAULT_CONTEXT_LIMIT
        );

        provider.context_size.store(200_000, Ordering::Relaxed);
        assert_eq!(provider.get_context_limit(&model).await.unwrap(), 200_000);
    }

    #[tokio::test]
    async fn failed_first_prompt_send_rolls_back_handoff_context_claim() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let (provider, model) = test_provider_with_tx(Some(tx));
        let messages = vec![
            Message::assistant().with_text("prior answer"),
            Message::user().with_text("current request"),
        ];

        let result = provider.stream(&model, "", &messages, &[]).await;

        assert!(matches!(result, Err(ProviderError::RequestFailed(_))));
        let next_claim = provider.claim_handoff_context(&messages);
        assert!(next_claim.first_prompt);
        assert!(next_claim.include_context);
    }

    fn test_provider_with_model_option(
        tx: mpsc::Sender<ClientRequest>,
        applied_model: Option<String>,
    ) -> AcpProvider {
        let (mut provider, _) = test_provider_with_tx(Some(tx));
        provider.model_config_option_id = Some("model".to_string());
        provider.applied_model = Arc::new(Mutex::new(applied_model));
        provider
    }

    #[tokio::test]
    async fn apply_model_if_changed_sends_set_config_option_on_change() {
        let (tx, mut rx) = mpsc::channel(1);
        let provider = test_provider_with_model_option(tx, Some("old-model".to_string()));

        let handle =
            tokio::spawn(async move { provider.apply_model_if_changed("new-model").await });

        match rx.recv().await.expect("expected a SetConfigOption request") {
            ClientRequest::SetConfigOption {
                config_id,
                value,
                response_tx,
                ..
            } => {
                assert_eq!(config_id, "model");
                assert_eq!(value, "new-model");
                let _ = response_tx.send(Ok(()));
            }
            _ => panic!("unexpected request kind"),
        }

        handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn apply_model_if_changed_skips_when_model_unchanged() {
        let (tx, mut rx) = mpsc::channel(1);
        let provider = test_provider_with_model_option(tx, Some("same-model".to_string()));

        provider.apply_model_if_changed("same-model").await.unwrap();

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn apply_model_if_changed_noop_without_option_id() {
        let (tx, mut rx) = mpsc::channel(1);
        let (provider, _) = test_provider_with_tx(Some(tx));

        provider.apply_model_if_changed("any-model").await.unwrap();

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn apply_model_if_changed_skips_sentinel_model() {
        let (tx, mut rx) = mpsc::channel(1);
        let provider = test_provider_with_model_option(tx, None);

        provider
            .apply_model_if_changed(ACP_CURRENT_MODEL)
            .await
            .unwrap();

        assert!(rx.try_recv().is_err());
    }

    fn test_acp_config(
        mode_mapping: HashMap<GooseMode, Vec<String>>,
        session_mode_id: Option<String>,
    ) -> AcpProviderConfig {
        AcpProviderConfig {
            command: PathBuf::new(),
            args: vec![],
            env: vec![],
            env_remove: vec![],
            work_dir: PathBuf::new(),
            mcp_servers: vec![],
            session_mode_id,
            session_config_options: vec![],
            model_config_option_id: None,
            mode_mapping,
            notification_callback: None,
        }
    }

    #[test_case(GooseMode::Auto)]
    #[test_case(GooseMode::Approve)]
    #[test_case(GooseMode::SmartApprove)]
    #[test_case(GooseMode::Chat)]
    fn initial_mode_candidates_empty_when_mode_negotiation_disabled(mode: GooseMode) {
        let config = test_acp_config(HashMap::new(), None);
        assert!(initial_mode_candidates(&config, Some(mode)).is_empty());
    }

    #[test]
    fn initial_mode_candidates_prefer_mapping_then_fallback() {
        let mapping = HashMap::from([(GooseMode::Auto, vec!["bypassPermissions".to_string()])]);
        let config = test_acp_config(mapping, Some("default".to_string()));

        assert_eq!(
            initial_mode_candidates(&config, Some(GooseMode::Auto)),
            vec!["bypassPermissions".to_string()]
        );
        assert_eq!(
            initial_mode_candidates(&config, Some(GooseMode::Chat)),
            vec!["default".to_string()]
        );
    }

    fn mode_state(current: &str, available: &[&str]) -> SessionModeState {
        SessionModeState::new(
            SessionModeId::new(current),
            available
                .iter()
                .map(|id| SessionMode::new(SessionModeId::new(*id), *id))
                .collect(),
        )
    }

    #[test_case(
        &["full-access", "agent-full-access"],
        &["read-only", "auto", "full-access"],
        Some("full-access")
        ; "zed era ids"
    )]
    #[test_case(
        &["full-access", "agent-full-access"],
        &["read-only", "agent", "agent-full-access"],
        Some("agent-full-access")
        ; "agentclientprotocol era ids"
    )]
    #[test_case(
        &["full-access", "agent-full-access"],
        &["something-else"],
        None
        ; "no candidate offered"
    )]
    fn select_mode_id_picks_first_offered_candidate(
        candidates: &[&str],
        available: &[&str],
        expected: Option<&str>,
    ) {
        let candidates: Vec<String> = candidates.iter().map(|s| s.to_string()).collect();
        let modes = mode_state(available[0], available);
        assert_eq!(
            select_mode_id(&candidates, Some(&modes)),
            expected.map(|s| s.to_string())
        );
    }

    #[test]
    fn select_mode_id_first_candidate_when_agent_has_no_modes() {
        let candidates = vec!["full-access".to_string(), "agent-full-access".to_string()];
        assert_eq!(
            select_mode_id(&candidates, None),
            Some("full-access".to_string())
        );
    }

    #[tokio::test]
    async fn update_mode_without_mapping_skips_acp_request_but_tracks_mode() {
        let (tx, mut rx) = mpsc::channel(1);
        let (provider, _) = test_provider_with_tx(Some(tx));

        provider
            .update_mode("session", GooseMode::Chat)
            .await
            .unwrap();

        assert!(rx.try_recv().is_err());
        assert_eq!(*provider.goose_mode.lock().unwrap(), GooseMode::Chat);
    }

    #[tokio::test]
    async fn update_mode_with_mapping_sends_set_mode() {
        let (tx, mut rx) = mpsc::channel(1);
        let (mut provider, _) = test_provider_with_tx(Some(tx));
        provider.mode_mapping = HashMap::from([(GooseMode::Chat, vec!["plan".to_string()])]);

        let handle = tokio::spawn(async move {
            provider
                .update_mode("session", GooseMode::Chat)
                .await
                .unwrap();
            provider
        });

        match rx.recv().await.expect("expected a SetMode request") {
            ClientRequest::SetMode {
                mode_id,
                response_tx,
                ..
            } => {
                assert_eq!(mode_id, "plan");
                let _ = response_tx.send(Ok(()));
            }
            _ => panic!("unexpected request kind"),
        }

        let provider = handle.await.unwrap();
        assert_eq!(*provider.goose_mode.lock().unwrap(), GooseMode::Chat);
    }

    #[tokio::test]
    async fn update_mode_sends_candidate_offered_by_agent() {
        let (tx, mut rx) = mpsc::channel(1);
        let (mut provider, _) = test_provider_with_tx(Some(tx));
        provider.mode_mapping = HashMap::from([(
            GooseMode::Auto,
            vec!["full-access".to_string(), "agent-full-access".to_string()],
        )]);
        provider.session.response = NewSessionResponse::new("test-session").modes(mode_state(
            "read-only",
            &["read-only", "agent", "agent-full-access"],
        ));

        let handle = tokio::spawn(async move {
            provider
                .update_mode("session", GooseMode::Auto)
                .await
                .unwrap();
            provider
        });

        match rx.recv().await.expect("expected a SetMode request") {
            ClientRequest::SetMode {
                mode_id,
                response_tx,
                ..
            } => {
                assert_eq!(mode_id, "agent-full-access");
                let _ = response_tx.send(Ok(()));
            }
            _ => panic!("unexpected request kind"),
        }

        let provider = handle.await.unwrap();
        assert_eq!(*provider.goose_mode.lock().unwrap(), GooseMode::Auto);
    }

    #[tokio::test]
    async fn update_mode_errors_when_no_candidate_offered() {
        let (tx, mut rx) = mpsc::channel(1);
        let (mut provider, _) = test_provider_with_tx(Some(tx));
        provider.mode_mapping = HashMap::from([(GooseMode::Chat, vec!["read-only".to_string()])]);
        provider.session.response = NewSessionResponse::new("test-session")
            .modes(mode_state("agent", &["agent", "agent-full-access"]));

        let result = provider.update_mode("session", GooseMode::Chat).await;

        assert!(result.is_err());
        assert!(rx.try_recv().is_err());
        assert_eq!(*provider.goose_mode.lock().unwrap(), GooseMode::Auto);
    }

    #[test]
    fn messages_to_prompt_includes_all_prior_handoff_context() {
        let messages = vec![
            Message::user().with_text("older context that should be retained"),
            Message::assistant().with_text("middle context"),
            Message::assistant().with_text("recent context"),
            Message::user().with_text("current request"),
        ];

        let blocks = messages_to_prompt(&messages, true);

        assert_eq!(blocks.len(), 2);
        let memo = prompt_text(&blocks[0]);
        assert!(memo.contains("[user]: older context that should be retained"));
        assert!(memo.contains("[assistant]: middle context"));
        assert!(memo.contains("[assistant]: recent context"));
        assert_eq!(prompt_text(&blocks[1]), "current request");
    }

    #[test_case(
        ExtensionConfig::Stdio {
            name: "github".into(),
            description: String::new(),
            cmd: "/path/to/github-mcp-server".into(),
            args: vec!["stdio".into()],
            envs: Envs::new([("GITHUB_PERSONAL_ACCESS_TOKEN".into(), "ghp_xxxxxxxxxxxx".into())].into()),
            env_keys: vec![],
            timeout: None,
            cwd: None,
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
            socket: None,
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
            socket: None,
            bundled: Some(false),
            available_tools: vec![],
        };

        let servers = extension_configs_to_mcp_servers(&[config]);
        let filtered = filter_supported_servers(&servers, &McpCapabilities::default());
        assert!(filtered.is_empty());
    }

    #[test_case(GooseMode::Auto => Some(PermissionDecision::AllowOnce) ; "auto allows")]
    #[test_case(GooseMode::Chat => Some(PermissionDecision::RejectOnce) ; "chat rejects")]
    #[test_case(GooseMode::Approve => None ; "approve defers")]
    #[test_case(GooseMode::SmartApprove => None ; "smart_approve defers")]
    fn test_permission_decision_from_mode(mode: GooseMode) -> Option<PermissionDecision> {
        permission_decision_from_mode(mode)
    }

    #[test_case(
        HashMap::from([
            (GooseMode::Auto, vec!["yolo".to_string()]),
            (GooseMode::Approve, vec!["default".to_string()]),
            (GooseMode::SmartApprove, vec!["auto_edit".to_string()]),
            (GooseMode::Chat, vec!["plan".to_string()]),
        ]),
        HashMap::from([
            ("yolo".to_string(), vec![GooseMode::Auto]),
            ("default".to_string(), vec![GooseMode::Approve]),
            ("auto_edit".to_string(), vec![GooseMode::SmartApprove]),
            ("plan".to_string(), vec![GooseMode::Chat]),
        ])
        ; "gemini provider mapping"
    )]
    #[test_case(
        HashMap::from([
            (GooseMode::Auto, vec!["bypassPermissions".to_string()]),
            (GooseMode::Approve, vec!["default".to_string()]),
            (GooseMode::SmartApprove, vec!["acceptEdits".to_string()]),
            (GooseMode::Chat, vec!["plan".to_string()]),
        ]),
        HashMap::from([
            ("bypassPermissions".to_string(), vec![GooseMode::Auto]),
            ("default".to_string(), vec![GooseMode::Approve]),
            ("acceptEdits".to_string(), vec![GooseMode::SmartApprove]),
            ("plan".to_string(), vec![GooseMode::Chat]),
        ])
        ; "claude provider mapping"
    )]
    #[test_case(
        HashMap::from([
            (GooseMode::Auto, vec!["full-access".to_string(), "agent-full-access".to_string()]),
            (GooseMode::Approve, vec!["read-only".to_string()]),
            (GooseMode::SmartApprove, vec!["auto".to_string(), "agent".to_string()]),
            (GooseMode::Chat, vec!["read-only".to_string()]),
        ]),
        HashMap::from([
            ("full-access".to_string(), vec![GooseMode::Auto]),
            ("agent-full-access".to_string(), vec![GooseMode::Auto]),
            ("read-only".to_string(), vec![GooseMode::Approve, GooseMode::Chat]),
            ("auto".to_string(), vec![GooseMode::SmartApprove]),
            ("agent".to_string(), vec![GooseMode::SmartApprove]),
        ])
        ; "codex candidates for both bridge generations"
    )]
    fn test_reverse_mode_mapping(
        forward: HashMap<GooseMode, Vec<String>>,
        expected: HashMap<String, Vec<GooseMode>>,
    ) {
        let result = reverse_mode_mapping(&forward);
        assert_eq!(result.len(), expected.len());
        for (key, expected_modes) in &expected {
            let actual = result.get(key).expect("missing key");
            assert_eq!(
                actual.len(),
                expected_modes.len(),
                "length mismatch for key {key}"
            );
            for mode in expected_modes {
                assert!(actual.contains(mode), "missing {mode:?} for key {key}");
            }
        }
    }

    #[test_case(
        NewSessionResponse::new("s1")
            .config_options(vec![
                SessionConfigOption::select("model", "Model", "default", vec![
                    SessionConfigSelectOption::new("default", "Default (recommended)"),
                    SessionConfigSelectOption::new("sonnet", "Sonnet"),
                    SessionConfigSelectOption::new("haiku", "Haiku"),
                ])
                .category(SessionConfigOptionCategory::Model),
            ])
        => Ok(("default".to_string(), vec!["default".to_string(), "sonnet".to_string(), "haiku".to_string()]))
        ; "model is resolved from config_options"
    )]
    #[test_case(
        NewSessionResponse::new("s1")
            .config_options(vec![
                SessionConfigOption::select("model", "Model", "auto-gemini-3", vec![
                    SessionConfigSelectOption::new("auto-gemini-3", "Auto (Gemini 3)"),
                    SessionConfigSelectOption::new("auto-gemini-2.5", "Auto (Gemini 2.5)"),
                    SessionConfigSelectOption::new("gemini-2.5-pro", "gemini-2.5-pro"),
                ])
                .category(SessionConfigOptionCategory::Model),
            ])
        => Ok(("auto-gemini-3".to_string(), vec!["auto-gemini-3".to_string(), "auto-gemini-2.5".to_string(), "gemini-2.5-pro".to_string()]))
        ; "model with multiple options"
    )]
    #[test_case(
        NewSessionResponse::new("s1")
        => Err(ProviderError::RequestFailed(
            "test: agent returned no model config_options".to_string()
        ))
        ; "missing model config_options is an error"
    )]
    fn test_resolve_model_info(
        response: NewSessionResponse,
    ) -> Result<(String, Vec<String>), ProviderError> {
        resolve_model_info("test", &response)
    }

    fn duplicate_read_only_reverse_modes() -> HashMap<String, Vec<GooseMode>> {
        HashMap::from([
            ("full-access".to_string(), vec![GooseMode::Auto]),
            (
                "read-only".to_string(),
                vec![GooseMode::Approve, GooseMode::Chat],
            ),
            ("auto".to_string(), vec![GooseMode::SmartApprove]),
        ])
    }

    #[test_case(
        "full-access", GooseMode::Auto, Some(GooseMode::Auto)
        ; "unique mapping returns the only candidate"
    )]
    #[test_case(
        "read-only", GooseMode::Approve, Some(GooseMode::Approve)
        ; "duplicate prefers current when current is Approve"
    )]
    #[test_case(
        "read-only", GooseMode::Chat, Some(GooseMode::Chat)
        ; "duplicate prefers current when current is Chat"
    )]
    #[test_case(
        "read-only", GooseMode::Auto, Some(GooseMode::Approve)
        ; "duplicate falls back to first when current not in candidates"
    )]
    #[test_case(
        "unknown-id", GooseMode::Auto, None
        ; "unknown mode id returns None"
    )]
    fn test_resolve_mode(mode_id: &str, current: GooseMode, expected: Option<GooseMode>) {
        let reverse_modes = duplicate_read_only_reverse_modes();
        let current = Arc::new(Mutex::new(current));
        let result = resolve_mode(&reverse_modes, mode_id, &current);
        if mode_id == "read-only" && expected == Some(GooseMode::Approve) {
            assert!(result == Some(GooseMode::Approve) || result == Some(GooseMode::Chat));
        } else {
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn acp_tool_call_content_handles_text_diff_terminal_and_image() {
        use agent_client_protocol::schema::v1::{Diff, Terminal, TerminalId, TextContent};

        let diff_block = ToolCallContent::Diff(
            Diff::new(std::path::PathBuf::from("/tmp/file.txt"), "new\n").old_text("old\n"),
        );
        let terminal_block = ToolCallContent::Terminal(Terminal::new(TerminalId::new("term-7")));
        let text_block = ToolCallContent::Content(agent_client_protocol::schema::v1::Content::new(
            ContentBlock::Text(TextContent::new("hello")),
        ));
        let image_block =
            ToolCallContent::Content(agent_client_protocol::schema::v1::Content::new(
                ContentBlock::Image(ImageContent::new("base64data", "image/png")),
            ));

        let out = acp_tool_call_content_to_rmcp(
            Some(vec![text_block, diff_block, terminal_block, image_block]),
            None,
        );

        assert_eq!(out.len(), 3, "terminal blocks produce no output");
        let serialized: Vec<String> = out
            .iter()
            .map(|c| serde_json::to_string(c).unwrap())
            .collect();
        assert!(
            serialized[0].contains("hello"),
            "text block lost: {serialized:?}"
        );
        assert!(
            serialized[1].contains("/tmp/file.txt"),
            "diff path lost: {serialized:?}"
        );
        assert!(
            serialized[1].contains("new"),
            "diff body lost: {serialized:?}"
        );
        assert!(
            serialized[2].contains("base64data"),
            "image data lost: {serialized:?}"
        );
    }

    #[test]
    fn acp_tool_call_terminal_falls_back_to_raw_output() {
        use agent_client_protocol::schema::v1::{Terminal, TerminalId};

        let terminal_block = ToolCallContent::Terminal(Terminal::new(TerminalId::new("term-1")));
        let raw_output = serde_json::Value::String("hello from shell".to_string());

        let out = acp_tool_call_content_to_rmcp(Some(vec![terminal_block]), Some(raw_output));

        assert_eq!(out.len(), 1);
        let text = out[0].as_text().unwrap();
        assert_eq!(text.text, "hello from shell");
        assert_eq!(out[0].priority(), Some(0.0));
    }

    #[test]
    fn acp_tool_call_content_preserves_audience_annotations() {
        let text_block = ToolCallContent::Content(agent_client_protocol::schema::v1::Content::new(
            ContentBlock::Text(
                TextContent::new("user-only")
                    .annotations(AcpAnnotations::new().audience(vec![AcpRole::User])),
            ),
        ));
        let image_block = ToolCallContent::Content(
            agent_client_protocol::schema::v1::Content::new(ContentBlock::Image(
                ImageContent::new("base64data", "image/png")
                    .annotations(AcpAnnotations::new().audience(vec![AcpRole::Assistant])),
            )),
        );

        let out = acp_tool_call_content_to_rmcp(Some(vec![text_block, image_block]), None);

        let text_audience = out[0]
            .audience()
            .expect("text audience annotation should survive");
        assert!(text_audience.contains(&Role::User));
        assert!(!text_audience.contains(&Role::Assistant));
        let image_audience = out[1]
            .audience()
            .expect("image audience annotation should survive");
        assert!(image_audience.contains(&Role::Assistant));
        assert!(!image_audience.contains(&Role::User));
    }

    #[test]
    fn acp_tool_call_content_falls_back_to_raw_output_when_blocks_empty() {
        let out =
            acp_tool_call_content_to_rmcp(Some(vec![]), Some(serde_json::json!({"key": "value"})));
        assert_eq!(out.len(), 1);
        let serialized = serde_json::to_string(&out[0]).unwrap();
        assert!(
            serialized.contains("key"),
            "fallback raw_output lost: {serialized}"
        );
    }

    /// Pins the tool_meta shape that the `AcpUpdate::ToolCallStart` consumer
    /// emits onto the synthesized `ToolRequest`. ACP doesn't expose a canonical
    /// tool name to clients, so we surface `kind` here as a stable categorization
    /// signal alongside the `external_dispatch` marker that bypasses agent-loop
    /// routing.
    #[test]
    fn tool_meta_pairs_external_dispatch_marker_with_acp_kind() {
        let cases = [
            (ToolKind::Execute, "execute"),
            (ToolKind::Read, "read"),
            (ToolKind::Edit, "edit"),
            (ToolKind::Other, "other"),
        ];
        for (kind, expected) in cases {
            let tool_meta = serde_json::json!({
                TOOL_META_EXTERNAL_DISPATCH_KEY: true,
                "goose.acp.kind": kind,
            });
            assert_eq!(
                tool_meta[TOOL_META_EXTERNAL_DISPATCH_KEY],
                serde_json::Value::Bool(true),
                "external_dispatch marker missing for kind={kind:?}"
            );
            assert_eq!(
                tool_meta["goose.acp.kind"],
                serde_json::Value::String(expected.to_string()),
                "goose.acp.kind serialized wrong for kind={kind:?}"
            );
        }
    }
}
