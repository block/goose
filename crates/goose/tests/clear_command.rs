//! `/clear` and `/compact` against a provider that owns the conversation.
//!
//! For such providers goose's transcript is a display copy: the model sees the
//! provider's own history. Clearing only the copy leaves the real context in
//! place while the token meter reads zero. See issue #10763.

use async_trait::async_trait;
use goose::agents::execute_commands::{
    compaction_unsupported_message, stale_provider_context_message,
};
use goose::agents::Agent;
use goose::config::GooseMode;
use goose::conversation::message::{Message, MessageContent};
use goose::conversation::Conversation;
use goose::providers::base::{stream_from_single_message, MessageStream, Provider};
use goose::session::session_manager::SessionType;
use goose::session::Session;
use goose_providers::conversation::token_usage::{ProviderUsage, Usage};
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use rmcp::model::Tool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

const PROVIDER_NAME: &str = "mock-owns-context";

struct MockOwnContextProvider {
    resets: AtomicUsize,
    reset_error: Option<String>,
}

impl MockOwnContextProvider {
    fn resettable() -> Self {
        Self {
            resets: AtomicUsize::new(0),
            reset_error: None,
        }
    }

    fn unresettable(error: &str) -> Self {
        Self {
            resets: AtomicUsize::new(0),
            reset_error: Some(error.to_string()),
        }
    }
}

/// A context-owning provider that never overrides `reset_context`, standing in
/// for one added later by someone unaware of `/clear`.
struct ForgetfulProvider;

#[async_trait]
impl Provider for ForgetfulProvider {
    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system_prompt: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let usage = ProviderUsage::new(
            "mock-model".to_string(),
            Usage::new(Some(10), Some(10), Some(20)),
        );
        Ok(stream_from_single_message(
            Message::assistant().with_text("ok"),
            usage,
        ))
    }

    fn get_name(&self) -> &str {
        "forgetful"
    }

    fn manages_own_context(&self) -> bool {
        true
    }
}

#[async_trait]
impl Provider for MockOwnContextProvider {
    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system_prompt: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let usage = ProviderUsage::new(
            "mock-model".to_string(),
            Usage::new(Some(10), Some(10), Some(20)),
        );
        Ok(stream_from_single_message(
            Message::assistant().with_text("ok"),
            usage,
        ))
    }

    fn get_name(&self) -> &str {
        PROVIDER_NAME
    }

    fn manages_own_context(&self) -> bool {
        true
    }

    async fn reset_context(&self) -> Result<(), ProviderError> {
        self.resets.fetch_add(1, Ordering::SeqCst);
        match &self.reset_error {
            Some(error) => Err(ProviderError::NotImplemented(error.clone())),
            None => Ok(()),
        }
    }
}

async fn setup_session(agent: &Agent, temp_dir: &TempDir, name: &str) -> anyhow::Result<Session> {
    let session = agent
        .config
        .session_manager
        .create_session(
            temp_dir.path().to_path_buf(),
            name.to_string(),
            SessionType::Hidden,
            GooseMode::default(),
        )
        .await?;

    let conversation = Conversation::new_unvalidated(vec![
        Message::user().with_text("Hello"),
        Message::assistant().with_text("Hi there!"),
    ]);
    agent
        .config
        .session_manager
        .replace_conversation(&session.id, &conversation)
        .await?;
    agent
        .config
        .session_manager
        .update(&session.id)
        .usage(Usage::new(Some(6000), Some(400), Some(6400)))
        .apply()
        .await?;

    Ok(session)
}

fn message_text(message: &Message) -> String {
    message
        .content
        .iter()
        .filter_map(|content| match content {
            MessageContent::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn clear_resets_provider_context_and_zeroes_usage() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let agent = Agent::new();
    let session = setup_session(&agent, &temp_dir, "clear-resettable").await?;

    let provider = Arc::new(MockOwnContextProvider::resettable());
    agent
        .update_provider(
            provider.clone(),
            ModelConfig::new("mock-model"),
            &session.id,
        )
        .await?;

    let result = agent
        .execute_command("/clear", &session.id)
        .await?
        .expect("/clear returns a message");
    assert_eq!(message_text(&result), "Conversation cleared");
    assert_eq!(provider.resets.load(Ordering::SeqCst), 1);

    let updated = agent
        .config
        .session_manager
        .get_session(&session.id, true)
        .await?;
    assert!(updated
        .conversation
        .expect("session has a conversation")
        .messages()
        .is_empty());
    assert_eq!(updated.usage.total_tokens, Some(0));

    Ok(())
}

#[tokio::test]
async fn clear_keeps_usage_when_the_provider_cannot_reset() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let agent = Agent::new();
    let session = setup_session(&agent, &temp_dir, "clear-unresettable").await?;

    let provider = Arc::new(MockOwnContextProvider::unresettable("no reset for you"));
    agent
        .update_provider(
            provider.clone(),
            ModelConfig::new("mock-model"),
            &session.id,
        )
        .await?;

    let result = agent
        .execute_command("/clear", &session.id)
        .await?
        .expect("/clear returns a message");
    assert_eq!(
        message_text(&result),
        stale_provider_context_message(PROVIDER_NAME, "Unsupported operation: no reset for you")
    );

    let updated = agent
        .config
        .session_manager
        .get_session(&session.id, true)
        .await?;
    assert!(updated
        .conversation
        .expect("session has a conversation")
        .messages()
        .is_empty());
    assert_eq!(
        updated.usage.total_tokens,
        Some(6400),
        "the meter must keep reflecting the context the model still holds"
    );

    Ok(())
}

#[tokio::test]
async fn a_context_owning_provider_does_not_inherit_a_silent_reset() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let agent = Agent::new();
    let session = setup_session(&agent, &temp_dir, "clear-forgetful").await?;

    agent
        .update_provider(
            Arc::new(ForgetfulProvider),
            ModelConfig::new("mock-model"),
            &session.id,
        )
        .await?;

    let result = agent
        .execute_command("/clear", &session.id)
        .await?
        .expect("/clear returns a message");
    assert!(
        message_text(&result).contains("still holds the previous conversation"),
        "the default reset_context must refuse for providers that own their context, got: {}",
        message_text(&result)
    );

    let updated = agent
        .config
        .session_manager
        .get_session(&session.id, true)
        .await?;
    assert_eq!(updated.usage.total_tokens, Some(6400));

    Ok(())
}

/// A resumed session whose provider restore failed leaves the agent with no
/// provider at all. There is still a local transcript to clear, and no
/// provider-side context to go stale.
#[tokio::test]
async fn clear_falls_back_to_a_local_clear_without_a_provider() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let agent = Agent::new();
    let session = setup_session(&agent, &temp_dir, "clear-no-provider").await?;

    let result = agent
        .execute_command("/clear", &session.id)
        .await?
        .expect("/clear returns a message");
    assert_eq!(message_text(&result), "Conversation cleared");

    let updated = agent
        .config
        .session_manager
        .get_session(&session.id, true)
        .await?;
    assert!(updated
        .conversation
        .expect("session has a conversation")
        .messages()
        .is_empty());
    assert_eq!(updated.usage.total_tokens, Some(0));

    Ok(())
}

#[tokio::test]
async fn compact_is_rejected_for_providers_that_own_their_context() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let agent = Agent::new();
    let session = setup_session(&agent, &temp_dir, "compact-own-context").await?;

    let provider = Arc::new(MockOwnContextProvider::resettable());
    agent
        .update_provider(provider, ModelConfig::new("mock-model"), &session.id)
        .await?;

    let result = agent
        .execute_command("/compact", &session.id)
        .await?
        .expect("/compact returns a message");
    assert_eq!(
        message_text(&result),
        compaction_unsupported_message(PROVIDER_NAME)
    );

    let updated = agent
        .config
        .session_manager
        .get_session(&session.id, true)
        .await?;
    assert_eq!(
        updated
            .conversation
            .expect("session has a conversation")
            .messages()
            .len(),
        2,
        "a rejected /compact must leave the transcript alone"
    );
    assert_eq!(updated.usage.total_tokens, Some(6400));

    Ok(())
}
