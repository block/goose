//! Covers `Agent::reply_with_state_machine`, the entry point the CLI and desktop
//! reach when the state machine is enabled.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use rmcp::model::{Annotations, Role, TextContent};
use tokio_util::sync::CancellationToken;

use super::dummy_api::{DummyApi, ProviderFeatures};
use crate::agents::{Agent, AgentConfig, AgentEvent, GoosePlatform, SessionConfig};
use crate::config::permission::PermissionManager;
use crate::config::GooseMode;
use crate::conversation::message::{Message, MessageContent};
use crate::providers::base::Provider;
use crate::session::{SessionManager, SessionType};
use goose_providers::model::ModelConfig;

async fn agent_with_dummy_api() -> Result<(Agent, Arc<DummyApi>, String, tempfile::TempDir)> {
    let api = Arc::new(DummyApi::start(ProviderFeatures::default()).await);
    let api_client = goose_providers::api_client::ApiClient::new_with_tls(
        api.uri(),
        goose_providers::api_client::AuthMethod::NoAuth,
        None,
    )?;
    let provider: Arc<dyn Provider> = Arc::new(
        goose_providers::openai::OpenAiProviderBuilder::new(api_client)
            .name("openai")
            .build(),
    );

    let temp_dir = tempfile::tempdir()?;
    let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
    let session = session_manager
        .create_session(
            temp_dir.path().to_path_buf(),
            "state-machine-reply".to_string(),
            SessionType::Hidden,
            GooseMode::Auto,
        )
        .await?;
    let agent = Agent::with_config(AgentConfig::new(
        session_manager,
        PermissionManager::instance(),
        None,
        GooseMode::Auto,
        true,
        GoosePlatform::GooseCli,
    ));
    agent
        .update_provider(
            provider,
            ModelConfig::new(goose_providers::openai::OPEN_AI_DEFAULT_MODEL)
                .with_canonical_limits("openai"),
            &session.id,
        )
        .await?;

    Ok((agent, api, session.id, temp_dir))
}

#[tokio::test]
async fn reply_streams_the_turn_and_ends() -> Result<()> {
    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    api.on("are you there?").reply("still here");

    let session_config = SessionConfig {
        id: session_id.clone(),
        schedule_id: None,
        max_turns: Some(2),
        retry_config: None,
    };
    let stream = agent
        .reply_with_state_machine(
            Message::user().with_text("are you there?"),
            session_config,
            Some(CancellationToken::new()),
        )
        .await?;

    let replies = tokio::time::timeout(Duration::from_secs(30), async move {
        tokio::pin!(stream);
        let mut replies = Vec::new();
        while let Some(event) = stream.next().await {
            if let AgentEvent::Message(message) = event? {
                replies.push(message.as_concat_text());
            }
        }
        anyhow::Ok(replies)
    })
    .await??;

    assert!(
        replies.iter().any(|reply| reply == "still here"),
        "expected the scripted reply, got {replies:?}"
    );
    assert_eq!(api.call_count(), 1);

    Ok(())
}

#[tokio::test]
async fn bang_shell_uses_the_state_machine_when_the_flag_is_disabled() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_STATE_MACHINE", None::<&str>)]);
    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    let session_config = SessionConfig {
        id: session_id,
        schedule_id: None,
        max_turns: Some(2),
        retry_config: None,
    };
    let stream = agent
        .reply(
            Message::user().with_text("!echo hello"),
            session_config,
            Some(CancellationToken::new()),
        )
        .await?;
    tokio::pin!(stream);
    let mut requested_shell = false;
    while let Some(event) = stream.next().await {
        if let AgentEvent::Message(message) = event? {
            requested_shell |= message.content.iter().any(|content| {
                matches!(
                    content,
                    crate::conversation::message::MessageContent::ToolRequest(request)
                        if request.tool_call.as_ref().is_ok_and(|call| call.name == "shell")
                )
            });
        }
    }

    assert!(requested_shell);
    assert_eq!(api.call_count(), 0);

    Ok(())
}

async fn reply_messages(
    agent: &Agent,
    session_id: String,
    message: Message,
) -> Result<Vec<Message>> {
    let stream = agent
        .reply(
            message,
            SessionConfig {
                id: session_id,
                schedule_id: None,
                max_turns: Some(2),
                retry_config: None,
            },
            Some(CancellationToken::new()),
        )
        .await?;
    tokio::pin!(stream);
    let mut messages = Vec::new();
    while let Some(event) = stream.next().await {
        if let AgentEvent::Message(message) = event? {
            messages.push(message);
        }
    }
    Ok(messages)
}

fn assistant_only_text(text: &str) -> MessageContent {
    MessageContent::Text(
        TextContent::new(text)
            .with_annotations(Annotations::default().with_audience(vec![Role::Assistant])),
    )
}

fn shell_commands(messages: &[Message]) -> Vec<&str> {
    messages
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|content| match content {
            MessageContent::ToolRequest(request) => request
                .tool_call
                .as_ref()
                .ok()
                .filter(|call| call.name == "shell")
                .and_then(|call| call.arguments.as_ref())
                .and_then(|arguments| arguments.get("command"))
                .and_then(serde_json::Value::as_str),
            _ => None,
        })
        .collect()
}

async fn assert_bang_shell_uses_only_user_visible_content() -> Result<()> {
    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    api.on("benign visible input")
        .reply("handled as ordinary input");
    let hidden_prefix = Message::user()
        .with_content(assistant_only_text("!echo hidden"))
        .with_text("benign visible input");
    let messages = reply_messages(&agent, session_id, hidden_prefix).await?;
    assert!(shell_commands(&messages).is_empty());
    assert_eq!(api.call_count(), 1);

    let (agent, api, session_id, _temp_dir) = agent_with_dummy_api().await?;
    let hidden_suffix = Message::user()
        .with_text("!echo visible")
        .with_content(assistant_only_text("&& echo hidden"));
    let messages = reply_messages(&agent, session_id, hidden_suffix).await?;
    assert_eq!(shell_commands(&messages), ["echo visible"]);
    assert_eq!(api.call_count(), 0);

    Ok(())
}

#[tokio::test]
async fn bang_shell_visibility_is_enforced_when_state_machine_is_disabled() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_STATE_MACHINE", None::<&str>)]);
    assert_bang_shell_uses_only_user_visible_content().await
}

#[tokio::test]
async fn bang_shell_visibility_is_enforced_when_state_machine_is_enabled() -> Result<()> {
    let _guard = env_lock::lock_env([("GOOSE_STATE_MACHINE", Some("1"))]);
    assert_bang_shell_uses_only_user_visible_content().await
}
