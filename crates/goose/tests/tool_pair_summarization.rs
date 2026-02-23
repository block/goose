use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use goose::agents::{Agent, AgentEvent, SessionConfig};
use goose::conversation::message::{Message, MessageContent};
use goose::conversation::Conversation;
use goose::model::ModelConfig;
use goose::providers::base::{
    stream_from_single_message, MessageStream, Provider, ProviderDef, ProviderMetadata,
    ProviderUsage, Usage,
};
use goose::providers::errors::ProviderError;
use goose::session::session_manager::SessionType;
use goose::session::Session;
use rmcp::model::{AnnotateAble, CallToolRequestParams, RawContent, Tool};
use serial_test::serial;
use std::sync::Arc;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Mock provider that recognises summarization calls via the system prompt
// ---------------------------------------------------------------------------

struct MockSummarizationProvider;

impl MockSummarizationProvider {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Provider for MockSummarizationProvider {
    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _session_id: &str,
        system_prompt: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        // complete_fast → complete → stream; the summarization path passes the
        // indoc system prompt containing "summarize a tool call".
        let is_summarization = system_prompt
            .to_lowercase()
            .contains("summarize a tool call");

        let message = if is_summarization {
            Message::assistant().with_text("A call to shell was made to list files")
        } else {
            // Regular reply — no tool requests so the agent loop exits.
            Message::assistant().with_text("Done.")
        };

        let usage = ProviderUsage::new(
            "mock-model".to_string(),
            Usage::new(Some(100), Some(50), Some(150)),
        );

        Ok(stream_from_single_message(message, usage))
    }

    fn get_model_config(&self) -> ModelConfig {
        ModelConfig::new("mock-model").unwrap()
    }

    fn get_name(&self) -> &str {
        "mock-summarization"
    }
}

impl ProviderDef for MockSummarizationProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata {
            name: "mock".to_string(),
            display_name: "Mock Summarization Provider".to_string(),
            description: "Mock provider for tool-pair summarization testing".to_string(),
            default_model: "mock-model".to_string(),
            known_models: vec![],
            model_doc_link: "".to_string(),
            config_keys: vec![],
            allows_unlisted_models: false,
        }
    }

    fn from_env(
        _model: ModelConfig,
        _extensions: Vec<goose::config::ExtensionConfig>,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<Self>> {
        Box::pin(async { Ok(Self::new()) })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a tool-request / tool-response pair linked by `call_id`.
/// Both messages carry `.with_id()` — required by the `msg.id.is_some()`
/// guard at agent.rs:1586.
fn create_tool_pair(
    call_id: &str,
    response_id: &str,
    tool_name: &str,
    response_text: &str,
) -> Vec<Message> {
    vec![
        Message::assistant()
            .with_tool_request(
                call_id,
                Ok(CallToolRequestParams {
                    task: None,
                    name: tool_name.to_string().into(),
                    arguments: None,
                    meta: None,
                }),
            )
            .with_id(call_id),
        Message::user()
            .with_tool_response(
                call_id,
                Ok(rmcp::model::CallToolResult {
                    content: vec![RawContent::text(response_text).no_annotation()],
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }),
            )
            .with_id(response_id),
    ]
}

/// Set up a session pre-populated with `messages` and sensible token counts.
async fn setup_test_session(
    agent: &Agent,
    temp_dir: &TempDir,
    session_name: &str,
    messages: Vec<Message>,
) -> Result<Session> {
    let session = agent
        .config
        .session_manager
        .create_session(
            temp_dir.path().to_path_buf(),
            session_name.to_string(),
            SessionType::Hidden,
        )
        .await?;

    let conversation = Conversation::new_unvalidated(messages);
    agent
        .config
        .session_manager
        .replace_conversation(&session.id, &conversation)
        .await?;

    agent
        .config
        .session_manager
        .update(&session.id)
        .total_tokens(Some(1000))
        .input_tokens(Some(600))
        .output_tokens(Some(400))
        .accumulated_total_tokens(Some(1000))
        .accumulated_input_tokens(Some(600))
        .accumulated_output_tokens(Some(400))
        .apply()
        .await?;

    Ok(session)
}

/// Build the initial conversation: one user message + `n` tool pairs.
fn build_conversation_with_tool_pairs(n: usize) -> Vec<Message> {
    let mut messages = vec![Message::user().with_text("list files").with_id("msg_user_0")];
    for i in 1..=n {
        messages.extend(create_tool_pair(
            &format!("call_{i}"),
            &format!("resp_{i}"),
            "shell",
            &format!("output from tool call {i}"),
        ));
    }
    messages
}

// ---------------------------------------------------------------------------
// Test 1: HistoryReplaced is emitted after tool-pair summarization
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_history_replaced_emitted_after_tool_pair_summarization() -> Result<()> {
    // cutoff=2 means summarization triggers when tool_call_count > 2.
    // We supply 3 tool pairs so the first one gets summarised.
    std::env::set_var("GOOSE_TOOL_CALL_CUTOFF", "2");

    let temp_dir = TempDir::new()?;
    let agent = Agent::new();

    let messages = build_conversation_with_tool_pairs(3);
    let session =
        setup_test_session(&agent, &temp_dir, "summarization-test", messages).await?;

    let mock_provider = Arc::new(MockSummarizationProvider::new());
    agent.update_provider(mock_provider, &session.id).await?;

    let session_config = SessionConfig {
        id: session.id.clone(),
        schedule_id: None,
        max_turns: Some(1),
        retry_config: None,
    };

    let new_user_message = Message::user()
        .with_text("continue")
        .with_id("msg_user_continue");

    let reply_stream = agent.reply(new_user_message, session_config, None).await?;
    tokio::pin!(reply_stream);

    let mut history_replaced_events: Vec<Conversation> = Vec::new();

    while let Some(event_result) = reply_stream.next().await {
        match event_result {
            Ok(AgentEvent::HistoryReplaced(conv)) => {
                history_replaced_events.push(conv);
            }
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    }

    // --- Assertions ---

    // 1. At least one HistoryReplaced event was emitted.
    assert!(
        !history_replaced_events.is_empty(),
        "Expected at least one HistoryReplaced event from tool-pair summarization"
    );

    let final_conv = history_replaced_events.last().unwrap();
    let msgs = final_conv.messages();

    // 2. There should be a hidden summary message (agent-visible, user-invisible).
    let hidden_summaries: Vec<&Message> = msgs
        .iter()
        .filter(|m: &&Message| !m.is_user_visible() && m.is_agent_visible())
        .collect();
    assert!(
        !hidden_summaries.is_empty(),
        "Expected at least one hidden summary message in the conversation"
    );

    // 3. The summary text should contain "shell" (from our mock response).
    let summary_text: String = hidden_summaries
        .iter()
        .flat_map(|m| m.content.iter())
        .filter_map(|c| match c {
            MessageContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        summary_text.contains("shell"),
        "Summary text should mention 'shell', got: {summary_text}"
    );

    // 4. The original first tool pair should be marked agent-invisible.
    let agent_invisible_msgs: Vec<&Message> = msgs
        .iter()
        .filter(|m: &&Message| !m.is_agent_visible())
        .collect();
    assert!(
        agent_invisible_msgs.len() >= 2,
        "Expected the original tool pair (2 messages) to be marked agent-invisible, found {}",
        agent_invisible_msgs.len()
    );

    std::env::remove_var("GOOSE_TOOL_CALL_CUTOFF");
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: Stale conversation_so_far overwrites hidden summaries
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_stale_conversation_overwrites_hidden_summary() -> Result<()> {
    std::env::set_var("GOOSE_TOOL_CALL_CUTOFF", "2");

    let temp_dir = TempDir::new()?;
    let agent = Agent::new();

    let messages = build_conversation_with_tool_pairs(3);
    let session = setup_test_session(&agent, &temp_dir, "desync-test", messages).await?;

    let mock_provider = Arc::new(MockSummarizationProvider::new());
    agent.update_provider(mock_provider, &session.id).await?;

    let session_config = SessionConfig {
        id: session.id.clone(),
        schedule_id: None,
        max_turns: Some(1),
        retry_config: None,
    };

    let new_user_message = Message::user()
        .with_text("continue")
        .with_id("msg_user_continue");

    // Run the agent so tool-pair summarization fires.
    let reply_stream = agent.reply(new_user_message, session_config, None).await?;
    tokio::pin!(reply_stream);
    while let Some(event_result) = reply_stream.next().await {
        match event_result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    }

    // --- Step 1: Read back server state and confirm hidden messages exist ---
    let server_session = agent
        .config
        .session_manager
        .get_session(&session.id, true)
        .await?;
    let server_conv = server_session.conversation.as_ref().unwrap();
    let server_msgs = server_conv.messages();

    let hidden_count_before = server_msgs
        .iter()
        .filter(|m: &&Message| !m.is_user_visible() && m.is_agent_visible())
        .count();
    assert!(
        hidden_count_before > 0,
        "Server should have at least one hidden summary after tool-pair summarization, found 0"
    );

    // --- Step 2: Simulate stale UI — keep only user-visible messages ---
    let stale_messages: Vec<Message> = server_msgs
        .iter()
        .filter(|m: &&Message| m.is_user_visible())
        .cloned()
        .collect();

    let stale_conv = Conversation::new_unvalidated(stale_messages);
    agent
        .config
        .session_manager
        .replace_conversation(&session.id, &stale_conv)
        .await?;

    // --- Step 3: Read back and verify hidden summaries were wiped ---
    let after_session = agent
        .config
        .session_manager
        .get_session(&session.id, true)
        .await?;
    let after_conv = after_session.conversation.as_ref().unwrap();
    let after_msgs = after_conv.messages();

    let hidden_count_after = after_msgs
        .iter()
        .filter(|m: &&Message| !m.is_user_visible() && m.is_agent_visible())
        .count();
    assert_eq!(
        hidden_count_after, 0,
        "After replacing with stale (user-visible only) conversation, \
         hidden summaries should be gone, but found {hidden_count_after}"
    );

    std::env::remove_var("GOOSE_TOOL_CALL_CUTOFF");
    Ok(())
}
