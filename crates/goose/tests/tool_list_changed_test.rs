//! End-to-end reproducer for the dropped `notifications/tools/list_changed` bug.
//!
//! This drives a real MCP stdio server (`tests/dynamic_tools_server.py`) through
//! goose's own extension machinery and rmcp transport — no mocks, no internal-API
//! pokes. The server advertises `tools.listChanged = true`, serves a single tool
//! `alpha`, and on the first `alpha` call sends `notifications/tools/list_changed`
//! and begins serving a second tool, `beta`.
//!
//! We observe the PUBLIC tool list (`get_prefixed_tools`, i.e. what the agent would
//! see) and assert `beta` is absent before the notification and present after.
//!
//! Before the fix (`GooseClient` never overrides `on_tool_list_changed`) the
//! notification is dropped, the cache stays at `[alpha]`, and this test TIMES OUT
//! waiting for `beta` — i.e. it FAILS on unpatched main. With the fix it PASSES.
//!
//! Requires `python3` on PATH (as the existing fastmcp integration test requires
//! `uv`). The test uses only public, pre-fix APIs so it also compiles on main.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rmcp::model::CallToolRequestParams;
use tokio_util::sync::CancellationToken;

use async_trait::async_trait;
use goose::agents::extension::{Envs, ExtensionConfig};
use goose::agents::extension_manager::{ExtensionManager, ExtensionManagerCapabilities};
use goose::agents::GoosePlatform;
use goose::conversation::message::Message;
use goose::providers::base::{
    stream_from_single_message, MessageStream, Provider, ProviderDef, ProviderMetadata,
};
use goose_providers::conversation::token_usage::{ProviderUsage, Usage};
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use rmcp::model::Tool;

#[derive(Clone, Default)]
struct MockProvider;

impl MockProvider {
    fn new() -> Self {
        Self
    }
}

impl goose::providers::base::ProviderDescriptor for MockProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::empty()
    }
}

impl ProviderDef for MockProvider {
    type Provider = Self;

    fn from_env(
        _extensions: Vec<goose::config::ExtensionConfig>,
        _tls_config: Option<goose::providers::api_client::TlsConfig>,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<Self>> {
        Box::pin(async move { Ok(Self::new()) })
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn get_name(&self) -> &str {
        "mock"
    }

    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let message = Message::assistant().with_text("mock");
        let usage = ProviderUsage::new("mock".to_string(), Usage::default());
        Ok(stream_from_single_message(message, usage))
    }
}

fn dynamic_tools_server_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("dynamic_tools_server.py")
}

async fn tool_names(extension_manager: &ExtensionManager) -> Vec<String> {
    extension_manager
        .get_prefixed_tools("test-session-id", None)
        .await
        .expect("get_prefixed_tools failed")
        .iter()
        .map(|t| t.name.to_string())
        .collect()
}

#[tokio::test]
async fn tool_list_changed_refreshes_public_tool_list() {
    let server_path = dynamic_tools_server_path();
    assert!(
        server_path.exists(),
        "reproducer server missing: {}",
        server_path.display()
    );

    let extension_config = ExtensionConfig::Stdio {
        name: "dynamic".to_string(),
        description: "dynamic tools reproducer".to_string(),
        cmd: "python3".to_string(),
        args: vec![server_path.to_string_lossy().to_string()],
        envs: Envs::new(std::collections::HashMap::new()),
        env_keys: vec![],
        timeout: Some(30),
        cwd: None,
        bundled: Some(false),
        available_tools: vec![],
    };

    let provider = Arc::new(tokio::sync::Mutex::new(Some(
        Arc::new(MockProvider::new()) as Arc<dyn Provider>
    )));
    let temp_dir = tempfile::tempdir().unwrap();
    let session_manager = Arc::new(goose::session::SessionManager::new(
        temp_dir.path().to_path_buf(),
    ));
    let extension_manager = Arc::new(ExtensionManager::new(
        provider,
        session_manager,
        GoosePlatform::GooseDesktop.to_string(),
        ExtensionManagerCapabilities {
            mcpui: true,
            host_info: None,
        },
        true,
    ));

    extension_manager
        .add_extension(extension_config, None, None, None)
        .await
        .expect("add_extension failed (is python3 on PATH?)");

    // Before the notification: only `alpha` is published.
    let names = tool_names(&extension_manager).await;
    assert!(
        names.iter().any(|n| n == "dynamic__alpha"),
        "expected dynamic__alpha in {names:?}"
    );
    assert!(
        !names.iter().any(|n| n == "dynamic__beta"),
        "dynamic__beta should not exist yet, got {names:?}"
    );

    // Call `alpha`; the server responds and emits notifications/tools/list_changed,
    // then starts publishing `beta`.
    let ctx = goose::agents::ToolCallContext::new(
        "test-session-id".to_string(),
        None,
        Some("test-id".to_string()),
    );
    let call = CallToolRequestParams::new("dynamic__alpha");
    let result = extension_manager
        .dispatch_tool_call(&ctx, call, CancellationToken::default())
        .await
        .expect("dispatch_tool_call failed");
    result.result.await.expect("alpha call errored");

    // Bounded wait for the public tool list to reflect the notification. The
    // notification is handled asynchronously on the client's event loop, so we
    // poll rather than sleep a fixed amount. On unpatched main the notification is
    // dropped and this loop never observes `beta`, so the test fails on timeout.
    let deadline = Duration::from_secs(10);
    let mut beta_visible = false;
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        let names = tool_names(&extension_manager).await;
        if names.iter().any(|n| n == "dynamic__beta") {
            beta_visible = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        beta_visible,
        "dynamic__beta never appeared after notifications/tools/list_changed — \
         the notification was dropped (bug present)"
    );
}
