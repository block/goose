use agent_client_protocol_schema::Usage as AcpUsage;
use agent_client_protocol_schema::AGENT_METHOD_NAMES;
use anyhow::{Context, Result};
use async_stream::try_stream;
use futures::future::BoxFuture;
use rmcp::model::{Role, Tool};
use sacp::schema::{
    AuthMethod, CloseSessionRequest, ContentBlock, ContentChunk, EnvVariable, HttpHeader,
    ImageContent, InitializeRequest, InitializeResponse, ListSessionsRequest, ListSessionsResponse,
    McpCapabilities, McpServer, McpServerHttp, McpServerStdio, NewSessionRequest,
    NewSessionResponse, PromptRequest, PromptResponse, ProtocolVersion, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SessionConfigKind,
    SessionConfigOptionCategory, SessionConfigSelectOptions, SessionId, SessionNotification,
    SessionUpdate, SetSessionConfigOptionRequest, SetSessionModeRequest, SetSessionModeResponse,
    SetSessionModelRequest, StopReason, TextContent, ToolCallContent,
};
use sacp::{Agent, Client, ConnectionTo};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex as TokioMutex, OnceCell};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::acp::{map_permission_response, PermissionDecision, PermissionMapping};
use crate::config::{ExtensionConfig, GooseMode};
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::permission::permission_confirmation::PrincipalType;
use crate::permission::{Permission, PermissionConfirmation};
use crate::providers::base::{MessageStream, PermissionRouting, Provider, ProviderUsage, Usage};
use crate::providers::errors::ProviderError;
use crate::subprocess::configure_subprocess;

/// Sentinel: resolved to SessionModelState.current_model_id at connect time.
pub const ACP_CURRENT_MODEL: &str = "current";

pub struct AcpProviderConfig {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub env_remove: Vec<String>,
    pub work_dir: PathBuf,
    pub mcp_servers: Vec<McpServer>,
    pub session_mode_id: Option<String>,
    pub mode_mapping: HashMap<GooseMode, String>,
    pub permission_mapping: PermissionMapping,
    pub notification_callback: Option<Arc<dyn Fn(SessionNotification) + Send + Sync>>,
}

enum ClientRequest {
    NewSession {
        response_tx: oneshot::Sender<Result<NewSessionResponse>>,
    },
    ListSessions {
        response_tx: oneshot::Sender<Result<ListSessionsResponse>>,
    },
    SetMode {
        session_id: SessionId,
        mode_id: String,
        response_tx: oneshot::Sender<Result<()>>,
    },
    SetModel {
        session_id: SessionId,
        model_id: String,
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
    CloseSession {
        session_id: SessionId,
        response_tx: oneshot::Sender<Result<()>>,
    },
    // For ACP methods not yet in agent-client-protocol-schema (e.g. session/delete)
    Untyped {
        method: String,
        params: serde_json::Value,
        response_tx: oneshot::Sender<Result<serde_json::Value>>,
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
    Text(String),
    Thought(String),
    ToolCallStart {
        id: String,
    },
    ToolCallComplete {
        id: String,
    },
    PermissionRequest {
        request: Box<RequestPermissionRequest>,
        response_tx: oneshot::Sender<RequestPermissionResponse>,
    },
    Complete(StopReason, Option<AcpUsage>),
    Error(String),
}

pub struct AcpProvider {
    name: String,
    model: ModelConfig,
    goose_mode: Arc<Mutex<GooseMode>>,
    tx: Option<mpsc::Sender<ClientRequest>>,
    loop_thread: Option<JoinHandle<()>>,
    mode_mapping: HashMap<GooseMode, String>,
    permission_mapping: PermissionMapping,
    rejected_tool_calls: Arc<TokioMutex<HashSet<String>>>,
    pending_confirmations:
        Arc<TokioMutex<HashMap<String, oneshot::Sender<PermissionConfirmation>>>>,
    goose_to_acp_id: Arc<TokioMutex<HashMap<String, NewSessionResponse>>>,
    acp_to_goose_id: Arc<TokioMutex<HashMap<String, String>>>,
    /// Per-session model tracking for detecting model changes in stream().
    session_model: Arc<TokioMutex<HashMap<String, String>>>,
    auth_methods: Vec<AuthMethod>,
    supports_close: bool,
    init_session: OnceCell<NewSessionResponse>,
}

impl std::fmt::Debug for AcpProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcpProvider")
            .field("name", &self.name)
            .field("model", &self.model)
            .finish()
    }
}

// Dedicated runtime on an OS thread so session/close completes even during
// main runtime shutdown. See reqwest InnerClientHandle.
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
        model: ModelConfig,
        goose_mode: GooseMode,
        config: AcpProviderConfig,
    ) -> Result<Self> {
        Self::start(
            name,
            model,
            goose_mode,
            config,
            Box::new(|cl, rx, init_tx| Box::pin(cl.spawn(rx, init_tx))),
        )
        .await
    }

    #[doc(hidden)]
    pub async fn connect_with_transport(
        name: String,
        model: ModelConfig,
        goose_mode: GooseMode,
        config: AcpProviderConfig,
        transport: impl sacp::ConnectTo<Client> + 'static,
    ) -> Result<Self> {
        Self::start(
            name,
            model,
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
        model: ModelConfig,
        goose_mode: GooseMode,
        config: AcpProviderConfig,
        run: ClientLoopFn,
    ) -> Result<Self> {
        let (tx, rx) = mpsc::channel(32);
        let (init_tx, init_rx) = oneshot::channel();
        let mode_mapping = config.mode_mapping.clone();
        let permission_mapping = config.permission_mapping.clone();
        let rejected_tool_calls = Arc::new(TokioMutex::new(HashSet::new()));
        let goose_mode = Arc::new(Mutex::new(goose_mode));
        let client_loop = AcpClientLoop::new(config, goose_mode.clone());
        let loop_thread = spawn_client_loop(run(client_loop, rx, init_tx));

        let init_response = init_rx
            .await
            .context("ACP client initialization cancelled")??;

        let supports_close = init_response
            .agent_capabilities
            .session_capabilities
            .close
            .is_some();
        let mut provider = Self::new_with_runtime(
            name,
            model,
            goose_mode,
            tx,
            loop_thread,
            mode_mapping,
            permission_mapping,
            rejected_tool_calls,
            init_response.auth_methods,
            supports_close,
        );
        if provider.model.model_name == ACP_CURRENT_MODEL {
            let response = provider.get_init_session().await?;
            let (current_model, _) = resolve_model_info(&provider.name, response)?;
            tracing::info!(from = ACP_CURRENT_MODEL, to = %current_model, "resolved ACP model");
            provider.model.model_name = current_model;
        }
        Ok(provider)
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_runtime(
        name: String,
        model: ModelConfig,
        goose_mode: Arc<Mutex<GooseMode>>,
        tx: mpsc::Sender<ClientRequest>,
        loop_thread: JoinHandle<()>,
        mode_mapping: HashMap<GooseMode, String>,
        permission_mapping: PermissionMapping,
        rejected_tool_calls: Arc<TokioMutex<HashSet<String>>>,
        auth_methods: Vec<AuthMethod>,
        supports_close: bool,
    ) -> Self {
        Self {
            name,
            model,
            goose_mode,
            tx: Some(tx),
            loop_thread: Some(loop_thread),
            mode_mapping,
            permission_mapping,
            rejected_tool_calls,
            pending_confirmations: Arc::new(TokioMutex::new(HashMap::new())),
            goose_to_acp_id: Arc::new(TokioMutex::new(HashMap::new())),
            acp_to_goose_id: Arc::new(TokioMutex::new(HashMap::new())),
            session_model: Arc::new(TokioMutex::new(HashMap::new())),
            auth_methods,
            supports_close,
            init_session: OnceCell::new(),
        }
    }

    pub fn auth_methods(&self) -> &[AuthMethod] {
        &self.auth_methods
    }

    pub async fn new_session(&self) -> Result<NewSessionResponse> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .as_ref()
            .unwrap()
            .send(ClientRequest::NewSession { response_tx })
            .await
            .context("ACP client is unavailable")?;
        response_rx
            .await
            .context(format!("ACP {} cancelled", AGENT_METHOD_NAMES.session_new))?
    }

    pub async fn list_sessions(&self) -> Result<ListSessionsResponse> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .as_ref()
            .unwrap()
            .send(ClientRequest::ListSessions { response_tx })
            .await
            .context("ACP client is unavailable")?;
        let raw = response_rx.await.context("ACP request cancelled")??;
        let acp_to_goose = self.acp_to_goose_id.lock().await;
        Ok(map_sessions_to_goose_ids(raw, &acp_to_goose))
    }

    async fn resolve_acp_session_id(&self, goose_id: &str) -> Result<SessionId> {
        let map = self.goose_to_acp_id.lock().await;
        map.get(goose_id)
            .map(|r| r.session_id.clone())
            .ok_or_else(|| {
                sacp::Error::resource_not_found(Some(goose_id.to_string()))
                    .data(format!("Session not found: {goose_id}"))
                    .into()
            })
    }

    pub(crate) async fn send_set_mode(&self, goose_id: &str, mode_id: String) -> Result<()> {
        let session_id = self.resolve_acp_session_id(goose_id).await?;
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

    pub(crate) async fn send_set_model(&self, goose_id: &str, model_id: String) -> Result<()> {
        let session_id = self.resolve_acp_session_id(goose_id).await?;
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .as_ref()
            .unwrap()
            .send(ClientRequest::SetModel {
                session_id,
                model_id,
                response_tx,
            })
            .await
            .context("ACP client is unavailable")?;
        response_rx.await.context("ACP request cancelled")?
    }

    pub(crate) async fn send_set_config_option(
        &self,
        goose_id: &str,
        config_id: String,
        value: String,
    ) -> Result<()> {
        let session_id = self.resolve_acp_session_id(goose_id).await?;
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

    // Only used by tests; session/delete has no typed request in agent-client-protocol-schema yet.
    #[doc(hidden)]
    pub async fn delete_session(&self, goose_id: &str) -> Result<()> {
        let session_id = self.resolve_acp_session_id(goose_id).await?;
        self.send_untyped(
            "session/delete",
            serde_json::json!({ "sessionId": session_id.0 }),
        )
        .await?;

        // Clean up cached mappings so ensure_session doesn't return a stale entry.
        self.goose_to_acp_id.lock().await.remove(goose_id);
        self.acp_to_goose_id
            .lock()
            .await
            .remove(session_id.0.as_ref());
        self.session_model.lock().await.remove(goose_id);
        Ok(())
    }

    pub async fn send_untyped(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .as_ref()
            .unwrap()
            .send(ClientRequest::Untyped {
                method: method.to_string(),
                params,
                response_tx,
            })
            .await
            .context("ACP client is unavailable")?;
        response_rx.await.context("ACP request cancelled")?
    }

    pub async fn has_session(&self, goose_id: &str) -> bool {
        self.goose_to_acp_id.lock().await.contains_key(goose_id)
    }

    // If false, callers fall back to legacy set_mode/set_model.
    async fn session_has_config_option(
        &self,
        goose_id: &str,
        category: SessionConfigOptionCategory,
    ) -> bool {
        let map = self.goose_to_acp_id.lock().await;
        map.get(goose_id)
            .and_then(|r| r.config_options.as_ref())
            .is_some_and(|opts| opts.iter().any(|o| o.category.as_ref() == Some(&category)))
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

    pub async fn ensure_session(
        &self,
        session_id: Option<&str>,
    ) -> Result<NewSessionResponse, ProviderError> {
        if let Some(session_id) = session_id {
            if let Some(response) = self.goose_to_acp_id.lock().await.get(session_id) {
                return Ok(response.clone());
            }
        }

        let response = self.new_session().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to create ACP session: {e}"))
        })?;

        if let Some(session_id) = session_id {
            self.goose_to_acp_id
                .lock()
                .await
                .insert(session_id.to_string(), response.clone());
            self.acp_to_goose_id
                .lock()
                .await
                .insert(response.session_id.0.to_string(), session_id.to_string());

            // Initialize model tracking so stream() can detect changes.
            let (current_model, _) = resolve_model_info(&self.name, &response)?;
            self.session_model
                .lock()
                .await
                .entry(session_id.to_string())
                .or_insert(current_model);
        }

        Ok(response)
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

    async fn get_init_session(&self) -> Result<&NewSessionResponse> {
        self.init_session
            .get_or_try_init(|| async {
                let response = self.new_session().await?;
                if self.supports_close {
                    self.close_session_by_acp_id(response.session_id.clone())
                        .await?;
                }
                Ok(response)
            })
            .await
    }

    async fn close_session_by_acp_id(&self, session_id: SessionId) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .as_ref()
            .unwrap()
            .send(ClientRequest::CloseSession {
                session_id,
                response_tx,
            })
            .await
            .context("ACP client is unavailable")?;
        response_rx.await.context("ACP request cancelled")?
    }
}

#[async_trait::async_trait]
impl Provider for AcpProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn update_mode(&self, session_id: &str, mode: GooseMode) -> Result<(), ProviderError> {
        let map = self.goose_to_acp_id.lock().await;
        if map.is_empty() {
            // Pre-initialization: no ACP session yet, just store the mode.
            // The shared Arc<Mutex<GooseMode>> is read at session creation time.
            drop(map);
        } else {
            drop(map);
            let mode_str = self.mode_mapping[&mode].clone();
            if self
                .session_has_config_option(session_id, SessionConfigOptionCategory::Mode)
                .await
            {
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

        let mut current = self
            .goose_mode
            .lock()
            .map_err(|_| ProviderError::RequestFailed("Failed to update mode".into()))?;
        *current = mode;
        Ok(())
    }

    fn permission_routing(&self) -> PermissionRouting {
        PermissionRouting::ActionRequired
    }

    async fn handle_permission_confirmation(
        &self,
        request_id: &str,
        confirmation: &PermissionConfirmation,
    ) -> bool {
        AcpProvider::handle_permission_confirmation(self, request_id, confirmation).await
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let response = self.ensure_session(Some(session_id)).await?;

        // Provider trait has no update_model — stream() is the only place to forward model changes.
        {
            let new_model = &model_config.model_name;
            let tracked = self.session_model.lock().await.get(session_id).cloned();
            if tracked.as_deref() != Some(new_model) {
                if self
                    .session_has_config_option(session_id, SessionConfigOptionCategory::Model)
                    .await
                {
                    self.send_set_config_option(session_id, "model".into(), new_model.clone())
                        .await
                        .map_err(|e| {
                            ProviderError::RequestFailed(format!("Failed to set model: {e}"))
                        })?;
                } else {
                    self.send_set_model(session_id, new_model.clone())
                        .await
                        .map_err(|e| {
                            ProviderError::RequestFailed(format!("Failed to set model: {e}"))
                        })?;
                }
                self.session_model
                    .lock()
                    .await
                    .insert(session_id.to_string(), new_model.clone());
            }
        }

        let prompt_blocks = messages_to_prompt(messages);
        let mut rx = self
            .prompt(response.session_id, prompt_blocks)
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to send ACP prompt: {e}")))?;

        let pending_confirmations = self.pending_confirmations.clone();
        let rejected_tool_calls = self.rejected_tool_calls.clone();
        let permission_mapping = self.permission_mapping.clone();
        let goose_mode = *self
            .goose_mode
            .lock()
            .map_err(|_| ProviderError::RequestFailed("goose_mode lock poisoned".into()))?;

        let reject_all_tools = goose_mode == GooseMode::Chat;
        let model_name = model_config.model_name.clone();

        Ok(Box::pin(try_stream! {
            // ACP agents execute tools internally. Goose never dispatches tool calls;
            // it only sees text, thoughts, and permission requests from the agent.
            //
            // In Chat mode (reject_all_tools), we suppress all text after a tool
            // starts because the agent may send tool results as AcpUpdate::Text,
            // bypassing the permission response.
            let mut suppress_text = false;

            while let Some(update) = rx.recv().await {
                match update {
                    AcpUpdate::Text(text) => {
                        if !suppress_text {
                            let message = Message::assistant().with_text(text);
                            yield (Some(message), None);
                        }
                    }
                    AcpUpdate::Thought(text) => {
                        let message = Message::assistant()
                            .with_thinking(text, "")
                            .with_visibility(true, false);
                        yield (Some(message), None);
                    }
                    AcpUpdate::ToolCallStart { id, .. } => {
                        if reject_all_tools {
                            suppress_text = true;
                            rejected_tool_calls.lock().await.insert(id);
                        }
                    }
                    AcpUpdate::ToolCallComplete { id, .. } => {
                        let is_error = rejected_tool_calls.lock().await.remove(&id);
                        if is_error {
                            let message = Message::assistant().with_text("Tool call was denied.");
                            yield (Some(message), None);
                        }
                    }
                    AcpUpdate::PermissionRequest { request, response_tx } => {
                        if let Some(decision) = permission_decision_from_mode(goose_mode) {
                            if decision.should_record_rejection() {
                                rejected_tool_calls.lock().await.insert(request.tool_call.tool_call_id.0.to_string());
                            }
                            let response = map_permission_response(&permission_mapping, &request, decision);
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

                        let decision = PermissionDecision::from(confirmation.permission);
                        if decision.should_record_rejection() {
                            rejected_tool_calls.lock().await.insert(request.tool_call.tool_call_id.0.to_string());
                        }
                        let response = map_permission_response(&permission_mapping, &request, decision);
                        let _ = response_tx.send(response);
                    }
                    AcpUpdate::Complete(_reason, acp_usage) => {
                        if let Some(u) = acp_usage {
                            let provider_usage = ProviderUsage::new(
                                model_name.clone(),
                                Usage {
                                    input_tokens: Some(u.input_tokens as i32),
                                    output_tokens: Some(u.output_tokens as i32),
                                    total_tokens: Some(u.total_tokens as i32),
                                    ..Default::default()
                                },
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
        let response = self.get_init_session().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to create ACP session: {e}"))
        })?;
        let (_, available) = resolve_model_info(&self.name, response)?;
        Ok(available)
    }
}

impl Drop for AcpProvider {
    fn drop(&mut self) {
        // Join OS thread so session/close completes before runtime exits (reqwest InnerClientHandle pattern).
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
}

impl AcpClientLoop {
    fn new(config: AcpProviderConfig, goose_mode: Arc<Mutex<GooseMode>>) -> Self {
        Self {
            config,
            goose_mode,
            prompt_response_tx: Arc::new(Mutex::new(None)),
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
        let transport = sacp::ByteStreams::new(stdin.compat_write(), stdout.compat());
        self.run(transport, rx, init_tx).await
    }

    async fn run(
        self,
        transport: impl sacp::ConnectTo<Client> + 'static,
        rx: &mut mpsc::Receiver<ClientRequest>,
        init_tx: oneshot::Sender<Result<InitializeResponse>>,
    ) -> Result<()> {
        let AcpClientLoop {
            config,
            goose_mode,
            prompt_response_tx,
        } = self;
        let notification_callback = config.notification_callback.clone();
        let reverse_modes = reverse_mode_mapping(&config.mode_mapping);

        Client
            .builder()
            .on_receive_notification(
                {
                    let prompt_response_tx = prompt_response_tx.clone();
                    let reverse_modes = reverse_modes.clone();
                    async move |notification: SessionNotification, _cx| {
                        if let Some(ref cb) = notification_callback {
                            cb(notification.clone());
                        }
                        // stream() reads goose_mode at call time, so it must
                        // reflect any prior set_mode before the next prompt.
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
                                    let _ = tx.try_send(AcpUpdate::ToolCallStart {
                                        id: tool_call.tool_call_id.0.to_string(),
                                    });
                                }
                                SessionUpdate::ToolCallUpdate(update) => {
                                    if update.fields.status.is_some() {
                                        let _ = tx.try_send(AcpUpdate::ToolCallComplete {
                                            id: update.tool_call_id.0.to_string(),
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
                    async move |request: RequestPermissionRequest, responder, _connection_cx| {
                        let (response_tx, response_rx) = oneshot::channel();

                        let handler = prompt_response_tx
                            .lock()
                            .ok()
                            .as_ref()
                            .and_then(|g| g.as_ref().cloned());
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
                        responder.respond(response)
                    }
                },
                sacp::on_receive_request!(),
            )
            .connect_with(transport, async move |cx: ConnectionTo<Agent>| {
                handle_requests(config, cx, rx, prompt_response_tx, init_tx).await
            })
            .await?;

        Ok(())
    }
}

async fn spawn_acp_process(config: &AcpProviderConfig) -> Result<Child> {
    let mut cmd = Command::new(&config.command);
    cmd.args(&config.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
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

// sacp panics on Err from connect_with handlers, so log send failures instead of ?.
fn log_undelivered<E: std::fmt::Debug>(result: Result<(), E>, method: &str) {
    if let Err(e) = result {
        tracing::debug!(method, error = ?e, "response not delivered");
    }
}

async fn handle_requests(
    config: AcpProviderConfig,
    cx: ConnectionTo<Agent>,
    rx: &mut mpsc::Receiver<ClientRequest>,
    prompt_response_tx: Arc<Mutex<Option<mpsc::Sender<AcpUpdate>>>>,
    init_tx: oneshot::Sender<Result<InitializeResponse>>,
) -> Result<(), sacp::Error> {
    let mut init_tx = Some(init_tx);

    let init_response: InitializeResponse = cx
        .send_request(InitializeRequest::new(ProtocolVersion::LATEST))
        .block_task()
        .await
        .map_err(|err| {
            let message = format!("ACP {} failed: {err}", AGENT_METHOD_NAMES.initialize);
            // Attempt to send a specific error to the ctor waiting on init_rx;
            if let Some(tx) = init_tx.take() {
                let _ = tx.send(Err(anyhow::anyhow!(message.clone())));
            }
            sacp::Error::internal_error().data(message)
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
                        apply_session_mode(&config, &cx, session).await
                    }
                    Err(err) => Err(anyhow::anyhow!(
                        "ACP {} failed: {err}",
                        AGENT_METHOD_NAMES.session_new
                    )),
                };
                log_undelivered(response_tx.send(result), AGENT_METHOD_NAMES.session_new);
            }
            ClientRequest::ListSessions { response_tx } => {
                let result: Result<ListSessionsResponse> = cx
                    .send_request(ListSessionsRequest::new())
                    .block_task()
                    .await
                    .map_err(anyhow::Error::from);
                log_undelivered(response_tx.send(result), AGENT_METHOD_NAMES.session_list);
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
            ClientRequest::SetModel {
                session_id,
                model_id,
                response_tx,
            } => {
                let result: Result<()> = cx
                    .send_request(SetSessionModelRequest::new(session_id, model_id))
                    .block_task()
                    .await
                    .map(|_| ())
                    .map_err(anyhow::Error::from);
                log_undelivered(
                    response_tx.send(result),
                    AGENT_METHOD_NAMES.session_set_model,
                );
            }
            ClientRequest::SetConfigOption {
                session_id,
                config_id,
                value,
                response_tx,
            } => {
                let value_id = sacp::schema::SessionConfigValueId::new(value);
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
            ClientRequest::CloseSession {
                session_id,
                response_tx,
            } => {
                let result: Result<()> = cx
                    .send_request(CloseSessionRequest::new(session_id.clone()))
                    .block_task()
                    .await
                    .map(|_| ())
                    .map_err(anyhow::Error::from);
                session_ids.retain(|s| s != &session_id);
                log_undelivered(response_tx.send(result), AGENT_METHOD_NAMES.session_close);
            }
            ClientRequest::Untyped {
                method,
                params,
                response_tx,
            } => {
                let result: Result<serde_json::Value> =
                    match sacp::UntypedMessage::new(&method, params) {
                        Ok(msg) => cx
                            .send_request(msg)
                            .block_task()
                            .await
                            .map_err(anyhow::Error::from),
                        Err(e) => Err(anyhow::Error::from(e)),
                    };
                log_undelivered(response_tx.send(result), &method);
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

    // After loop exits (channel closed by Drop):
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

async fn apply_session_mode(
    config: &AcpProviderConfig,
    cx: &ConnectionTo<Agent>,
    session: NewSessionResponse,
) -> Result<NewSessionResponse> {
    if let (Some(mode_id), Some(modes)) = (config.session_mode_id.clone(), session.modes.as_ref()) {
        if modes.current_mode_id.0.as_ref() != mode_id.as_str() {
            let available: Vec<String> = modes
                .available_modes
                .iter()
                .map(|mode| mode.id.0.to_string())
                .collect();

            if !available.iter().any(|id| id == &mode_id) {
                return Err(anyhow::anyhow!(
                    "Requested mode '{}' not offered by agent. Available modes: {}",
                    mode_id,
                    available.join(", ")
                ));
            }
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

    Ok(session)
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

fn messages_to_prompt(messages: &[Message]) -> Vec<ContentBlock> {
    let mut content_blocks = Vec::new();

    let last_user = messages
        .iter()
        .rev()
        .find(|m| m.role == Role::User && m.is_agent_visible());

    if let Some(message) = last_user {
        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    content_blocks.push(ContentBlock::Text(TextContent::new(text.text.clone())));
                }
                MessageContent::Image(image) => {
                    content_blocks.push(ContentBlock::Image(ImageContent::new(
                        &image.data,
                        &image.mime_type,
                    )));
                }
                _ => {}
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

// config_options supersedes models; fall back to models.
fn resolve_model_info(
    provider_name: &str,
    response: &NewSessionResponse,
) -> Result<(String, Vec<String>), ProviderError> {
    if let Some(opts) = &response.config_options {
        if let Some(sel) = opts.iter().find_map(|opt| {
            if opt.category.as_ref() != Some(&SessionConfigOptionCategory::Model) {
                return None;
            }
            match &opt.kind {
                SessionConfigKind::Select(s) => Some(s),
                _ => None,
            }
        }) {
            let current = sel.current_value.0.to_string();
            let available = match &sel.options {
                SessionConfigSelectOptions::Ungrouped(opts) => {
                    opts.iter().map(|o| o.value.0.to_string()).collect()
                }
                SessionConfigSelectOptions::Grouped(groups) => groups
                    .iter()
                    .flat_map(|g| g.options.iter().map(|o| o.value.0.to_string()))
                    .collect(),
                _ => vec![],
            };
            return Ok((current, available));
        }
    }

    let models = response.models.as_ref().ok_or_else(|| {
        ProviderError::RequestFailed(format!(
            "{provider_name}: agent returned neither config_options nor models"
        ))
    })?;
    let current = models.current_model_id.0.to_string();
    let available = models
        .available_models
        .iter()
        .map(|am| am.model_id.0.to_string())
        .collect();
    Ok((current, available))
}

fn reverse_mode_mapping(
    mode_mapping: &HashMap<GooseMode, String>,
) -> HashMap<String, Vec<GooseMode>> {
    let mut reverse: HashMap<String, Vec<GooseMode>> = HashMap::new();
    for (mode, id) in mode_mapping {
        reverse.entry(id.clone()).or_default().push(*mode);
    }
    reverse
}

// When multiple GooseModes map to the same provider ID (e.g. codex "read-only"),
// prefer the current mode if it's among candidates.
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

// TODO: ID mapping is in-memory only — sessions from prior runs or other clients are dropped.
// Persisting requires a schema change to map goose↔ACP IDs in the session DB.
fn map_sessions_to_goose_ids(
    response: ListSessionsResponse,
    acp_to_goose: &HashMap<String, String>,
) -> ListSessionsResponse {
    let sessions = response
        .sessions
        .into_iter()
        .filter_map(|mut info| {
            let goose_id = acp_to_goose.get(info.session_id.0.as_ref())?;
            info.session_id = SessionId::new(goose_id.clone());
            Some(info)
        })
        .collect();
    ListSessionsResponse::new(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::extension::Envs;
    use sacp::schema::{SessionConfigOption, SessionConfigSelectOption, SessionInfo};
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

    #[test_case(
        ListSessionsResponse::new(vec![
            SessionInfo::new(SessionId::new("20260318_1"), "/Users/codefromthecrypt/oss/goose-2")
                .title("Fix login bug".to_string())
                .updated_at("2026-03-18T07:02:42.549655Z".to_string()),
            SessionInfo::new(SessionId::new("20260318_2"), "/tmp/test-acpx")
                .title("Add caching layer".to_string())
                .updated_at("2026-03-18T07:05:01.123Z".to_string()),
        ]),
        HashMap::from([
            ("20260318_1".to_string(), "goose-session-1".to_string()),
            ("20260318_2".to_string(), "goose-session-2".to_string()),
        ]),
        ListSessionsResponse::new(vec![
            SessionInfo::new(SessionId::new("goose-session-1"), "/Users/codefromthecrypt/oss/goose-2")
                .title("Fix login bug".to_string())
                .updated_at("2026-03-18T07:02:42.549655Z".to_string()),
            SessionInfo::new(SessionId::new("goose-session-2"), "/tmp/test-acpx")
                .title("Add caching layer".to_string())
                .updated_at("2026-03-18T07:05:01.123Z".to_string()),
        ])
        ; "all sessions mapped with all fields preserved"
    )]
    #[test_case(
        ListSessionsResponse::new(vec![
            SessionInfo::new(SessionId::new("20260318_1"), "/Users/codefromthecrypt/oss/goose-2")
                .title("Fix login bug".to_string()),
            SessionInfo::new(SessionId::new("other-agent-session"), "/tmp/other")
                .title("Not our session".to_string()),
        ]),
        HashMap::from([
            ("20260318_1".to_string(), "goose-session-1".to_string()),
        ]),
        ListSessionsResponse::new(vec![
            SessionInfo::new(SessionId::new("goose-session-1"), "/Users/codefromthecrypt/oss/goose-2")
                .title("Fix login bug".to_string()),
        ])
        ; "unmapped sessions filtered out"
    )]
    #[test_case(
        ListSessionsResponse::new(vec![
            SessionInfo::new(SessionId::new("20260318_1"), "/Users/codefromthecrypt/oss/goose-2")
                .title("ACP Session".to_string())
                .updated_at("2026-03-18T01:29:02.141700Z".to_string()),
        ]),
        HashMap::new(),
        ListSessionsResponse::new(vec![])
        ; "empty map returns empty list"
    )]
    fn test_map_sessions_to_goose_ids(
        response: ListSessionsResponse,
        acp_to_goose: HashMap<String, String>,
        expected: ListSessionsResponse,
    ) {
        let result = map_sessions_to_goose_ids(response, &acp_to_goose);
        assert_eq!(result, expected);
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
            (GooseMode::Auto, "yolo".to_string()),
            (GooseMode::Approve, "default".to_string()),
            (GooseMode::SmartApprove, "auto_edit".to_string()),
            (GooseMode::Chat, "plan".to_string()),
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
            (GooseMode::Auto, "bypassPermissions".to_string()),
            (GooseMode::Approve, "default".to_string()),
            (GooseMode::SmartApprove, "acceptEdits".to_string()),
            (GooseMode::Chat, "plan".to_string()),
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
            (GooseMode::Auto, "full-access".to_string()),
            (GooseMode::Approve, "read-only".to_string()),
            (GooseMode::SmartApprove, "auto".to_string()),
            (GooseMode::Chat, "read-only".to_string()),
        ]),
        HashMap::from([
            ("full-access".to_string(), vec![GooseMode::Auto]),
            ("read-only".to_string(), vec![GooseMode::Approve, GooseMode::Chat]),
            ("auto".to_string(), vec![GooseMode::SmartApprove]),
        ])
        ; "codex duplicate read-only"
    )]
    fn test_reverse_mode_mapping(
        forward: HashMap<GooseMode, String>,
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
            .models(sacp::schema::SessionModelState::new(
                "default",
                vec![
                    sacp::schema::ModelInfo::new("default", "Default (recommended)"),
                    sacp::schema::ModelInfo::new("sonnet", "Sonnet"),
                    sacp::schema::ModelInfo::new("haiku", "Haiku"),
                ],
            ))
            .config_options(vec![
                SessionConfigOption::select("model", "Model", "default", vec![
                    SessionConfigSelectOption::new("default", "Default (recommended)"),
                    SessionConfigSelectOption::new("sonnet", "Sonnet"),
                    SessionConfigSelectOption::new("haiku", "Haiku"),
                ])
                .category(SessionConfigOptionCategory::Model),
            ])
        => Ok(("default".to_string(), vec!["default".to_string(), "sonnet".to_string(), "haiku".to_string()]))
        ; "claude-agent-acp config_options supersedes models"
    )]
    #[test_case(
        NewSessionResponse::new("s1")
            .models(sacp::schema::SessionModelState::new(
                "auto-gemini-3",
                vec![
                    sacp::schema::ModelInfo::new("auto-gemini-3", "Auto (Gemini 3)"),
                    sacp::schema::ModelInfo::new("auto-gemini-2.5", "Auto (Gemini 2.5)"),
                    sacp::schema::ModelInfo::new("gemini-2.5-pro", "gemini-2.5-pro"),
                ],
            ))
        => Ok(("auto-gemini-3".to_string(), vec!["auto-gemini-3".to_string(), "auto-gemini-2.5".to_string(), "gemini-2.5-pro".to_string()]))
        ; "gemini-acp falls back to models"
    )]
    #[test_case(
        NewSessionResponse::new("s1")
        => Err(ProviderError::RequestFailed(
            "test: agent returned neither config_options nor models".to_string()
        ))
        ; "neither config_options nor models is an error"
    )]
    fn test_resolve_model_info(
        response: NewSessionResponse,
    ) -> Result<(String, Vec<String>), ProviderError> {
        resolve_model_info("test", &response)
    }

    // Codex mapping: read-only maps to both Approve and Chat.
    fn codex_reverse_modes() -> HashMap<String, Vec<GooseMode>> {
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
        let reverse_modes = codex_reverse_modes();
        let current = Arc::new(Mutex::new(current));
        let result = resolve_mode(&reverse_modes, mode_id, &current);
        // For the fallback case, just check we got *some* candidate (order is nondeterministic).
        if mode_id == "read-only" && expected == Some(GooseMode::Approve) {
            // Current (Auto) not in candidates — any candidate is valid.
            assert!(result == Some(GooseMode::Approve) || result == Some(GooseMode::Chat));
        } else {
            assert_eq!(result, expected);
        }
    }
}
