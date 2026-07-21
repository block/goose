//! Regression test: a `notifications/tools/list_changed` that arrives *during* an
//! agent reply must reach the model within that same reply — not only on the next
//! one.
//!
//! The companion `tool_list_changed_test` proves the shared `ExtensionManager`
//! cache is invalidated (so the next reply/turn sees new tools). This test proves
//! the *active reply loop* also picks the change up: `agent.rs` snapshots the tool
//! list up front, so without propagating the invalidation into the loop, a tool
//! call that unlocks `beta` mid-reply wouldn't offer `beta` to the model until a
//! later reply.
//!
//! Setup: a scripted provider calls `dynamic__alpha` (which makes the stdio server
//! emit `tools/list_changed` and start serving `beta`) and keeps calling it until
//! it is offered `dynamic__beta`, then stops. We record the tools handed to the
//! provider on each turn and assert `beta` shows up within the same reply.
//!
//! Before the fix (loop keeps passing its stale `tools` snapshot) `beta` never
//! appears and the reply runs to `max_turns` — i.e. this FAILS. With the fix it
//! PASSES. Requires `python3` on PATH.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use rmcp::model::{CallToolRequestParams, Tool};

use goose::agents::extension::{Envs, ExtensionConfig};
use goose::agents::{Agent, AgentConfig, GoosePlatform, SessionConfig};
use goose::config::permission::PermissionManager;
use goose::config::GooseMode;
use goose::conversation::message::Message;
use goose::providers::base::{
    stream_from_single_message, MessageStream, Provider, ProviderDef, ProviderMetadata,
};
use goose::session::session_manager::SessionType;
use goose::session::SessionManager;
use goose_providers::conversation::token_usage::{ProviderUsage, Usage};
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;

/// Provider that records the tool names it is offered each turn, and drives the
/// agentic loop by calling `dynamic__alpha` until it sees `dynamic__beta`.
struct RecordingProvider {
    tools_per_turn: Arc<Mutex<Vec<Vec<String>>>>,
    call_id: AtomicUsize,
}

impl RecordingProvider {
    fn new(tools_per_turn: Arc<Mutex<Vec<Vec<String>>>>) -> Self {
        Self {
            tools_per_turn,
            call_id: AtomicUsize::new(0),
        }
    }
}

impl goose::providers::base::ProviderDescriptor for RecordingProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::empty()
    }
}

impl ProviderDef for RecordingProvider {
    type Provider = Self;

    fn from_env(
        _extensions: Vec<goose::config::ExtensionConfig>,
        _tls_config: Option<goose::providers::api_client::TlsConfig>,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<Self>> {
        unimplemented!("RecordingProvider is constructed directly in the test")
    }
}

#[async_trait]
impl Provider for RecordingProvider {
    fn get_name(&self) -> &str {
        "recording-mock"
    }

    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        _messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
        let has_beta = names.iter().any(|n| n == "dynamic__beta");
        self.tools_per_turn.lock().unwrap().push(names);

        let usage = ProviderUsage::new("recording-mock".to_string(), Usage::default());
        let message = if has_beta {
            // Goal reached — end the reply with a plain assistant message.
            Message::assistant().with_text("beta is available")
        } else {
            // Keep the agentic loop going by calling alpha. The first call makes
            // the server emit tools/list_changed and publish beta; later calls are
            // harmless no-ops that just give the loop another iteration to observe
            // the (asynchronously delivered) refresh.
            let id = self.call_id.fetch_add(1, Ordering::SeqCst);
            Message::assistant().with_tool_request(
                format!("call_{id}"),
                Ok(CallToolRequestParams::new("dynamic__alpha")),
            )
        };
        Ok(stream_from_single_message(message, usage))
    }
}

fn dynamic_tools_server_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("dynamic_tools_server.py")
}

#[tokio::test]
async fn tool_list_changed_refreshes_active_reply_loop() {
    let server_path = dynamic_tools_server_path();
    assert!(
        server_path.exists(),
        "reproducer server missing: {}",
        server_path.display()
    );

    // Isolate the server's "unlocked" sentinel to a temp path so `beta` starts
    // locked on every run (the server persists it next to the script by default).
    let state_dir = tempfile::tempdir().unwrap();
    let mut envs = HashMap::new();
    envs.insert(
        "DYNAMIC_TOOLS_STATE".to_string(),
        state_dir
            .path()
            .join("beta_unlocked")
            .to_string_lossy()
            .to_string(),
    );

    let extension_config = ExtensionConfig::Stdio {
        name: "dynamic".to_string(),
        description: "dynamic tools reproducer".to_string(),
        cmd: "python3".to_string(),
        args: vec![server_path.to_string_lossy().to_string()],
        envs: Envs::new(envs),
        env_keys: vec![],
        timeout: Some(30),
        cwd: None,
        bundled: Some(false),
        available_tools: vec![],
    };

    let tools_per_turn = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));

    // Auto mode so tool calls run without confirmation prompts.
    let temp_dir = tempfile::tempdir().unwrap();
    let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
    let config = AgentConfig::new(
        session_manager.clone(),
        PermissionManager::instance(),
        None,
        GooseMode::Auto,
        false,
        GoosePlatform::GooseCli,
    );
    let agent = Agent::with_config(config);

    let session = session_manager
        .create_session(
            PathBuf::from("."),
            "tlc-reply-loop".to_string(),
            SessionType::Hidden,
            GooseMode::Auto,
        )
        .await
        .expect("create_session failed");

    agent
        .update_provider(
            Arc::new(RecordingProvider::new(tools_per_turn.clone())),
            ModelConfig::new("recording-mock"),
            &session.id,
        )
        .await
        .expect("update_provider failed");

    agent
        .add_extension(extension_config, &session.id)
        .await
        .expect("add_extension failed (is python3 on PATH?)");

    let session_config = SessionConfig {
        id: session.id.clone(),
        schedule_id: None,
        // Bounds the buggy case: without the fix `beta` never appears and the
        // provider keeps calling alpha, so cap the loop instead of hanging.
        max_turns: Some(20),
        retry_config: None,
    };

    let stream = agent
        .reply(Message::user().with_text("go"), session_config, None)
        .await
        .expect("reply failed");
    tokio::pin!(stream);
    while let Some(event) = stream.next().await {
        match event {
            Ok(_) => {}
            Err(e) => panic!("reply stream errored: {e}"),
        }
    }

    let turns = tools_per_turn.lock().unwrap().clone();
    assert!(!turns.is_empty(), "provider was never called");

    // Baseline: the first turn sees alpha but not beta.
    assert!(
        turns[0].iter().any(|n| n == "dynamic__alpha"),
        "expected dynamic__alpha on the first turn, got {:?}",
        turns[0]
    );
    assert!(
        !turns[0].iter().any(|n| n == "dynamic__beta"),
        "dynamic__beta should not be offered on the first turn, got {:?}",
        turns[0]
    );

    // The fix: beta becomes visible to the model within the same reply.
    let beta_turn = turns
        .iter()
        .position(|t| t.iter().any(|n| n == "dynamic__beta"));
    assert!(
        beta_turn.is_some(),
        "dynamic__beta was never offered to the model within the reply ({} turns) — \
         the list_changed refresh did not reach the active reply loop",
        turns.len()
    );
}
