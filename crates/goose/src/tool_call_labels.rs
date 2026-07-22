use crate::agents::Agent;
use crate::conversation::message::{Message, MessageContent, ToolRequest, TOOL_META_TITLE_KEY};
use crate::model_config::get_fast_model;
use crate::providers::base::Provider;
use crate::session::SessionManager;
use crate::session_context::with_session_id;
use crate::utils::safe_truncate;
use goose_providers::model::ModelConfig;
use serde_json::json;
use std::slice::from_ref;
use std::time::Duration;
use tokio::time::sleep;
use tracing::warn;

const TOOL_TITLE_SYSTEM_PROMPT: &str =
    "Summarize this tool call in a short lowercase phrase (3-8 words). \
     No punctuation. No quotes. Examples: reading project configuration, \
     checking network connectivity, listing files in src directory";
const TOOL_TITLE_ARGUMENTS_MAX_LENGTH: usize = 300;

pub(crate) async fn generate_tool_title(
    agent: &Agent,
    session_manager: &SessionManager,
    session_id: &str,
    message_id: Option<&str>,
    tool_request: &ToolRequest,
) -> Option<String> {
    let provider = agent.provider().await.ok()?;
    if provider.manages_own_context() {
        return None;
    }

    let model_config = agent.model_config_for_session(session_id).await.ok()?;
    let fast_model_config = get_fast_model(provider.get_name(), &model_config)
        .await
        .ok()?;
    let title = generate_tool_title_with_provider(
        provider.as_ref(),
        &fast_model_config,
        session_id,
        tool_request,
    )
    .await?;
    let request_id = &tool_request.id;

    if let Some(message_id) = message_id {
        let patch = json!({
            (TOOL_META_TITLE_KEY): &title,
        });
        if let Err(error) = session_manager
            .update_tool_request_meta(session_id, message_id, request_id, patch)
            .await
        {
            warn!("tool call title: persist failed for {request_id} in {message_id}: {error}",);
        }
    }

    Some(title)
}

async fn generate_tool_title_with_provider(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
    tool_request: &ToolRequest,
) -> Option<String> {
    let tool_call = tool_request.tool_call.as_ref().ok()?;
    let name = &tool_call.name;
    let args_json = tool_call
        .arguments
        .as_ref()
        .map(|arguments| {
            let serialized = serde_json::to_string(arguments).unwrap_or_default();
            if serialized.len() > TOOL_TITLE_ARGUMENTS_MAX_LENGTH {
                format!(
                    "{}…",
                    safe_truncate(&serialized, TOOL_TITLE_ARGUMENTS_MAX_LENGTH)
                )
            } else {
                serialized
            }
        })
        .unwrap_or_default();
    let message = Message::user().with_text(format!("Tool: {name}\nArguments: {args_json}"));

    const MAX_ATTEMPTS: usize = 2;
    for attempt in 0..MAX_ATTEMPTS {
        if let Ok((response, _)) = with_session_id(
            Some(session_id.to_string()),
            provider.complete(
                model_config,
                TOOL_TITLE_SYSTEM_PROMPT,
                from_ref(&message),
                &[],
            ),
        )
        .await
        {
            let title = response
                .content
                .iter()
                .filter_map(MessageContent::as_text)
                .collect::<String>()
                .trim()
                .to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }

        if attempt + 1 < MAX_ATTEMPTS {
            sleep(Duration::from_millis(150)).await;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{AgentConfig, GoosePlatform};
    use crate::config::{GooseMode, PermissionManager};
    use crate::providers::base::{MessageStream, ProviderUsage, Usage};
    use crate::session::{SessionManager, SessionType};
    use async_trait::async_trait;
    use goose_providers::errors::ProviderError;
    use rmcp::model::{CallToolRequestParams, Tool};
    use serde_json::json;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    struct MockProvider {
        outcomes: Mutex<VecDeque<Result<Message, ProviderError>>>,
        calls: AtomicUsize,
        messages: Mutex<Vec<Vec<Message>>>,
        manages_own_context: bool,
    }

    impl MockProvider {
        fn new(outcomes: Vec<Result<Message, ProviderError>>) -> Self {
            Self {
                outcomes: Mutex::new(outcomes.into()),
                calls: AtomicUsize::new(0),
                messages: Mutex::new(Vec::new()),
                manages_own_context: false,
            }
        }

        fn managing_own_context() -> Self {
            Self {
                outcomes: Mutex::new(VecDeque::new()),
                calls: AtomicUsize::new(0),
                messages: Mutex::new(Vec::new()),
                manages_own_context: true,
            }
        }

        fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }

        fn first_user_message(&self) -> String {
            self.messages.lock().unwrap()[0][0].as_concat_text()
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn get_name(&self) -> &str {
            "tool-title-test"
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<MessageStream, ProviderError> {
            unreachable!("title generation calls complete directly")
        }

        async fn complete(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.messages.lock().unwrap().push(messages.to_vec());
            let outcome = self
                .outcomes
                .lock()
                .unwrap()
                .pop_front()
                .expect("test provider should have a configured outcome");
            outcome.map(|message| {
                (
                    message,
                    ProviderUsage::new("tool-title-test".to_string(), Usage::default()),
                )
            })
        }

        fn manages_own_context(&self) -> bool {
            self.manages_own_context
        }
    }

    fn tool_request(arguments: serde_json::Value) -> ToolRequest {
        ToolRequest {
            id: "request-1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("developer__shell")
                .with_arguments(arguments.as_object().unwrap().clone())),
            metadata: None,
            tool_meta: None,
        }
    }

    async fn generate_with(provider: &MockProvider, tool_request: &ToolRequest) -> Option<String> {
        generate_tool_title_with_provider(
            provider,
            &ModelConfig::new("test-model"),
            "session-1",
            tool_request,
        )
        .await
    }

    #[tokio::test]
    async fn returns_trimmed_generated_title() {
        let provider = MockProvider::new(vec![Ok(
            Message::assistant().with_text("  checking project status  ")
        )]);

        let title = generate_with(&provider, &tool_request(json!({"command": "git status"}))).await;

        assert_eq!(title.as_deref(), Some("checking project status"));
        assert_eq!(provider.call_count(), 1);
    }

    #[tokio::test]
    async fn retries_once_after_empty_response() {
        let provider = MockProvider::new(vec![
            Ok(Message::assistant().with_text("  ")),
            Ok(Message::assistant().with_text("reading project configuration")),
        ]);

        let title = generate_with(&provider, &tool_request(json!({}))).await;

        assert_eq!(title.as_deref(), Some("reading project configuration"));
        assert_eq!(provider.call_count(), 2);
    }

    #[tokio::test]
    async fn retries_once_after_provider_error() {
        let provider = MockProvider::new(vec![
            Err(ProviderError::ExecutionError("temporary".to_string())),
            Ok(Message::assistant().with_text("checking network connectivity")),
        ]);

        let title = generate_with(&provider, &tool_request(json!({}))).await;

        assert_eq!(title.as_deref(), Some("checking network connectivity"));
        assert_eq!(provider.call_count(), 2);
    }

    #[tokio::test]
    async fn returns_none_after_two_unsuccessful_attempts() {
        let provider = MockProvider::new(vec![
            Ok(Message::assistant().with_text("")),
            Err(ProviderError::ExecutionError(
                "still unavailable".to_string(),
            )),
        ]);

        let title = generate_with(&provider, &tool_request(json!({}))).await;

        assert_eq!(title, None);
        assert_eq!(provider.call_count(), 2);
    }

    #[tokio::test]
    async fn skips_provider_that_manages_own_context() {
        let temp_dir = TempDir::new().unwrap();
        let session_manager = Arc::new(SessionManager::new(temp_dir.path().join("sessions")));
        let permission_manager =
            Arc::new(PermissionManager::new(temp_dir.path().join("permissions")));
        let agent = Agent::with_config(AgentConfig::new(
            session_manager.clone(),
            permission_manager,
            None,
            GooseMode::Auto,
            true,
            GoosePlatform::GooseCli,
        ));
        let session = session_manager
            .create_session(
                PathBuf::new(),
                "test".to_string(),
                SessionType::Hidden,
                GooseMode::Auto,
            )
            .await
            .unwrap();
        let provider = Arc::new(MockProvider::managing_own_context());
        agent
            .update_provider(
                provider.clone(),
                ModelConfig::new("test-model"),
                &session.id,
            )
            .await
            .unwrap();

        let title = generate_tool_title(
            &agent,
            session_manager.as_ref(),
            &session.id,
            None,
            &tool_request(json!({})),
        )
        .await;

        assert_eq!(title, None);
        assert_eq!(provider.call_count(), 0);
    }

    #[tokio::test]
    async fn preserves_short_serialized_arguments() {
        let provider = MockProvider::new(vec![Ok(Message::assistant().with_text("title"))]);
        let tool_request = tool_request(json!({"command": "git status"}));

        generate_with(&provider, &tool_request).await;

        assert_eq!(
            provider.first_user_message(),
            "Tool: developer__shell\nArguments: {\"command\":\"git status\"}",
        );
    }

    #[tokio::test]
    async fn truncates_long_serialized_arguments() {
        let provider = MockProvider::new(vec![Ok(Message::assistant().with_text("title"))]);
        let arguments = json!({"command": "x".repeat(400)});
        let serialized = serde_json::to_string(arguments.as_object().unwrap()).unwrap();
        let expected = format!(
            "Tool: developer__shell\nArguments: {}…",
            safe_truncate(&serialized, TOOL_TITLE_ARGUMENTS_MAX_LENGTH),
        );
        let tool_request = tool_request(arguments);

        generate_with(&provider, &tool_request).await;

        assert_eq!(provider.first_user_message(), expected);
    }

    #[tokio::test]
    async fn persists_title_for_known_message_id() {
        let temp_dir = TempDir::new().unwrap();
        let session_manager = Arc::new(SessionManager::new(temp_dir.path().join("sessions")));
        let permission_manager =
            Arc::new(PermissionManager::new(temp_dir.path().join("permissions")));
        let agent = Agent::with_config(AgentConfig::new(
            session_manager.clone(),
            permission_manager,
            None,
            GooseMode::Auto,
            true,
            GoosePlatform::GooseCli,
        ));
        let session = session_manager
            .create_session(
                PathBuf::new(),
                "test".to_string(),
                SessionType::Hidden,
                GooseMode::Auto,
            )
            .await
            .unwrap();
        let provider = Arc::new(MockProvider::new(vec![Ok(
            Message::assistant().with_text("checking project status")
        )]));
        agent
            .update_provider(provider, ModelConfig::new("test-model"), &session.id)
            .await
            .unwrap();
        let tool_request = tool_request(json!({"command": "git status"}));
        let message = Message::assistant()
            .with_id("message-1")
            .with_tool_request(tool_request.id.clone(), tool_request.tool_call.clone());
        session_manager
            .add_message(&session.id, &message)
            .await
            .unwrap();

        let title = generate_tool_title(
            &agent,
            session_manager.as_ref(),
            &session.id,
            message.id.as_deref(),
            &tool_request,
        )
        .await;

        assert_eq!(title.as_deref(), Some("checking project status"));
        let loaded = session_manager
            .get_session(&session.id, true)
            .await
            .unwrap();
        let persisted_title = loaded
            .conversation
            .as_ref()
            .unwrap()
            .messages()
            .iter()
            .flat_map(|message| &message.content)
            .find_map(|content| match content {
                MessageContent::ToolRequest(request) if request.id == "request-1" => {
                    request.generated_title()
                }
                _ => None,
            });

        assert_eq!(persisted_title, Some("checking project status"));
    }
}
