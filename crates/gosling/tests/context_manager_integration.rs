use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use gosling::agents::{Agent, AgentEvent, SessionConfig};
use gosling::config::GoslingMode;
use gosling::conversation::message::Message;
use gosling::conversation::Conversation;
use gosling::providers::base::{
    stream_from_single_message, MessageStream, Provider, ProviderDef, ProviderMetadata,
};
use gosling::session::session_manager::SessionType;
use gosling::session::Session;
use gosling_providers::conversation::token_usage::{ProviderUsage, Usage};
use gosling_providers::errors::ProviderError;
use gosling_providers::model::ModelConfig;
use rmcp::model::{AnnotateAble, CallToolRequestParams, CallToolResult, RawContent, Tool};
use serial_test::serial;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// Snapshot of what the provider actually saw for one `stream()` call.
struct CapturedCall {
    duplicate_tool_response_count: usize,
    saw_retrieved_memory: bool,
}

struct CapturingProvider {
    last_call: Arc<Mutex<Option<CapturedCall>>>,
}

#[async_trait]
impl Provider for CapturingProvider {
    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system_prompt: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let duplicate_tool_response_count = messages
            .iter()
            .flat_map(|m| m.content.iter())
            .filter_map(|c| c.as_tool_response_text())
            .filter(|text| text == "identical result")
            .count();
        let saw_retrieved_memory = messages
            .iter()
            .flat_map(|m| m.content.iter())
            .filter_map(|c| c.as_text())
            .any(|t| t.contains("[Retrieved memory"));

        *self.last_call.lock().unwrap() = Some(CapturedCall {
            duplicate_tool_response_count,
            saw_retrieved_memory,
        });

        Ok(stream_from_single_message(
            Message::assistant().with_text("ok"),
            ProviderUsage::new("mock-model".to_string(), Usage::default()),
        ))
    }

    fn get_name(&self) -> &str {
        "mock-capturing"
    }
}

impl gosling::providers::base::ProviderDescriptor for CapturingProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata {
            name: "mock-capturing".to_string(),
            display_name: "Mock Capturing Provider".to_string(),
            description: "Mock provider that records what it was called with".to_string(),
            default_model: "mock-model".to_string(),
            known_models: vec![],
            model_doc_link: "".to_string(),
            config_keys: vec![],
            setup_steps: vec![],
            model_selection_hint: None,
            fast_model: None,
        }
    }
}

impl ProviderDef for CapturingProvider {
    type Provider = Self;

    fn from_env(
        _extensions: Vec<gosling::config::ExtensionConfig>,
        _tls_config: Option<gosling::providers::api_client::TlsConfig>,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<Self>> {
        Box::pin(async {
            Ok(Self {
                last_call: Arc::new(Mutex::new(None)),
            })
        })
    }
}

/// A conversation whose earlier tool call was repeated verbatim later on,
/// which the Context Manager (in `on` mode) should recognize as duplicate
/// tool output and drop.
fn duplicate_tool_conversation() -> Vec<Message> {
    vec![
        Message::user().with_text("look this up twice please"),
        Message::assistant().with_tool_request("call1", Ok(CallToolRequestParams::new("search"))),
        Message::user().with_tool_response(
            "call1",
            Ok(CallToolResult::success(vec![RawContent::text(
                "identical result",
            )
            .no_annotation()])),
        ),
        Message::assistant().with_text("let me check once more"),
        Message::assistant().with_tool_request("call2", Ok(CallToolRequestParams::new("search"))),
        Message::user().with_tool_response(
            "call2",
            Ok(CallToolResult::success(vec![RawContent::text(
                "identical result",
            )
            .no_annotation()])),
        ),
        Message::user().with_text("thanks, anything else to add?"),
    ]
}

async fn setup_session(
    agent: &Agent,
    temp_dir: &TempDir,
    name: &str,
    messages: Vec<Message>,
) -> Result<Session> {
    let session = agent
        .config
        .session_manager
        .create_session(
            temp_dir.path().to_path_buf(),
            name.to_string(),
            SessionType::Hidden,
            GoslingMode::default(),
        )
        .await?;
    let conversation = Conversation::new_unvalidated(messages);
    agent
        .config
        .session_manager
        .replace_conversation(&session.id, &conversation)
        .await?;
    Ok(session)
}

/// Runs a full `agent.reply()` turn with `GOSLING_CONTEXT_MANAGER` set to
/// `mode` and returns what the provider actually received.
async fn run_and_capture(mode: &str) -> Result<CapturedCall> {
    std::env::set_var("GOSLING_CONTEXT_MANAGER", mode);

    let temp_dir = TempDir::new()?;
    let agent = Agent::new();
    let session = setup_session(
        &agent,
        &temp_dir,
        &format!("ctx-mgr-{mode}"),
        duplicate_tool_conversation(),
    )
    .await?;

    let last_call = Arc::new(Mutex::new(None));
    let provider = Arc::new(CapturingProvider {
        last_call: last_call.clone(),
    });
    agent
        .update_provider(provider, ModelConfig::new("mock-model"), &session.id)
        .await?;

    let session_config = SessionConfig {
        id: session.id.clone(),
        max_turns: None,
    };
    let reply_stream = agent
        .reply(Message::user().with_text("go ahead"), session_config, None)
        .await?;
    tokio::pin!(reply_stream);
    while let Some(event) = reply_stream.next().await {
        let _: AgentEvent = event?;
    }

    std::env::remove_var("GOSLING_CONTEXT_MANAGER");

    let captured = last_call.lock().unwrap().take();
    captured.ok_or_else(|| anyhow::anyhow!("provider was never called"))
}

#[tokio::test]
#[serial]
async fn off_mode_leaves_duplicate_tool_output_untouched() -> Result<()> {
    let captured = run_and_capture("off").await?;
    assert_eq!(
        captured.duplicate_tool_response_count, 2,
        "off mode must not alter provider input"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn shadow_mode_leaves_provider_input_untouched() -> Result<()> {
    let captured = run_and_capture("shadow").await?;
    assert_eq!(
        captured.duplicate_tool_response_count, 2,
        "shadow mode must build a packet but never change provider input"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn on_mode_drops_duplicate_tool_output() -> Result<()> {
    let captured = run_and_capture("on").await?;
    assert_eq!(
        captured.duplicate_tool_response_count, 1,
        "on mode should route provider input through the Context Manager and drop the earlier duplicate"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn on_mode_recalls_file_backed_memory() -> Result<()> {
    let memory_dir = TempDir::new()?;
    let memory_path = memory_dir.path().join("memories.jsonl");
    // The trailing user message in run_and_capture is "go ahead"; this entry
    // shares the keyword "ahead" so FileMemorySource recalls it.
    std::fs::write(
        &memory_path,
        r#"{"content": "when told to go ahead, the lookup results are cached upstream", "source": "note"}"#,
    )?;
    std::env::set_var("GOSLING_MEMORY_FILE", &memory_path);

    let on = run_and_capture("on").await;
    let shadow = run_and_capture("shadow").await;
    std::env::remove_var("GOSLING_MEMORY_FILE");

    assert!(
        on?.saw_retrieved_memory,
        "on mode should surface recalled memory to the provider"
    );
    assert!(
        !shadow?.saw_retrieved_memory,
        "shadow mode must keep recalled memory out of provider input"
    );
    Ok(())
}
