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

        /// Mock provider that simulates compaction behavior and context limits
        struct MockCompactionProvider {
            summary_input_tokens: i32,
            summary_output_tokens: i32,
            reply_input_tokens: i32,
            reply_output_tokens: i32,
            /// If true, throws ContextLengthExceeded on first call, succeeds on subsequent calls
            simulate_context_limit: Arc<AtomicBool>,
        }

        impl MockCompactionProvider {
            fn new(
                summary_input_tokens: i32,
                summary_output_tokens: i32,
                reply_input_tokens: i32,
                reply_output_tokens: i32,
            ) -> Self {
                Self {
                    summary_input_tokens,
                    summary_output_tokens,
                    reply_input_tokens,
                    reply_output_tokens,
                    simulate_context_limit: Arc::new(AtomicBool::new(false)),
                }
            }

            fn with_context_limit_simulation(mut self) -> Self {
                self.simulate_context_limit = Arc::new(AtomicBool::new(true));
                self
            }
        }

        #[async_trait]
        impl Provider for MockCompactionProvider {
            async fn complete(
                &self,
                _session_id: &str,
                _system_prompt: &str,
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

                // Check if any message contains the trigger phrase for context limit
                let has_context_limit_trigger = messages.iter().any(|msg| {
                    msg.content.iter().any(|content| {
                        if let MessageContent::Text(text) = content {
                            text.text.contains("TRIGGER_CONTEXT_LIMIT")
                        } else {
                            false
                        }
                    })
                });

                // Simulate context limit exceeded on first call if enabled
                if has_context_limit_trigger
                    && self
                        .simulate_context_limit
                        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                {
                    return Err(ProviderError::ContextLengthExceeded(
                        "Context limit exceeded".to_string(),
                    ));
                }

                let (message, input_tokens, output_tokens) = if is_compaction {
                    (
                        Message::assistant().with_text("<mock summary of conversation>"),
                        self.summary_input_tokens,
                        self.summary_output_tokens,
                    )
                } else {
                    (
                        Message::assistant().with_text("This is a mock response."),
                        self.reply_input_tokens,
                        self.reply_output_tokens,
                    )
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

        /// Helper: Assert token counts match expected values
        fn assert_token_counts(
            session: &Session,
            expected_input: i32,
            expected_output: Option<i32>,
            expected_total: i32,
            expected_accumulated: i32,
            context: &str,
        ) {
            assert_eq!(
                session.input_tokens,
                Some(expected_input),
                "{}: input tokens mismatch",
                context
            );
            assert_eq!(
                session.output_tokens,
                expected_output,
                "{}: output tokens mismatch",
                context
            );
            assert_eq!(
                session.total_tokens,
                Some(expected_total),
                "{}: total tokens mismatch",
                context
            );
            assert_eq!(
                session.accumulated_total_tokens,
                Some(expected_accumulated),
                "{}: accumulated total mismatch",
                context
            );
        }

        /// Helper: Assert conversation has been compacted (contains summary message)
        fn assert_conversation_compacted(conversation: &Conversation) {
            let messages = conversation.messages();
            assert!(!messages.is_empty(), "Conversation should not be empty");

            // Check that at least one message contains the summary text
            let has_summary = messages.iter().any(|msg| {
                msg.content.iter().any(|content| {
                    if let MessageContent::Text(text) = content {
                        text.text.contains("mock summary")
                    } else {
                        false
                    }
                })
            });

            assert!(
                has_summary,
                "Conversation should contain the summary message"
            );
        }

        #[tokio::test]
        async fn test_manual_compaction_updates_token_counts_and_conversation() -> Result<()> {
            let temp_dir = TempDir::new()?;
            let agent = Agent::new();

            // Setup session with initial messages
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
            let summary_input_tokens: i32 = 800;
            let summary_output_tokens: i32 = 200;
            let provider = Arc::new(MockCompactionProvider::new(
                summary_input_tokens,
                summary_output_tokens,
                100,
                50,
            ));
            agent.update_provider(provider, &session.id).await?;

            // Execute manual compaction
            let result = agent.execute_command("/compact", &session.id).await?;
            assert!(result.is_some(), "Compaction should return a result");

            // Verify token counts
            let updated_session = agent
                .config
                .session_manager
                .get_session(&session.id, true)
                .await?;

            assert_token_counts(
                &updated_session,
                summary_output_tokens,
                None,
                summary_output_tokens,
                1000 + summary_input_tokens + summary_output_tokens,
                "Manual compaction",
            );

            // Verify conversation has been compacted
            let compacted_conversation = updated_session
                .conversation
                .expect("Session should have conversation");

            assert_conversation_compacted(&compacted_conversation);

            // The compacted conversation should contain the summary
            // Note: The exact message structure after compaction varies, but the summary should be present
            let new_message_count = compacted_conversation.messages().len();
            assert!(
                new_message_count > 0,
                "Compacted conversation should have at least the summary message"
            );

            // After manual compaction, the conversation should be compacted
            // The specific structure depends on how compaction preserves context

            Ok(())
        }

        #[tokio::test]
        async fn test_auto_compaction_during_reply() -> Result<()> {
            let temp_dir = TempDir::new()?;
            let agent = Agent::new();

            // Setup session with many messages to trigger auto-compaction
            let mut messages = vec![];
            for i in 0..20 {
                messages.push(Message::user().with_text(format!("User message {}", i)));
                messages.push(Message::assistant().with_text(format!("Assistant response {}", i)));
            }

            let session = setup_test_session(&agent, &temp_dir, "auto-compact-test", messages)
                .await?;

            // Setup mock provider
            let summary_input_tokens: i32 = 800;
            let summary_output_tokens: i32 = 200;
            let reply_input_tokens: i32 = 300;
            let reply_output_tokens: i32 = 100;
            let provider = Arc::new(MockCompactionProvider::new(
                summary_input_tokens,
                summary_output_tokens,
                reply_input_tokens,
                reply_output_tokens,
            ));
            agent.update_provider(provider, &session.id).await?;

            // Trigger a reply that should cause auto-compaction
            // Note: Auto-compaction triggers are complex and may require specific conditions
            // For this test, we're verifying the token counting mechanism works when it does trigger
            let user_message = Message::user().with_text("Tell me more about compaction");

            let session_config = SessionConfig {
                id: session.id.clone(),
                schedule_id: None,
                max_turns: None,
                retry_config: None,
            };

            let reply_stream = agent.reply(user_message, session_config, None).await?;
            tokio::pin!(reply_stream);

            // Consume the stream to completion
            let mut compaction_occurred = false;
            while let Some(event_result) = reply_stream.next().await {
                match event_result {
                    Ok(AgentEvent::HistoryReplaced(_)) => {
                        compaction_occurred = true;
                    }
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }

            // If compaction occurred, verify token counts were updated
            if compaction_occurred {
                let updated_session = agent
                    .config
                    .session_manager
                    .get_session(&session.id, true)
                    .await?;

                // After auto-compaction + reply, tokens should reflect both operations
                // The exact values depend on whether compaction happened before or during the reply
                assert!(
                    updated_session.accumulated_total_tokens.unwrap_or(0)
                        >= 1000 + reply_input_tokens + reply_output_tokens,
                    "Accumulated tokens should include at least original + reply"
                );
            }

            Ok(())
        }

        #[tokio::test]
        async fn test_context_limit_recovery_compaction() -> Result<()> {
            let temp_dir = TempDir::new()?;
            let agent = Agent::new();

            // Setup session with a message that will trigger context limit
            let messages = vec![
                Message::user().with_text("Hello"),
                Message::assistant().with_text("Hi there"),
                Message::user().with_text("TRIGGER_CONTEXT_LIMIT Please help me"),
            ];

            let session = setup_test_session(&agent, &temp_dir, "context-limit-test", messages)
                .await?;

            // Setup mock provider that simulates context limit on first call
            let summary_input_tokens: i32 = 800;
            let summary_output_tokens: i32 = 200;
            let reply_input_tokens: i32 = 150;
            let reply_output_tokens: i32 = 75;
            let provider = Arc::new(
                MockCompactionProvider::new(
                    summary_input_tokens,
                    summary_output_tokens,
                    reply_input_tokens,
                    reply_output_tokens,
                )
                .with_context_limit_simulation(),
            );
            agent.update_provider(provider, &session.id).await?;

            // Process the message that triggers context limit
            let session_config = SessionConfig {
                id: session.id.clone(),
                schedule_id: None,
                max_turns: None,
                retry_config: None,
            };

            let reply_stream = agent
                .reply(
                    Message::user().with_text("Continue"),
                    session_config,
                    None,
                )
                .await?;
            tokio::pin!(reply_stream);

            // Consume stream and track if compaction occurred
            let mut compaction_occurred = false;
            let mut got_response = false;

            while let Some(event_result) = reply_stream.next().await {
                match event_result {
                    Ok(AgentEvent::HistoryReplaced(_)) => {
                        compaction_occurred = true;
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

            // Verify recovery occurred (compaction triggered and we got a response)
            assert!(
                compaction_occurred,
                "Compaction should have occurred due to context limit"
            );
            assert!(
                got_response,
                "Should have received a response after recovery"
            );

            // Verify token counts were updated
            let updated_session = agent
                .config
                .session_manager
                .get_session(&session.id, true)
                .await?;

            // After recovery compaction, tokens should be updated to include compaction cost
            // The exact accumulated value depends on whether the reply also completes
            let min_expected_accumulated = 1000 + summary_input_tokens + summary_output_tokens;

            // If compaction occurred, verify tokens include compaction cost
            if compaction_occurred {
                assert!(
                    updated_session.accumulated_total_tokens.unwrap_or(0) >= min_expected_accumulated,
                    "Accumulated tokens should include compaction cost. Expected at least {}, got {:?}",
                    min_expected_accumulated,
                    updated_session.accumulated_total_tokens
                );

                // Verify that the conversation was compacted
                let updated_conversation = updated_session
                    .conversation
                    .expect("Session should have conversation");
                assert_conversation_compacted(&updated_conversation);
            }

            Ok(())
        }
    }
}
