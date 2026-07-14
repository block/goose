use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    LoggingLevel, LoggingMessageNotification, LoggingMessageNotificationParam, ServerCapabilities,
    ServerNotification, Tool,
};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use futures::StreamExt;

use crate::action_required_manager::ActionRequiredManager;
use crate::agents::extension::ExtensionConfig;
use crate::agents::mcp_client::{Error as McpError, McpClientTrait};
use crate::agents::state_machine;
use crate::agents::tool_execution::ToolCallContext;
use crate::agents::types::SessionConfig;
use crate::agents::{Agent, AgentConfig, AgentEvent, GoosePlatform};
use crate::config::permission::PermissionManager;
use crate::config::GooseMode;
use crate::conversation::message::Message;
use crate::providers::base::{MessageStream, Provider};
use crate::session::{Session, SessionManager, SessionType};
use goose_providers::conversation::token_usage::{ProviderUsage, Usage as ProviderTokenUsage};
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;

/// What a [`ScriptedProvider`] emits for one `stream()` call.
pub enum Step {
    /// A single assistant text message.
    Text(String),
    /// An assistant message requesting one tool call.
    ToolCall {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    /// Arbitrary pre-built messages emitted as one stream.
    Messages(Vec<Message>),
    /// The provider fails this call with the given error.
    Error(ProviderError),
}

impl Step {
    fn into_outcome(self) -> Result<Vec<Message>, ProviderError> {
        match self {
            Step::Text(text) => Ok(vec![Message::assistant().with_text(text)]),
            Step::ToolCall { id, name, args } => {
                let call = rmcp::model::CallToolRequestParams::new(name)
                    .with_arguments(args.as_object().cloned().unwrap_or_default());
                Ok(vec![Message::assistant().with_tool_request(id, Ok(call))])
            }
            Step::Messages(messages) => Ok(messages),
            Step::Error(err) => Err(err),
        }
    }
}

type ScriptFn = dyn Fn(&[Message], &[Tool]) -> Result<Vec<Message>, ProviderError> + Send + Sync;

/// A reusable provider whose responses are scripted as data or via a callback,
/// replacing per-test bespoke `impl Provider` mocks.
pub struct ScriptedProvider {
    script: Box<ScriptFn>,
    calls: AtomicUsize,
}

impl ScriptedProvider {
    /// Build from a fixed sequence of steps, one consumed per `stream()` call.
    pub fn from_steps(steps: impl IntoIterator<Item = Step>) -> Self {
        // Steps become messages lazily, at the call, so their `created` stamps
        // follow the conversation. Messages pre-built here would all carry the
        // construction time, and the persisted conversation is ordered by
        // `created_timestamp` (second resolution) — once a test crosses a
        // second boundary, a pre-built reply sorts before the tool response it
        // answers and the loop reruns the LLM op on a user-tailed conversation.
        let queue: Mutex<VecDeque<Step>> = Mutex::new(steps.into_iter().collect());
        Self::from_fn_result(move |_messages, _tools| {
            queue
                .lock()
                .unwrap()
                .pop_front()
                .map(Step::into_outcome)
                .unwrap_or_else(|| {
                    Ok(vec![
                        Message::assistant().with_text("(no more scripted steps)")
                    ])
                })
        })
    }

    /// Build from a callback that sees the conversation and tools each call.
    pub fn from_fn(
        script: impl Fn(&[Message], &[Tool]) -> Vec<Message> + Send + Sync + 'static,
    ) -> Self {
        Self::from_fn_result(move |messages, tools| Ok(script(messages, tools)))
    }

    /// Build from a callback that may also fail the call with a `ProviderError`.
    pub fn from_fn_result(
        script: impl Fn(&[Message], &[Tool]) -> Result<Vec<Message>, ProviderError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            script: Box::new(script),
            calls: AtomicUsize::new(0),
        }
    }

    /// Number of times `stream()` has been called.
    pub fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Provider for ScriptedProvider {
    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let messages = (self.script)(messages, tools)?;
        let usage = ProviderUsage::new(
            "scripted-model".to_string(),
            ProviderTokenUsage::new(Some(10), Some(5), Some(15)),
        );
        let last = messages.len().saturating_sub(1);
        let stream =
            futures::stream::iter(messages.into_iter().enumerate().map(move |(i, msg)| {
                let u = if i == last { Some(usage.clone()) } else { None };
                Ok((Some(msg), u))
            }));
        Ok(Box::pin(stream))
    }

    fn get_name(&self) -> &str {
        "scripted"
    }
}

/// Behavior for one tool exposed by [`TestExtensionClient`].
pub enum TestToolBehavior {
    /// Echo the call arguments back as JSON text.
    Echo,
    /// Return a tool error with the given message.
    Error(String),
    /// Emit `count` log notifications, then succeed.
    Notify { count: usize },
    /// Block until the cancellation token fires, then succeed.
    SlowUntilCancelled,
    /// Request an elicitation and block until it is answered, then return the
    /// outcome as text.
    Elicit,
}

/// An in-process extension client for tests, injected via
/// `ExtensionManager::add_client` — a real `McpClientTrait`, not a transport mock.
pub struct TestExtensionClient {
    info: InitializeResult,
    tools: Vec<(String, TestToolBehavior)>,
    notification_tx: mpsc::Sender<ServerNotification>,
    notification_rx: Mutex<Option<mpsc::Receiver<ServerNotification>>>,
    // The session store's elicitation registry, filled in by
    // `TestHarness::with_extension`; the `Elicit` tool blocks on it.
    action_required: std::sync::OnceLock<Arc<ActionRequiredManager>>,
}

impl TestExtensionClient {
    pub fn new(tools: Vec<(String, TestToolBehavior)>) -> Self {
        let info = InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("test".to_string(), "1.0.0".to_string()));
        let (notification_tx, notification_rx) = mpsc::channel(32);
        Self {
            info,
            tools,
            notification_tx,
            notification_rx: Mutex::new(Some(notification_rx)),
            action_required: std::sync::OnceLock::new(),
        }
    }

    /// The default set covering the tool-execution op's code paths.
    /// Tool names are unprefixed; `list_tools` advertises them as `test__<name>`.
    pub fn with_default_tools() -> Self {
        Self::new(vec![
            ("echo".to_string(), TestToolBehavior::Echo),
            (
                "error".to_string(),
                TestToolBehavior::Error("boom".to_string()),
            ),
            ("notify".to_string(), TestToolBehavior::Notify { count: 2 }),
            ("slow".to_string(), TestToolBehavior::SlowUntilCancelled),
            ("elicit".to_string(), TestToolBehavior::Elicit),
        ])
    }
}

#[async_trait]
impl McpClientTrait for TestExtensionClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, McpError> {
        let tools = self
            .tools
            .iter()
            .map(|(name, _)| {
                Tool::new(
                    format!("test__{name}"),
                    "test tool",
                    std::sync::Arc::new(JsonObject::new()),
                )
            })
            .collect();
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<JsonObject>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        let behavior = self
            .tools
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b)
            .ok_or(McpError::TransportClosed)?;

        match behavior {
            TestToolBehavior::Echo => {
                let args = arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(json!({}));
                Ok(CallToolResult::success(vec![Content::text(
                    args.to_string(),
                )]))
            }
            TestToolBehavior::Error(message) => {
                Ok(CallToolResult::error(vec![Content::text(message.clone())]))
            }
            TestToolBehavior::Notify { count } => {
                for i in 0..*count {
                    let param = LoggingMessageNotificationParam {
                        level: LoggingLevel::Info,
                        logger: None,
                        data: json!({ "message": format!("notify {i}") }),
                    };
                    let notification = LoggingMessageNotification::new(param);
                    let _ = self
                        .notification_tx
                        .send(ServerNotification::LoggingMessageNotification(notification))
                        .await;
                }
                Ok(CallToolResult::success(vec![Content::text("notified")]))
            }
            TestToolBehavior::SlowUntilCancelled => {
                cancel_token.cancelled().await;
                Ok(CallToolResult::success(vec![Content::text("cancelled")]))
            }
            TestToolBehavior::Elicit => {
                let request_id = ctx
                    .tool_call_request_id
                    .clone()
                    .expect("elicit tool needs a tool call request id");
                let outcome = self
                    .action_required
                    .get()
                    .expect("TestExtensionClient added outside TestHarness::with_extension")
                    .request_and_wait(
                        ctx.session_id.clone(),
                        request_id,
                        "What should I use?".to_string(),
                        json!({ "type": "object" }),
                        std::time::Duration::from_secs(10),
                    )
                    .await;
                match outcome {
                    Ok(crate::action_required_manager::ElicitationOutcome::Accept(data)) => Ok(
                        CallToolResult::success(vec![Content::text(data.to_string())]),
                    ),
                    Ok(_) => Ok(CallToolResult::error(vec![Content::text(
                        "elicitation declined",
                    )])),
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
                }
            }
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        self.notification_rx
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(|| mpsc::channel(1).1)
    }
}

/// Drives the state-machine `reply` against a real `Agent` wired to a
/// [`ScriptedProvider`] and an optional [`TestExtensionClient`]. Owns the
/// session-manager temp dir and the agent setup so tests only express the
/// scenario.
pub struct TestHarness {
    pub agent: Agent,
    pub session_id: String,
    pub provider: Arc<ScriptedProvider>,
    _temp_dir: tempfile::TempDir,
}

impl TestHarness {
    /// Build a harness whose provider replays the given fixed steps.
    pub async fn with_steps(steps: impl IntoIterator<Item = Step>) -> Self {
        Self::with_provider(Arc::new(ScriptedProvider::from_steps(steps))).await
    }

    pub async fn with_provider(provider: Arc<ScriptedProvider>) -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
        let config = AgentConfig::new(
            session_manager.clone(),
            Arc::new(PermissionManager::new(temp_dir.path().join("permissions"))),
            None,
            GooseMode::Auto,
            true,
            GoosePlatform::GooseCli,
        );
        let agent = Agent::with_config(config);
        let session = session_manager
            .create_session(
                PathBuf::default(),
                "sm-test".to_string(),
                SessionType::Hidden,
                GooseMode::default(),
            )
            .await
            .unwrap();
        let session_id = session.id.clone();
        agent
            .update_provider(
                provider.clone(),
                ModelConfig::new("scripted-model"),
                &session_id,
            )
            .await
            .unwrap();
        Self {
            agent,
            session_id,
            provider,
            _temp_dir: temp_dir,
        }
    }

    /// Inject the default in-process test extension (`test__echo`, `test__error`,
    /// `test__notify`, `test__slow`).
    pub async fn with_default_extension(self) -> Self {
        self.with_extension(TestExtensionClient::with_default_tools())
            .await
    }

    pub async fn with_extension(self, client: TestExtensionClient) -> Self {
        let _ = client
            .action_required
            .set(self.agent.config.session_manager.action_required());
        let info = client.get_info().cloned();
        self.agent
            .extension_manager
            .add_client(
                "test".to_string(),
                ExtensionConfig::Platform {
                    name: "test".to_string(),
                    description: "test extension".to_string(),
                    display_name: None,
                    bundled: None,
                    available_tools: vec![],
                },
                Arc::new(client),
                info,
                None,
            )
            .await;
        self
    }

    pub async fn with_goose_mode(self, mode: GooseMode) -> Self {
        self.agent
            .update_goose_mode(mode, &self.session_id)
            .await
            .unwrap();
        self
    }

    #[cfg(test)]
    pub fn with_hook_manager(mut self, hook_manager: crate::hooks::HookManager) -> Self {
        self.agent.set_hook_manager_for_test(hook_manager);
        self
    }

    #[cfg(test)]
    pub fn with_stop_hook_block_cap(mut self, cap: u32) -> Self {
        self.agent.set_stop_hook_block_cap_for_test(cap);
        self
    }

    /// Set the session's recorded token total, used by proactive compaction.
    pub async fn set_total_tokens(&self, tokens: i32) {
        self.agent
            .config
            .session_manager
            .update(&self.session_id)
            .usage(ProviderTokenUsage::new(None, None, Some(tokens)))
            .apply()
            .await
            .unwrap();
    }

    fn session_config(&self, max_turns: u32) -> SessionConfig {
        SessionConfig {
            id: self.session_id.clone(),
            schedule_id: None,
            max_turns: Some(max_turns),
            retry_config: None,
        }
    }

    /// Run `reply` to completion and collect the emitted `Message` events.
    pub async fn run(&self, prompt: &str, max_turns: u32) -> anyhow::Result<Vec<Message>> {
        Ok(self
            .run_events(prompt, max_turns)
            .await?
            .into_iter()
            .filter_map(|e| match e {
                AgentEvent::Message(m) => Some(m),
                _ => None,
            })
            .collect())
    }

    /// Run `reply` to completion and collect every emitted event.
    pub async fn run_events(
        &self,
        prompt: &str,
        max_turns: u32,
    ) -> anyhow::Result<Vec<AgentEvent>> {
        let stream = state_machine::reply(
            &self.agent,
            Message::user().with_text(prompt),
            self.session_config(max_turns),
            None,
        )
        .await?;
        tokio::pin!(stream);

        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event?);
        }
        Ok(events)
    }

    /// Reload the persisted session from disk.
    pub async fn reload(&self) -> anyhow::Result<Session> {
        self.agent
            .config
            .session_manager
            .get_session(&self.session_id, true)
            .await
    }

    /// The persisted conversation messages.
    pub async fn persisted_messages(&self) -> anyhow::Result<Vec<Message>> {
        Ok(self
            .reload()
            .await?
            .conversation
            .unwrap()
            .messages()
            .to_vec())
    }
}

/// Extract the concatenated text from a tool-response message, including the
/// error message of a failed result.
pub fn tool_response_text(message: &Message) -> String {
    use crate::conversation::message::MessageContent;
    message
        .content
        .iter()
        .filter_map(|c| match c {
            MessageContent::ToolResponse(r) => Some(match r.tool_result.as_ref() {
                Ok(res) => res
                    .content
                    .iter()
                    .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                    .collect::<String>(),
                Err(e) => e.message.to_string(),
            }),
            _ => None,
        })
        .collect()
}
