use std::sync::Arc;

use anyhow::Result;
use futures::StreamExt;
use goose::agents::{Agent, AgentEvent};
use goose::config::extensions::{set_extension, ExtensionEntry};

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod schedule_tool_tests {
        use super::*;
        use async_trait::async_trait;
        use chrono::{DateTime, Utc};
        use goose::agents::platform_tools::PLATFORM_MANAGE_SCHEDULE_TOOL_NAME;
        use goose::agents::AgentConfig;
        use goose::config::permission::PermissionManager;
        use goose::config::GooseMode;
        use goose::scheduler::{ScheduledJob, SchedulerError};
        use goose::scheduler_trait::SchedulerTrait;
        use goose::session::{Session, SessionManager};
        use std::path::PathBuf;
        use std::sync::Arc;
        use tempfile::TempDir;

        struct MockScheduler {
            jobs: tokio::sync::Mutex<Vec<ScheduledJob>>,
        }

        impl MockScheduler {
            fn new() -> Self {
                Self {
                    jobs: tokio::sync::Mutex::new(Vec::new()),
                }
            }
        }

        #[async_trait]
        impl SchedulerTrait for MockScheduler {
            async fn add_scheduled_job(
                &self,
                job: ScheduledJob,
                _copy: bool,
            ) -> Result<(), SchedulerError> {
                let mut jobs = self.jobs.lock().await;
                jobs.push(job);
                Ok(())
            }

            async fn schedule_recipe(
                &self,
                _recipe_path: PathBuf,
                _cron_schedule: Option<String>,
            ) -> Result<(), SchedulerError> {
                Ok(())
            }

            async fn list_scheduled_jobs(&self) -> Vec<ScheduledJob> {
                let jobs = self.jobs.lock().await;
                jobs.clone()
            }

            async fn remove_scheduled_job(
                &self,
                id: &str,
                _remove: bool,
            ) -> Result<(), SchedulerError> {
                let mut jobs = self.jobs.lock().await;
                if let Some(pos) = jobs.iter().position(|job| job.id == id) {
                    jobs.remove(pos);
                    Ok(())
                } else {
                    Err(SchedulerError::JobNotFound(id.to_string()))
                }
            }

            async fn pause_schedule(&self, _id: &str) -> Result<(), SchedulerError> {
                Ok(())
            }

            async fn unpause_schedule(&self, _id: &str) -> Result<(), SchedulerError> {
                Ok(())
            }

            async fn run_now(&self, _id: &str) -> Result<String, SchedulerError> {
                Ok("test_session_123".to_string())
            }

            async fn sessions(
                &self,
                _sched_id: &str,
                _limit: usize,
            ) -> Result<Vec<(String, Session)>, SchedulerError> {
                Ok(vec![])
            }

            async fn update_schedule(
                &self,
                _sched_id: &str,
                _new_cron: String,
            ) -> Result<(), SchedulerError> {
                Ok(())
            }

            async fn kill_running_job(&self, _sched_id: &str) -> Result<(), SchedulerError> {
                Ok(())
            }

            async fn get_running_job_info(
                &self,
                _sched_id: &str,
            ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
                Ok(None)
            }
        }

        #[tokio::test]
        async fn test_schedule_management_tool_list() {
            let temp_dir = TempDir::new().unwrap();
            let data_dir = temp_dir.path().to_path_buf();
            let session_manager = Arc::new(SessionManager::new(data_dir.clone()));
            let permission_manager = Arc::new(PermissionManager::new(data_dir));
            let mock_scheduler = Arc::new(MockScheduler::new());
            let config = AgentConfig::new(
                session_manager,
                permission_manager,
                Some(mock_scheduler),
                GooseMode::Auto,
            );
            let agent = Agent::with_config(config);

            let tools = agent.list_tools("test-session-id", None).await;
            let schedule_tool = tools
                .iter()
                .find(|tool| tool.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME);
            assert!(schedule_tool.is_some());

            let tool = schedule_tool.unwrap();
            assert!(tool
                .description
                .clone()
                .unwrap_or_default()
                .contains("Manage goose's internal scheduled recipe execution"));
        }

        #[tokio::test]
        async fn test_no_schedule_management_tool_without_scheduler() {
            let agent = Agent::new();

            let tools = agent.list_tools("test-session-id", None).await;
            let schedule_tool = tools
                .iter()
                .find(|tool| tool.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME);
            assert!(schedule_tool.is_none());
        }

        #[tokio::test]
        async fn test_schedule_management_tool_in_platform_tools() {
            let temp_dir = TempDir::new().unwrap();
            let data_dir = temp_dir.path().to_path_buf();
            let session_manager = Arc::new(SessionManager::new(data_dir.clone()));
            let permission_manager = Arc::new(PermissionManager::new(data_dir));
            let mock_scheduler = Arc::new(MockScheduler::new());
            let config = AgentConfig::new(
                session_manager,
                permission_manager,
                Some(mock_scheduler),
                GooseMode::Auto,
            );
            let agent = Agent::with_config(config);

            let tools = agent
                .list_tools("test-session-id", Some("platform".to_string()))
                .await;

            // Check that the schedule management tool is included in platform tools
            let schedule_tool = tools
                .iter()
                .find(|tool| tool.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME);
            assert!(schedule_tool.is_some());

            let tool = schedule_tool.unwrap();
            assert!(tool
                .description
                .clone()
                .unwrap_or_default()
                .contains("Manage goose's internal scheduled recipe execution"));

            // Verify the tool has the expected actions in its schema
            if let Some(properties) = tool.input_schema.get("properties") {
                if let Some(action_prop) = properties.get("action") {
                    if let Some(enum_values) = action_prop.get("enum") {
                        let actions: Vec<String> = enum_values
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_str().unwrap().to_string())
                            .collect();

                        // Check that our session_content action is included
                        assert!(actions.contains(&"session_content".to_string()));
                        assert!(actions.contains(&"list".to_string()));
                        assert!(actions.contains(&"create".to_string()));
                        assert!(actions.contains(&"sessions".to_string()));
                    }
                }
            }
        }

        #[tokio::test]
        async fn test_schedule_management_tool_schema_validation() {
            let temp_dir = TempDir::new().unwrap();
            let data_dir = temp_dir.path().to_path_buf();
            let session_manager = Arc::new(SessionManager::new(data_dir.clone()));
            let permission_manager = Arc::new(PermissionManager::new(data_dir));
            let mock_scheduler = Arc::new(MockScheduler::new());
            let config = AgentConfig::new(
                session_manager,
                permission_manager,
                Some(mock_scheduler),
                GooseMode::Auto,
            );
            let agent = Agent::with_config(config);

            let tools = agent.list_tools("test-session-id", None).await;
            let schedule_tool = tools
                .iter()
                .find(|tool| tool.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME);
            assert!(schedule_tool.is_some());

            let tool = schedule_tool.unwrap();

            // Verify the tool schema has the session_id parameter for session_content action
            if let Some(properties) = tool.input_schema.get("properties") {
                assert!(properties.get("session_id").is_some());

                if let Some(session_id_prop) = properties.get("session_id") {
                    assert_eq!(
                        session_id_prop.get("type").unwrap().as_str().unwrap(),
                        "string"
                    );
                    assert!(session_id_prop
                        .get("description")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .contains("Session identifier for session_content action"));
                }
            }
        }
    }

    #[cfg(test)]
    mod retry_tests {
        use super::*;
        use goose::agents::types::{RetryConfig, SuccessCheck};

        #[tokio::test]
        async fn test_retry_success_check_execution() -> Result<()> {
            use goose::agents::retry::execute_success_checks;

            let retry_config = RetryConfig {
                max_retries: 3,
                checks: vec![],
                on_failure: None,
                timeout_seconds: Some(30),
                on_failure_timeout_seconds: Some(60),
            };

            let success_checks = vec![SuccessCheck::Shell {
                command: "echo 'test'".to_string(),
            }];

            let result = execute_success_checks(&success_checks, &retry_config).await;
            assert!(result.is_ok(), "Success check should pass");
            assert!(result.unwrap(), "Command should succeed");

            let fail_checks = vec![SuccessCheck::Shell {
                command: "false".to_string(),
            }];

            let result = execute_success_checks(&fail_checks, &retry_config).await;
            assert!(result.is_ok(), "Success check execution should not error");
            assert!(!result.unwrap(), "Command should fail");

            Ok(())
        }

        #[tokio::test]
        async fn test_retry_logic_with_validation_errors() -> Result<()> {
            let invalid_retry_config = RetryConfig {
                max_retries: 0,
                checks: vec![],
                on_failure: None,
                timeout_seconds: Some(0),
                on_failure_timeout_seconds: None,
            };

            let validation_result = invalid_retry_config.validate();
            assert!(
                validation_result.is_err(),
                "Should validate max_retries > 0"
            );
            assert!(validation_result
                .unwrap_err()
                .contains("max_retries must be greater than 0"));

            Ok(())
        }

        #[tokio::test]
        async fn test_retry_attempts_counter_reset() -> Result<()> {
            let agent = Agent::new();

            agent.reset_retry_attempts().await;
            let initial_attempts = agent.get_retry_attempts().await;
            assert_eq!(initial_attempts, 0);

            let new_attempts = agent.increment_retry_attempts().await;
            assert_eq!(new_attempts, 1);

            agent.reset_retry_attempts().await;
            let reset_attempts = agent.get_retry_attempts().await;
            assert_eq!(reset_attempts, 0);

            Ok(())
        }
    }

    #[cfg(test)]
    mod max_turns_tests {
        use super::*;
        use async_trait::async_trait;
        use goose::agents::SessionConfig;
        use goose::conversation::message::{Message, MessageContent};
        use goose::model::ModelConfig;
        use goose::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
        use goose::providers::errors::ProviderError;
        use goose::session::session_manager::SessionType;
        use rmcp::model::{CallToolRequestParam, Tool};
        use rmcp::object;
        use std::path::PathBuf;

        struct MockToolProvider {}

        impl MockToolProvider {
            fn new() -> Self {
                Self {}
            }
        }

        #[async_trait]
        impl Provider for MockToolProvider {
            async fn complete(
                &self,
                _session_id: &str,
                _system_prompt: &str,
                _messages: &[Message],
                _tools: &[Tool],
            ) -> Result<(Message, ProviderUsage), ProviderError> {
                let tool_call = CallToolRequestParam {
                    task: None,
                    name: "test_tool".into(),
                    arguments: Some(object!({"param": "value"})),
                };
                let message = Message::assistant().with_tool_request("call_123", Ok(tool_call));

                let usage = ProviderUsage::new(
                    "mock-model".to_string(),
                    Usage::new(Some(10), Some(5), Some(15)),
                );

                Ok((message, usage))
            }

            async fn complete_with_model(
                &self,
                session_id: &str,
                _model_config: &ModelConfig,
                system_prompt: &str,
                messages: &[Message],
                tools: &[Tool],
            ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
                self.complete(session_id, system_prompt, messages, tools)
                    .await
            }

            fn get_model_config(&self) -> ModelConfig {
                ModelConfig::new("mock-model").unwrap()
            }

            fn metadata() -> ProviderMetadata {
                ProviderMetadata {
                    name: "mock".to_string(),
                    display_name: "Mock Provider".to_string(),
                    description: "Mock provider for testing".to_string(),
                    default_model: "mock-model".to_string(),
                    known_models: vec![],
                    model_doc_link: "".to_string(),
                    config_keys: vec![],
                    allows_unlisted_models: false,
                }
            }

            fn get_name(&self) -> &str {
                "mock-test"
            }
        }

        #[tokio::test]
        async fn test_max_turns_limit() -> Result<()> {
            let agent = Agent::new();
            let provider = Arc::new(MockToolProvider::new());
            let user_message = Message::user().with_text("Hello");

            let session = agent
                .config
                .session_manager
                .create_session(
                    PathBuf::default(),
                    "max-turn-test".to_string(),
                    SessionType::Hidden,
                )
                .await?;

            agent.update_provider(provider, &session.id).await?;

            let session_config = SessionConfig {
                id: session.id,
                schedule_id: None,
                max_turns: Some(1),
                retry_config: None,
            };

            let reply_stream = agent.reply(user_message, session_config, None).await?;
            tokio::pin!(reply_stream);

            let mut responses = Vec::new();
            while let Some(response_result) = reply_stream.next().await {
                match response_result {
                    Ok(AgentEvent::Message(response)) => {
                        if let Some(MessageContent::ActionRequired(action)) =
                            response.content.first()
                        {
                            if let goose::conversation::message::ActionRequiredData::ToolConfirmation { id, .. } = &action.data {
                                agent.handle_confirmation(
                                    id.clone(),
                                    goose::permission::PermissionConfirmation {
                                        principal_type: goose::permission::permission_confirmation::PrincipalType::Tool,
                                        permission: goose::permission::Permission::AllowOnce,
                                    }
                                ).await;
                            }
                        }
                        responses.push(response);
                    }
                    Ok(AgentEvent::McpNotification(_)) => {}
                    Ok(AgentEvent::ModelChange { .. }) => {}
                    Ok(AgentEvent::HistoryReplaced(_updated_conversation)) => {
                        // We should update the conversation here, but we're not reading it
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }

            assert!(
                !responses.is_empty(),
                "Expected at least 1 response, got {}",
                responses.len()
            );

            // Look for the max turns message as the last response
            let last_response = responses.last().unwrap();
            let last_content = last_response.content.first().unwrap();
            if let MessageContent::Text(text_content) = last_content {
                assert!(text_content.text.contains(
                    "I've reached the maximum number of actions I can do without user input"
                ));
            } else {
                panic!("Expected text content in last message");
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod extension_manager_tests {
        use super::*;
        use goose::agents::extension::ExtensionConfig;
        use goose::agents::extension_manager_extension::{
            MANAGE_EXTENSIONS_TOOL_NAME, SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
        };
        use goose::agents::AgentConfig;
        use goose::config::permission::PermissionManager;
        use goose::config::GooseMode;
        use goose::session::SessionManager;

        async fn setup_agent_with_extension_manager() -> (Agent, String) {
            // Add the TODO extension to the config so it can be discovered by search_available_extensions
            // Set it as disabled initially so tests can enable it
            let todo_extension_entry = ExtensionEntry {
                enabled: false,
                config: ExtensionConfig::Platform {
                    name: "todo".to_string(),
                    description:
                        "Enable a todo list for goose so it can keep track of what it is doing"
                            .to_string(),
                    bundled: Some(true),
                    available_tools: vec![],
                },
            };
            set_extension(todo_extension_entry);

            // Create agent with session_id from the start
            let temp_dir = tempfile::tempdir().unwrap();
            let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
            let session_id = "test-session-id".to_string();
            let config = AgentConfig::new(
                session_manager,
                PermissionManager::instance(),
                None,
                GooseMode::Auto,
            );

            let agent = Agent::with_config(config);

            // Now add the extension manager platform extension
            let ext_config = ExtensionConfig::Platform {
                name: "extensionmanager".to_string(),
                description: "Extension Manager".to_string(),
                bundled: Some(true),
                available_tools: vec![],
            };

            agent
                .add_extension(ext_config)
                .await
                .expect("Failed to add extension manager");
            (agent, session_id)
        }

        #[tokio::test]
        async fn test_extension_manager_tools_available() {
            let (agent, session_id) = setup_agent_with_extension_manager().await;
            let tools = agent.list_tools(&session_id, None).await;

            // Note: Tool names are prefixed with the normalized extension name "extensionmanager"
            // not the display name "Extension Manager"
            let search_tool = tools.iter().find(|tool| {
                tool.name == format!("extensionmanager__{SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME}")
            });
            assert!(
                search_tool.is_some(),
                "search_available_extensions tool should be available"
            );

            let manage_tool = tools.iter().find(|tool| {
                tool.name == format!("extensionmanager__{MANAGE_EXTENSIONS_TOOL_NAME}")
            });
            assert!(
                manage_tool.is_some(),
                "manage_extensions tool should be available"
            );
        }
    }

    #[cfg(test)]
    mod compaction_tests {
        use super::*;
        use async_trait::async_trait;
        use futures::StreamExt;
        use goose::agents::SessionConfig;
        use goose::conversation::message::{Message, MessageContent};
        use goose::conversation::Conversation;
        use goose::model::ModelConfig;
        use goose::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
        use goose::providers::errors::ProviderError;
        use goose::session::session_manager::SessionType;
        use goose::session::Session;
        use rmcp::model::Tool;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use tempfile::TempDir;

        /// Mock provider that simulates realistic token counting and context limits
        struct MockCompactionProvider {
            /// Tracks whether compaction has occurred (for context limit recovery)
            has_compacted: Arc<AtomicBool>,
        }

        impl MockCompactionProvider {
            /// Create a new mock provider
            fn new() -> Self {
                Self {
                    has_compacted: Arc::new(AtomicBool::new(false)),
                }
            }

            /// Calculate input tokens based on system prompt and messages
            /// Simulates realistic token counts for different scenarios
            fn calculate_input_tokens(&self, system_prompt: &str, messages: &[Message]) -> i32 {
                // Check if this is a compaction call
                let is_compaction_call = messages.len() == 1
                    && messages[0].content.iter().any(|c| {
                        if let MessageContent::Text(text) = c {
                            text.text.to_lowercase().contains("summarize")
                        } else {
                            false
                        }
                    });

                if is_compaction_call {
                    // For compaction: system prompt length is a good proxy for conversation size
                    // Base: 6000 (system) + conversation content embedded in prompt
                    6000 + (system_prompt.len() as i32 / 4).max(400)
                } else {
                    // Regular call: system prompt + messages
                    let system_tokens = if system_prompt.is_empty() { 0 } else { 6000 };

                    let message_tokens: i32 = messages
                        .iter()
                        .map(|msg| {
                            let mut tokens = 100;
                            for content in &msg.content {
                                if let MessageContent::Text(text) = content {
                                    if text.text.contains("long_tool_call") {
                                        tokens += 15000;
                                    }
                                }
                            }
                            tokens
                        })
                        .sum();

                    system_tokens + message_tokens
                }
            }

            /// Calculate output tokens based on response type
            fn calculate_output_tokens(&self, is_compaction: bool, messages: &[Message]) -> i32 {
                if is_compaction {
                    // Compaction produces a compact summary
                    200
                } else {
                    // Regular responses vary by content
                    let has_hello = messages.iter().any(|msg| {
                        msg.content.iter().any(|c| {
                            if let MessageContent::Text(text) = c {
                                text.text.to_lowercase().contains("hello")
                            } else {
                                false
                            }
                        })
                    });

                    if has_hello {
                        50 // Simple greeting response
                    } else {
                        100 // Default response
                    }
                }
            }
        }

        #[async_trait]
        impl Provider for MockCompactionProvider {
            async fn complete(
                &self,
                _session_id: &str,
                system_prompt: &str,
                messages: &[Message],
                _tools: &[Tool],
            ) -> Result<(Message, ProviderUsage), ProviderError> {
                // Check if this is a compaction call (message contains "summarize")
                let is_compaction = messages.iter().any(|msg| {
                    msg.content.iter().any(|content| {
                        if let MessageContent::Text(text) = content {
                            text.text.to_lowercase().contains("summarize")
                        } else {
                            false
                        }
                    })
                });

                // Calculate realistic token counts based on actual content
                let input_tokens = self.calculate_input_tokens(system_prompt, messages);
                let output_tokens = self.calculate_output_tokens(is_compaction, messages);

                // Simulate context limit: if input > 20k tokens and we haven't compacted yet, fail
                const CONTEXT_LIMIT: i32 = 20000;
                if !is_compaction && input_tokens > CONTEXT_LIMIT {
                    if !self.has_compacted.load(Ordering::SeqCst) {
                        return Err(ProviderError::ContextLengthExceeded(format!(
                            "Context limit exceeded: {} > {}",
                            input_tokens, CONTEXT_LIMIT
                        )));
                    }
                }

                // If this is a compaction call, mark that we've compacted
                if is_compaction {
                    self.has_compacted.store(true, Ordering::SeqCst);
                }

                // Generate response
                let message = if is_compaction {
                    Message::assistant().with_text("<mock summary of conversation>")
                } else {
                    let response_text = if messages.iter().any(|msg| {
                        msg.content.iter().any(|c| {
                            if let MessageContent::Text(text) = c {
                                text.text.to_lowercase().contains("hello")
                            } else {
                                false
                            }
                        })
                    }) {
                        "Hi there! How can I help you?"
                    } else {
                        "This is a mock response."
                    };
                    Message::assistant().with_text(response_text)
                };

                let usage = ProviderUsage::new(
                    "mock-model".to_string(),
                    Usage::new(
                        Some(input_tokens),
                        Some(output_tokens),
                        Some(input_tokens + output_tokens),
                    ),
                );

                Ok((message, usage))
            }

            async fn complete_with_model(
                &self,
                session_id: &str,
                _model_config: &ModelConfig,
                system_prompt: &str,
                messages: &[Message],
                tools: &[Tool],
            ) -> Result<(Message, ProviderUsage), ProviderError> {
                self.complete(session_id, system_prompt, messages, tools)
                    .await
            }

            async fn complete_fast(
                &self,
                session_id: &str,
                system_prompt: &str,
                messages: &[Message],
                tools: &[Tool],
            ) -> Result<(Message, ProviderUsage), ProviderError> {
                // Compaction uses complete_fast, so delegate to complete
                self.complete(session_id, system_prompt, messages, tools)
                    .await
            }

            fn get_model_config(&self) -> ModelConfig {
                ModelConfig::new("mock-model").unwrap()
            }

            fn metadata() -> ProviderMetadata {
                ProviderMetadata {
                    name: "mock".to_string(),
                    display_name: "Mock Compaction Provider".to_string(),
                    description: "Mock provider for compaction testing".to_string(),
                    default_model: "mock-model".to_string(),
                    known_models: vec![],
                    model_doc_link: "".to_string(),
                    config_keys: vec![],
                    allows_unlisted_models: false,
                }
            }

            fn get_name(&self) -> &str {
                "mock-compaction"
            }
        }

        /// Helper: Setup a test session with initial messages and token counts
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

            // Set initial token counts
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

        /// Helper: Assert conversation has been compacted with proper message visibility
        fn assert_conversation_compacted(conversation: &Conversation) {
            let messages = conversation.messages();
            assert!(!messages.is_empty(), "Conversation should not be empty");

            // Find the summary message (contains "mock summary")
            let summary_index = messages
                .iter()
                .position(|msg| {
                    msg.content.iter().any(|content| {
                        if let MessageContent::Text(text) = content {
                            text.text.contains("mock summary")
                        } else {
                            false
                        }
                    })
                })
                .expect("Conversation should contain the summary message");

            let summary_msg = &messages[summary_index];

            // Assert summary message visibility:
            // - Agent visible: true (agent needs to see the summary)
            // - User visible: false (user doesn't see internal summary)
            assert!(
                summary_msg.is_agent_visible(),
                "Summary message should be agent visible"
            );
            assert!(
                !summary_msg.is_user_visible(),
                "Summary message should NOT be user visible"
            );

            // Check messages BEFORE the summary (the compacted original messages)
            // These should be made agent-invisible
            for (idx, msg) in messages.iter().enumerate() {
                if idx < summary_index {
                    // Old messages before summary: agent can't see them
                    assert!(
                        !msg.is_agent_visible(),
                        "Message before summary at index {} should be agent-invisible",
                        idx
                    );
                }
            }

            // Check for continuation message after summary
            // (Should exist and be agent-only)
            if summary_index + 1 < messages.len() {
                let continuation_msg = &messages[summary_index + 1];
                // Continuation message should contain instructions about not mentioning summary
                let has_continuation_text = continuation_msg.content.iter().any(|content| {
                    if let MessageContent::Text(text) = content {
                        text.text.contains("previous message contains a summary")
                            || text.text.contains("summarization occurred")
                    } else {
                        false
                    }
                });

                if has_continuation_text {
                    assert!(
                        continuation_msg.is_agent_visible(),
                        "Continuation message should be agent visible"
                    );
                    assert!(
                        !continuation_msg.is_user_visible(),
                        "Continuation message should NOT be user visible"
                    );
                }
            }

            // Any messages AFTER the continuation (e.g., preserved recent user message)
            // should be fully visible to both agent and user
            let continuation_end = summary_index + 2;
            for (idx, msg) in messages.iter().enumerate() {
                if idx >= continuation_end {
                    assert!(
                        msg.is_agent_visible() && msg.is_user_visible(),
                        "Message after compaction at index {} should be fully visible",
                        idx
                    );
                }
            }
        }

        #[tokio::test]
        async fn test_manual_compaction_updates_token_counts_and_conversation() -> Result<()> {
            let temp_dir = TempDir::new()?;
            let agent = Agent::new();

            // Setup session with initial messages
            // Each message ~100 tokens, so 4 messages = ~400 tokens in conversation
            let messages = vec![
                Message::user().with_text("Hello, can you help me with something?"),
                Message::assistant().with_text("Of course! What do you need help with?"),
                Message::user().with_text("I need to understand how compaction works."),
                Message::assistant()
                    .with_text("Compaction is a process that summarizes conversation history."),
            ];

            let session = setup_test_session(&agent, &temp_dir, "manual-compact-test", messages)
                .await?;

            // Setup mock provider
            let provider = Arc::new(MockCompactionProvider::new());
            agent.update_provider(provider, &session.id).await?;

            // Execute manual compaction
            // Execute manual compaction
            let result = agent.execute_command("/compact", &session.id).await?;
            assert!(result.is_some(), "Compaction should return a result");

            // Verify token counts
            let updated_session = agent
                .config
                .session_manager
                .get_session(&session.id, true)
                .await?;

            // Expected token calculation for compaction:
            // During compaction, the 4 messages are embedded in the system prompt template
            // - Input: system prompt with embedded conversation + "Please summarize" message
            // - Output: summary (200 tokens)
            //
            // From mock provider calculation:
            // - System prompt (with 4 embedded messages): varies based on template + content
            // - Single "summarize" message: 100 tokens
            // - Total input observed: ~6100 tokens
            //
            // After compaction:
            // - current input_tokens = summary output (200) - the new compact context
            // - current output_tokens = None (compaction doesn't produce new output)
            // - current total_tokens = 200
            // - accumulated_total = initial (1000) + compaction cost
            let expected_summary_output = 200; // compact summary

            // Verify the key invariants:
            // 1. Current context is the summary output
            assert_eq!(
                updated_session.input_tokens,
                Some(expected_summary_output),
                "Input tokens should be summary output (new context)"
            );
            assert_eq!(
                updated_session.output_tokens,
                None,
                "Output tokens should be None after compaction"
            );
            assert_eq!(
                updated_session.total_tokens,
                Some(expected_summary_output),
                "Total should equal input after compaction"
            );

            // 2. Accumulated tokens increased by the compaction cost
            let accumulated = updated_session.accumulated_total_tokens.unwrap();
            assert!(
                accumulated > 1000,
                "Accumulated should include compaction cost. Got: {}",
                accumulated
            );
            // The compaction cost varies based on template rendering, but should be substantial
            assert!(
                accumulated >= 1000 + 6000,
                "Accumulated should include at least initial (1000) + compaction cost (>6000). Got: {}",
                accumulated
            );

            // Verify conversation has been compacted
            let compacted_conversation = updated_session
                .conversation
                .expect("Session should have conversation");

            assert_conversation_compacted(&compacted_conversation);

            Ok(())
        }

        #[tokio::test]
        async fn test_auto_compaction_during_reply() -> Result<()> {
            let temp_dir = TempDir::new()?;
            let agent = Agent::new();

            // Setup session with many messages to have substantial context
            // 20 exchanges = 40 messages * 100 tokens = ~4000 tokens in conversation
            let mut messages = vec![];
            for i in 0..20 {
                messages.push(Message::user().with_text(format!("User message {}", i)));
                messages.push(Message::assistant().with_text(format!("Assistant response {}", i)));
            }

            let session = setup_test_session(&agent, &temp_dir, "auto-compact-test", messages)
                .await?;

            // Capture initial context size before triggering reply
            // Should be: system (6000) + 40 messages (4000) = ~10000 tokens
            let initial_session = agent
                .config
                .session_manager
                .get_session(&session.id, true)
                .await?;
            let initial_input_tokens = initial_session.input_tokens.unwrap_or(0);

            // Setup mock provider (no context limit enforcement)
            let provider = Arc::new(MockCompactionProvider::new());
            agent.update_provider(provider, &session.id).await?;

            // Trigger a reply
            // Expected tokens for reply:
            // - Input: system (6000) + 40 messages (4000) + new user message (100) = 10100 tokens
            // - Output: regular response (100 tokens)
            let user_message = Message::user().with_text("Tell me more about compaction");

            let session_config = SessionConfig {
                id: session.id.clone(),
                schedule_id: None,
                max_turns: None,
                retry_config: None,
            };

            let reply_stream = agent.reply(user_message, session_config, None).await?;
            tokio::pin!(reply_stream);

            // Track compaction and context size changes
            let mut compaction_occurred = false;
            let mut input_tokens_after_compaction: Option<i32> = None;

            while let Some(event_result) = reply_stream.next().await {
                match event_result {
                    Ok(AgentEvent::HistoryReplaced(_)) => {
                        compaction_occurred = true;

                        // Capture the input tokens immediately after compaction
                        let session_after_compact = agent
                            .config
                            .session_manager
                            .get_session(&session.id, true)
                            .await?;
                        input_tokens_after_compaction = session_after_compact.input_tokens;
                    }
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }

            let updated_session = agent
                .config
                .session_manager
                .get_session(&session.id, true)
                .await?;

            if compaction_occurred {
                // Verify that current input context decreased after compaction
                let tokens_after = input_tokens_after_compaction.expect("Should have captured tokens after compaction");

                // After compaction, the input context should be much smaller
                // Before: system (6000) + 40 messages (4000) = 10000
                // After: system (6000) + summary (200) = 6200
                assert!(
                    tokens_after < initial_input_tokens,
                    "Input tokens should decrease after compaction. Before: {}, After: {}",
                    initial_input_tokens,
                    tokens_after
                );

                // Specifically, should be roughly: system (6000) + summary (200) = 6200
                assert!(
                    tokens_after < 7000,
                    "Input tokens after compaction should be ~6200 (system + summary). Got: {}",
                    tokens_after
                );

                // After auto-compaction + reply, accumulated should include:
                // - Initial: 1000
                // - Compaction input: system (6000) + messages (~4000) = ~10000
                // - Compaction output: 200
                // - Reply after compaction: varies
                let min_accumulated = 1000 + 10000 + 200;
                assert!(
                    updated_session.accumulated_total_tokens.unwrap_or(0) >= min_accumulated,
                    "Accumulated tokens should include compaction cost. Expected >= {}, got {:?}",
                    min_accumulated,
                    updated_session.accumulated_total_tokens
                );
            } else {
                // If no compaction, accumulated should include reply cost
                // - Initial: 1000
                // - Reply: ~10100 input + 100 output = 10200
                let min_accumulated = 1000 + 10100 + 100;
                assert!(
                    updated_session.accumulated_total_tokens.unwrap_or(0) >= min_accumulated,
                    "Accumulated tokens should include reply cost. Expected >= {}, got {:?}",
                    min_accumulated,
                    updated_session.accumulated_total_tokens
                );
            }

            Ok(())
        }

        #[tokio::test]
        async fn test_context_limit_recovery_compaction() -> Result<()> {
            let temp_dir = TempDir::new()?;
            let agent = Agent::new();

            // Setup session with messages that will push context over the limit
            // Each message = 100 tokens, but we'll add a large one
            let messages = vec![
                Message::user().with_text("Hello"),
                Message::assistant().with_text("Hi there"),
                Message::user().with_text("Can you process this long_tool_call result?"),
                Message::assistant().with_text("Processing..."),
            ];
            // Token calculation:
            // - 3 regular messages: 300 tokens
            // - 1 message with "long_tool_call": 100 + 15000 = 15100 tokens
            // - Total conversation: ~15400 tokens
            // - With system prompt (6000): 21400 tokens

            let session = setup_test_session(&agent, &temp_dir, "context-limit-test", messages)
                .await?;

            // Note: The initial session input_tokens is set to 600 by setup_test_session,
            // but the actual context during the provider call will be calculated dynamically:
            // system (6000) + messages with long_tool_call (~15400) = ~21400 tokens
            // This will exceed the 20000 token limit when the call is made.

            // Setup mock provider with context limit of 20000 tokens
            // Initial context (6000 system + 15400 messages = 21400) exceeds this limit
            let provider = Arc::new(MockCompactionProvider::new());
            agent.update_provider(provider, &session.id).await?;

            // Try to send a message - should trigger context limit, then recover via compaction
            let session_config = SessionConfig {
                id: session.id.clone(),
                schedule_id: None,
                max_turns: None,
                retry_config: None,
            };

            let reply_stream = agent
                .reply(
                    Message::user().with_text("Tell me more"),
                    session_config,
                    None,
                )
                .await?;
            tokio::pin!(reply_stream);

            // Track compaction and context size changes
            let mut compaction_occurred = false;
            let mut got_response = false;
            let mut input_tokens_after_compaction: Option<i32> = None;

            while let Some(event_result) = reply_stream.next().await {
                match event_result {
                    Ok(AgentEvent::HistoryReplaced(_)) => {
                        compaction_occurred = true;

                        // Capture the input tokens immediately after compaction
                        let session_after_compact = agent
                            .config
                            .session_manager
                            .get_session(&session.id, true)
                            .await?;
                        input_tokens_after_compaction = session_after_compact.input_tokens;
                    }
                    Ok(AgentEvent::Message(msg)) => {
                        // Check if we got a real response (not just a notification)
                        if msg
                            .content
                            .iter()
                            .any(|c| matches!(c, MessageContent::Text(_)))
                        {
                            got_response = true;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }

            // Verify recovery occurred
            assert!(
                compaction_occurred,
                "Compaction should have occurred due to context limit (>20000 tokens)"
            );
            assert!(
                got_response,
                "Should have received a response after recovery"
            );

            // Verify token counts
            let updated_session = agent
                .config
                .session_manager
                .get_session(&session.id, true)
                .await?;

            // Expected token flow:
            // 1. Initial attempt: >20000 tokens -> Context limit exceeded
            // 2. Compaction triggered:
            //    - Input: system prompt + messages (including long_tool_call with 15k tokens)
            //    - Output: 200 tokens (summary)
            //    - New context size: 200 tokens
            // 3. Retry with compacted context:
            //    - Input: system prompt + summary (200) + new message
            //    - Output: 100 tokens (response)

            // Verify that current input context is small after compaction
            let tokens_after = input_tokens_after_compaction
                .expect("Should have captured tokens after compaction");

            // After compaction, the input context should be:
            // system (6000) + summary (200) = 6200
            // This is much smaller than the original >21k that triggered the limit

            // The compacted context should now be under the 20k limit
            assert!(
                tokens_after < 20000,
                "Input tokens after compaction should be under 20k limit. Got: {}",
                tokens_after
            );

            // Specifically, should be roughly: system (6000) + summary (200) = 6200
            assert!(
                tokens_after < 10000,
                "Input tokens after compaction should be ~6200 (system + summary). Got: {}",
                tokens_after
            );

            // Verify context limit was exceeded and recovered
            // Accumulated should include: initial (1000) + compaction cost + reply cost
            // The actual values vary based on template rendering, but should be substantial
            let accumulated = updated_session.accumulated_total_tokens.unwrap();
            assert!(
                accumulated > 20000,
                "Accumulated should include compaction and reply costs (exceeded context limit). Got: {}",
                accumulated
            );

            // Verify that the conversation was compacted
            let updated_conversation = updated_session
                .conversation
                .expect("Session should have conversation");
            assert_conversation_compacted(&updated_conversation);

            Ok(())
        }
    }
}
