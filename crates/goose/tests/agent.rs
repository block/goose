// src/lib.rs or tests/truncate_agent_tests.rs

use std::sync::Arc;

use anyhow::Result;
use futures::StreamExt;
use goose::agents::{Agent, AgentEvent};
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::base::Provider;
use goose::providers::{
    anthropic::AnthropicProvider, azure::AzureProvider, bedrock::BedrockProvider,
    databricks::DatabricksProvider, gcpvertexai::GcpVertexAIProvider, google::GoogleProvider,
    groq::GroqProvider, ollama::OllamaProvider, openai::OpenAiProvider,
    openrouter::OpenRouterProvider, xai::XaiProvider,
};

#[derive(Debug, PartialEq)]
enum ProviderType {
    Azure,
    OpenAi,
    Anthropic,
    Bedrock,
    Databricks,
    GcpVertexAI,
    Google,
    Groq,
    Ollama,
    OpenRouter,
    Xai,
}

impl ProviderType {
    fn required_env(&self) -> &'static [&'static str] {
        match self {
            ProviderType::Azure => &[
                "AZURE_OPENAI_API_KEY",
                "AZURE_OPENAI_ENDPOINT",
                "AZURE_OPENAI_DEPLOYMENT_NAME",
            ],
            ProviderType::OpenAi => &["OPENAI_API_KEY"],
            ProviderType::Anthropic => &["ANTHROPIC_API_KEY"],
            ProviderType::Bedrock => &["AWS_PROFILE"],
            ProviderType::Databricks => &["DATABRICKS_HOST"],
            ProviderType::Google => &["GOOGLE_API_KEY"],
            ProviderType::Groq => &["GROQ_API_KEY"],
            ProviderType::Ollama => &[],
            ProviderType::OpenRouter => &["OPENROUTER_API_KEY"],
            ProviderType::GcpVertexAI => &["GCP_PROJECT_ID", "GCP_LOCATION"],
            ProviderType::Xai => &["XAI_API_KEY"],
        }
    }

    fn pre_check(&self) -> Result<()> {
        match self {
            ProviderType::Ollama => {
                // Check if the `ollama ls` CLI command works
                use std::process::Command;
                let output = Command::new("ollama").arg("ls").output();
                if let Ok(output) = output {
                    if output.status.success() {
                        return Ok(()); // CLI is running
                    }
                }
                println!("Skipping Ollama tests - `ollama ls` command not found or failed");
                Err(anyhow::anyhow!("Ollama CLI is not running"))
            }
            _ => Ok(()), // Other providers don't need special pre-checks
        }
    }

    fn create_provider(&self, model_config: ModelConfig) -> Result<Arc<dyn Provider>> {
        Ok(match self {
            ProviderType::Azure => Arc::new(AzureProvider::from_env(model_config)?),
            ProviderType::OpenAi => Arc::new(OpenAiProvider::from_env(model_config)?),
            ProviderType::Anthropic => Arc::new(AnthropicProvider::from_env(model_config)?),
            ProviderType::Bedrock => Arc::new(BedrockProvider::from_env(model_config)?),
            ProviderType::Databricks => Arc::new(DatabricksProvider::from_env(model_config)?),
            ProviderType::GcpVertexAI => Arc::new(GcpVertexAIProvider::from_env(model_config)?),
            ProviderType::Google => Arc::new(GoogleProvider::from_env(model_config)?),
            ProviderType::Groq => Arc::new(GroqProvider::from_env(model_config)?),
            ProviderType::Ollama => Arc::new(OllamaProvider::from_env(model_config)?),
            ProviderType::OpenRouter => Arc::new(OpenRouterProvider::from_env(model_config)?),
            ProviderType::Xai => Arc::new(XaiProvider::from_env(model_config)?),
        })
    }
}

pub fn check_required_env_vars(required_vars: &[&str]) -> Result<()> {
    let missing_vars: Vec<&str> = required_vars
        .iter()
        .filter(|&&var| std::env::var(var).is_err())
        .cloned()
        .collect();

    if !missing_vars.is_empty() {
        println!(
            "Skipping tests. Missing environment variables: {:?}",
            missing_vars
        );
        return Err(anyhow::anyhow!("Required environment variables not set"));
    }
    Ok(())
}

async fn run_truncate_test(
    provider_type: ProviderType,
    model: &str,
    context_window: usize,
) -> Result<()> {
    let model_config = ModelConfig::new(model.to_string())
        .with_context_limit(Some(context_window))
        .with_temperature(Some(0.0));
    let provider = provider_type.create_provider(model_config)?;

    let agent = Agent::new();
    agent.update_provider(provider).await?;
    let repeat_count = context_window + 10_000;
    let large_message_content = "hello ".repeat(repeat_count);
    let messages = vec![
        Message::user().with_text("hi there. what is 2 + 2?"),
        Message::assistant().with_text("hey! I think it's 4."),
        Message::user().with_text(&large_message_content),
        Message::assistant().with_text("heyy!!"),
        Message::user().with_text("what's the meaning of life?"),
        Message::assistant().with_text("the meaning of life is 42"),
        Message::user().with_text(
            "did I ask you what's 2+2 in this message history? just respond with 'yes' or 'no'",
        ),
    ];

    let reply_stream = agent.reply(&messages, None).await?;
    tokio::pin!(reply_stream);

    let mut responses = Vec::new();
    while let Some(response_result) = reply_stream.next().await {
        match response_result {
            Ok(AgentEvent::Message(response)) => responses.push(response),
            Ok(AgentEvent::McpNotification(n)) => {
                println!("MCP Notification: {n:?}");
            }
            Ok(AgentEvent::ModelChange { .. }) => {
                // Model change events are informational, just continue
            }

            Err(e) => {
                println!("Error: {:?}", e);
                return Err(e);
            }
        }
    }

    println!("Responses: {responses:?}\n");
    assert_eq!(responses.len(), 1);

    // Ollama and OpenRouter truncate by default even when the context window is exceeded
    // We don't have control over the truncation behavior in these providers
    if provider_type == ProviderType::Ollama || provider_type == ProviderType::OpenRouter {
        println!("WARNING: Skipping test for {:?} because it truncates by default when the context window is exceeded", provider_type);
        return Ok(());
    }

    assert_eq!(responses[0].content.len(), 1);

    match responses[0].content[0] {
        goose::message::MessageContent::Text(ref text_content) => {
            assert!(text_content.text.to_lowercase().contains("no"));
            assert!(!text_content.text.to_lowercase().contains("yes"));
        }
        goose::message::MessageContent::ContextLengthExceeded(_) => {
            // This is an acceptable outcome for providers that don't truncate themselves
            // and correctly report that the context length was exceeded.
            println!(
                "Received ContextLengthExceeded as expected for {:?}",
                provider_type
            );
        }
        _ => {
            panic!(
                "Unexpected message content type: {:?}",
                responses[0].content[0]
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestConfig {
        provider_type: ProviderType,
        model: &'static str,
        context_window: usize,
    }

    async fn run_test_with_config(config: TestConfig) -> Result<()> {
        println!("Starting test for {config:?}");

        // Check for required environment variables
        if check_required_env_vars(config.provider_type.required_env()).is_err() {
            return Ok(()); // Skip test if env vars are missing
        }

        // Run provider-specific pre-checks
        if config.provider_type.pre_check().is_err() {
            return Ok(()); // Skip test if pre-check fails
        }

        // Run the truncate test
        run_truncate_test(config.provider_type, config.model, config.context_window).await
    }

    #[tokio::test]
    async fn test_agent_with_openai() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::OpenAi,
            model: "o3-mini-low",
            context_window: 200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_azure() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Azure,
            model: "gpt-4o-mini",
            context_window: 128_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_anthropic() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Anthropic,
            model: "claude-3-5-haiku-latest",
            context_window: 200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_bedrock() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Bedrock,
            model: "anthropic.claude-3-5-sonnet-20241022-v2:0",
            context_window: 200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_databricks() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Databricks,
            model: "databricks-meta-llama-3-3-70b-instruct",
            context_window: 128_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_databricks_bedrock() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Databricks,
            model: "claude-3-5-sonnet-2",
            context_window: 200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_databricks_openai() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Databricks,
            model: "gpt-4o-mini",
            context_window: 128_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_google() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Google,
            model: "gemini-2.0-flash-exp",
            context_window: 1_200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_groq() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Groq,
            model: "gemma2-9b-it",
            context_window: 9_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_openrouter() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::OpenRouter,
            model: "deepseek/deepseek-r1",
            context_window: 130_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_ollama() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Ollama,
            model: "llama3.2",
            context_window: 128_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_gcpvertexai() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::GcpVertexAI,
            model: "claude-3-5-sonnet-v2@20241022",
            context_window: 200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_agent_with_xai() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Xai,
            model: "grok-3",
            context_window: 9_000,
        })
        .await
    }
}

#[cfg(test)]
mod schedule_tool_tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use goose::agents::platform_tools::PLATFORM_MANAGE_SCHEDULE_TOOL_NAME;
    use goose::scheduler::{ScheduledJob, SchedulerError};
    use goose::scheduler_trait::SchedulerTrait;
    use goose::session::storage::SessionMetadata;
    use std::sync::Arc;

    // Mock scheduler for testing
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
        async fn add_scheduled_job(&self, job: ScheduledJob) -> Result<(), SchedulerError> {
            let mut jobs = self.jobs.lock().await;
            jobs.push(job);
            Ok(())
        }

        async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
            let jobs = self.jobs.lock().await;
            Ok(jobs.clone())
        }

        async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError> {
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
        ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError> {
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
        let agent = Agent::new();
        let mock_scheduler = Arc::new(MockScheduler::new());
        agent.set_scheduler(mock_scheduler.clone()).await;

        // Test that the schedule management tool is available in the tools list
        let tools = agent.list_tools(None).await;
        let schedule_tool = tools
            .iter()
            .find(|tool| tool.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME);
        assert!(schedule_tool.is_some());

        let tool = schedule_tool.unwrap();
        assert!(tool
            .description
            .contains("Manage scheduled recipe execution"));
    }

    #[tokio::test]
    async fn test_schedule_management_tool_no_scheduler() {
        let agent = Agent::new();
        // Don't set scheduler - test that the tool still appears in the list
        // but would fail if actually called (which we can't test directly through public API)

        let tools = agent.list_tools(None).await;
        let schedule_tool = tools
            .iter()
            .find(|tool| tool.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME);
        assert!(schedule_tool.is_some());
    }

    #[tokio::test]
    async fn test_schedule_management_tool_in_platform_tools() {
        let agent = Agent::new();
        let tools = agent.list_tools(Some("platform".to_string())).await;

        // Check that the schedule management tool is included in platform tools
        let schedule_tool = tools
            .iter()
            .find(|tool| tool.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME);
        assert!(schedule_tool.is_some());

        let tool = schedule_tool.unwrap();
        assert!(tool
            .description
            .contains("Manage scheduled recipe execution"));

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
        let agent = Agent::new();
        let tools = agent.list_tools(None).await;
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
mod final_output_tool_tests {
    use super::*;
    use goose::agents::final_output_tool::FINAL_OUTPUT_TOOL_NAME;
    use goose::recipe::Response;

    #[tokio::test]
    async fn test_final_output_assistant_message_in_reply() -> Result<()> {
        use async_trait::async_trait;
        use goose::model::ModelConfig;
        use goose::providers::base::{Provider, ProviderUsage, Usage};
        use goose::providers::errors::ProviderError;
        use mcp_core::tool::Tool;

        #[derive(Clone)]
        struct MockProvider {
            model_config: ModelConfig,
        }

        #[async_trait]
        impl Provider for MockProvider {
            fn metadata() -> goose::providers::base::ProviderMetadata {
                goose::providers::base::ProviderMetadata::empty()
            }

            fn get_model_config(&self) -> ModelConfig {
                self.model_config.clone()
            }

            async fn complete(
                &self,
                _system: &str,
                _messages: &[Message],
                _tools: &[Tool],
            ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
                Ok((
                    Message::assistant().with_text("Task completed."),
                    ProviderUsage::new("mock".to_string(), Usage::default()),
                ))
            }
        }

        let agent = Agent::new();

        let model_config = ModelConfig::new("test-model".to_string());
        let mock_provider = Arc::new(MockProvider { model_config });
        agent.update_provider(mock_provider).await?;

        let response = Response {
            json_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "result": {"type": "string"}
                },
                "required": ["result"]
            })),
        };
        agent.add_final_output_tool(response).await;

        // Simulate a final output tool call occurring.
        let tool_call = mcp_core::tool::ToolCall::new(
            FINAL_OUTPUT_TOOL_NAME,
            serde_json::json!({
                "result": "Test output"
            }),
        );
        let (_, result) = agent
            .dispatch_tool_call(tool_call, "request_id".to_string())
            .await;

        assert!(result.is_ok(), "Tool call should succeed");
        let final_result = result.unwrap().result.await;
        assert!(final_result.is_ok(), "Tool execution should succeed");

        let content = final_result.unwrap();
        let text = content.first().unwrap().as_text().unwrap();
        assert!(
            text.contains("Final output successfully collected."),
            "Tool result missing expected content: {}",
            text
        );

        // Simulate the reply stream continuing after the final output tool call.
        let reply_stream = agent.reply(&vec![], None).await?;
        tokio::pin!(reply_stream);

        let mut responses = Vec::new();
        while let Some(response_result) = reply_stream.next().await {
            match response_result {
                Ok(AgentEvent::Message(response)) => responses.push(response),
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        assert!(!responses.is_empty(), "Should have received responses");
        let last_message = responses.last().unwrap();

        // Check that the last message is an assistant message with our final output
        assert_eq!(last_message.role, mcp_core::role::Role::Assistant);
        let message_text = last_message.as_concat_text();
        assert_eq!(message_text, r#"{"result":"Test output"}"#);

        Ok(())
    }
}

#[cfg(test)]
mod max_turns_tests {
    use super::*;
    use async_trait::async_trait;
    use goose::message::MessageContent;
    use goose::model::ModelConfig;
    use goose::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
    use goose::providers::errors::ProviderError;
    use goose::session::storage::Identifier;
    use mcp_core::tool::{Tool, ToolCall};
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
            _system_prompt: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            let tool_call = ToolCall::new("test_tool", serde_json::json!({"param": "value"}));
            let message = Message::assistant().with_tool_request("call_123", Ok(tool_call));

            let usage = ProviderUsage::new(
                "mock-model".to_string(),
                Usage::new(Some(10), Some(5), Some(15)),
            );

            Ok((message, usage))
        }

        fn get_model_config(&self) -> ModelConfig {
            ModelConfig::new("mock-model".to_string())
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
            }
        }
    }

    #[tokio::test]
    async fn test_max_turns_limit() -> Result<()> {
        let agent = Agent::new();
        let provider = Arc::new(MockToolProvider::new());
        agent.update_provider(provider).await?;
        // The mock provider will call a non-existent tool, which will fail and allow the loop to continue

        // Create session config with max_turns = 1
        let session_config = goose::agents::SessionConfig {
            id: Identifier::Name("test_session".to_string()),
            working_dir: PathBuf::from("/tmp"),
            schedule_id: None,
            execution_mode: None,
            max_turns: Some(1),
        };
        let messages = vec![Message::user().with_text("Hello")];

        let reply_stream = agent.reply(&messages, Some(session_config)).await?;
        tokio::pin!(reply_stream);

        let mut responses = Vec::new();
        while let Some(response_result) = reply_stream.next().await {
            match response_result {
                Ok(AgentEvent::Message(response)) => {
                    if let Some(MessageContent::ToolConfirmationRequest(ref req)) =
                        response.content.first()
                    {
                        agent.handle_confirmation(
                            req.id.clone(),
                            goose::permission::PermissionConfirmation {
                                principal_type: goose::permission::permission_confirmation::PrincipalType::Tool,
                                permission: goose::permission::Permission::AllowOnce,
                            }
                        ).await;
                    }
                    responses.push(response);
                }
                Ok(AgentEvent::McpNotification(_)) => {}
                Ok(AgentEvent::ModelChange { .. }) => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }

        assert!(
            responses.len() >= 1,
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
