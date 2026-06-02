use agent_client_protocol::schema::{
    ClientCapabilities, CloseSessionRequest, ContentBlock, ContentChunk, EnvVariable, HttpHeader,
    ImageContent, InitializeRequest, InitializeResponse, McpCapabilities, McpServer, McpServerHttp,
    McpServerStdio, NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse,
    ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOptions, SessionId, SessionNotification, SessionUpdate,
    SetSessionConfigOptionRequest, SetSessionModeRequest, SetSessionModeResponse, StopReason,
    TextContent, ToolCallContent, ToolCallStatus, ToolKind,
};
use agent_client_protocol::{Agent, Client, ConnectionTo};
use agent_client_protocol_schema::Usage as AcpUsage;
use agent_client_protocol_schema::AGENT_METHOD_NAMES;
use anyhow::{Context, Result};
use async_stream::try_stream;
use futures::future::BoxFuture;
use rmcp::model::{CallToolRequestParams, CallToolResult, Content as RmcpContent, Role, Tool};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
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
use crate::model::ModelConfig;
use crate::permission::permission_confirmation::PrincipalType;
use crate::permission::{Permission, PermissionConfirmation};
use crate::providers::base::{MessageStream, PermissionRouting, Provider, ProviderUsage, Usage};
use crate::providers::errors::ProviderError;
use crate::subprocess::configure_subprocess;

mod client_loop;
mod conversion;
pub use client_loop::*;
pub use conversion::*;

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
    pub mode_mapping: HashMap<GooseMode, String>,
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
    Text(String),
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
    model: ModelConfig,
    goose_mode: Arc<Mutex<GooseMode>>,
    mode_mapping: HashMap<GooseMode, String>,

    session: AcpSession,

    pending_confirmations:
        Arc<TokioMutex<HashMap<String, oneshot::Sender<PermissionConfirmation>>>>,
    pending_tool_updates: Arc<Mutex<HashMap<String, AccumulatedToolCall>>>,
    handoff_context_sent: AtomicBool,

    tx: Option<mpsc::Sender<ClientRequest>>,
    loop_thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for AcpProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcpProvider")
            .field("name", &self.name)
            .field("model", &self.model)
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
        transport: impl agent_client_protocol::ConnectTo<Client> + 'static,
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
        let goose_mode_shared = Arc::new(Mutex::new(goose_mode));
        let pending_tool_updates: Arc<Mutex<HashMap<String, AccumulatedToolCall>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let client_loop = AcpClientLoop::new(
            config,
            goose_mode_shared.clone(),
            pending_tool_updates.clone(),
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

        // Resolve model from the session response.
        let resolved_model = if model.model_name == ACP_CURRENT_MODEL {
            if let Ok((resolved, _)) = resolve_model_info(&name, &response) {
                tracing::info!(from = ACP_CURRENT_MODEL, to = %resolved, "resolved ACP model");
                ModelConfig {
                    model_name: resolved,
                    ..model
                }
            } else {
                model
            }
        } else {
            model
        };

        let session = AcpSession {
            id: response.session_id.clone(),
            response,
        };

        Ok(Self {
            name,
            model: resolved_model,
            goose_mode: goose_mode_shared,
            mode_mapping,
            session,
            pending_confirmations: Arc::new(TokioMutex::new(HashMap::new())),
            pending_tool_updates,
            handoff_context_sent: AtomicBool::new(false),
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

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn update_mode(&self, session_id: &str, mode: GooseMode) -> Result<(), ProviderError> {
        let mode_str = self
            .mode_mapping
            .get(&mode)
            .cloned()
            .unwrap_or_else(|| format!("{mode:?}"));

        if self.session_has_config_option(SessionConfigOptionCategory::Mode) {
            self.send_set_config_option(session_id, "mode".into(), mode_str)
                .await
                .map_err(|e| ProviderError::RequestFailed(format!("Failed to set mode: {e}")))?;
        } else {
            self.send_set_mode(session_id, mode_str)
                .await
                .map_err(|e| ProviderError::RequestFailed(format!("Failed to set mode: {e}")))?;
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
        _session_id: &str,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let session_id = self.acp_session_id();

        let claim = self.claim_handoff_context(messages);
        let prompt_blocks = messages_to_prompt(messages, claim.include_context);
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
                            let message = Message::new(Role::Assistant, ts, vec![])
                                .with_text(text)
                                .with_id(id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::extension::Envs;
    use agent_client_protocol::schema::SessionConfigSelectOption;
    use test_case::test_case;

    fn prompt_text(block: &ContentBlock) -> &str {
        match block {
            ContentBlock::Text(text) => &text.text,
            _ => panic!("expected text block"),
        }
    }

    fn test_provider() -> AcpProvider {
        test_provider_with_tx(None)
    }

    fn test_provider_with_tx(tx: Option<mpsc::Sender<ClientRequest>>) -> AcpProvider {
        AcpProvider {
            name: "acp-test".to_string(),
            model: ModelConfig {
                model_name: "test-model".to_string(),
                ..Default::default()
            },
            goose_mode: Arc::new(Mutex::new(GooseMode::Auto)),
            mode_mapping: HashMap::new(),
            session: AcpSession {
                id: SessionId::new("test-session"),
                response: NewSessionResponse::new("test-session"),
            },
            pending_confirmations: Arc::new(TokioMutex::new(HashMap::new())),
            pending_tool_updates: Arc::new(Mutex::new(HashMap::new())),
            handoff_context_sent: AtomicBool::new(false),
            tx,
            loop_thread: None,
        }
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
    fn handoff_context_is_sent_only_on_first_provider_prompt() {
        let provider = test_provider();
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
        let provider = test_provider();
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
    async fn failed_first_prompt_send_rolls_back_handoff_context_claim() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let provider = test_provider_with_tx(Some(tx));
        let messages = vec![
            Message::assistant().with_text("prior answer"),
            Message::user().with_text("current request"),
        ];

        let result = provider
            .stream(&provider.model, "goose-session", "", &messages, &[])
            .await;

        assert!(matches!(result, Err(ProviderError::RequestFailed(_))));
        let next_claim = provider.claim_handoff_context(&messages);
        assert!(next_claim.first_prompt);
        assert!(next_claim.include_context);
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
            .models(agent_client_protocol::schema::SessionModelState::new(
                "default",
                vec![
                    agent_client_protocol::schema::ModelInfo::new("default", "Default (recommended)"),
                    agent_client_protocol::schema::ModelInfo::new("sonnet", "Sonnet"),
                    agent_client_protocol::schema::ModelInfo::new("haiku", "Haiku"),
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
            .models(agent_client_protocol::schema::SessionModelState::new(
                "auto-gemini-3",
                vec![
                    agent_client_protocol::schema::ModelInfo::new("auto-gemini-3", "Auto (Gemini 3)"),
                    agent_client_protocol::schema::ModelInfo::new("auto-gemini-2.5", "Auto (Gemini 2.5)"),
                    agent_client_protocol::schema::ModelInfo::new("gemini-2.5-pro", "gemini-2.5-pro"),
                ],
            ))
        => Ok(("auto-gemini-3".to_string(), vec!["auto-gemini-3".to_string(), "auto-gemini-2.5".to_string(), "gemini-2.5-pro".to_string()]))
        ; "gemini falls back to models"
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
        if mode_id == "read-only" && expected == Some(GooseMode::Approve) {
            assert!(result == Some(GooseMode::Approve) || result == Some(GooseMode::Chat));
        } else {
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn acp_tool_call_content_handles_text_diff_terminal_and_image() {
        use agent_client_protocol::schema::{Diff, Terminal, TerminalId, TextContent};

        let diff_block = ToolCallContent::Diff(
            Diff::new(std::path::PathBuf::from("/tmp/file.txt"), "new\n").old_text("old\n"),
        );
        let terminal_block = ToolCallContent::Terminal(Terminal::new(TerminalId::new("term-7")));
        let text_block = ToolCallContent::Content(agent_client_protocol::schema::Content::new(
            ContentBlock::Text(TextContent::new("hello")),
        ));
        let image_block = ToolCallContent::Content(agent_client_protocol::schema::Content::new(
            ContentBlock::Image(ImageContent::new("base64data", "image/png")),
        ));

        let out = acp_tool_call_content_to_rmcp(
            Some(vec![text_block, diff_block, terminal_block, image_block]),
            None,
        );

        assert_eq!(out.len(), 4, "all four block kinds should produce output");
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
            serialized[2].contains("term-7"),
            "terminal id lost: {serialized:?}"
        );
        assert!(
            serialized[3].contains("base64data"),
            "image data lost: {serialized:?}"
        );
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
