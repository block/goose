use async_trait::async_trait;
use goose::conversation::message::Message;
use goose::execution::manager::AgentManager;
use goose::model::ModelConfig;
use goose::providers::base::{Provider, ProviderUsage, Usage};
use goose::providers::errors::ProviderError;
use rmcp::model::Tool;
use serial_test::serial;
use std::sync::Arc;

// Mock provider for testing
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

    async fn complete_with_model(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
        Ok((
            Message::assistant().with_text("ok"),
            ProviderUsage::new("mock".to_string(), Usage::default()),
        ))
    }
}

#[tokio::test]
#[serial]
async fn test_subagents_cannot_spawn_subagents() -> anyhow::Result<()> {
    AgentManager::reset_for_test();

    let manager = AgentManager::instance().await?;

    let normal_agent = manager
        .get_or_create_agent("normal-session".to_string(), false)
        .await?;

    let subagent = manager
        .get_or_create_agent("subagent-session".to_string(), true)
        .await?;

    let model_config = ModelConfig::new("test-model").unwrap();
    let provider = Arc::new(MockProvider { model_config });
    normal_agent.update_provider(provider.clone()).await?;
    subagent.update_provider(provider).await?;

    let (normal_tools, _, _) = normal_agent.prepare_tools_and_prompt().await?;

    let (subagent_tools, _, _) = subagent.prepare_tools_and_prompt().await?;

    let normal_has_dynamic_task = normal_tools
        .iter()
        .any(|t| t.name == "dynamic_task__create_task");
    let normal_has_execute_task = normal_tools
        .iter()
        .any(|t| t.name == "subagent__execute_task");

    assert!(
        normal_has_dynamic_task,
        "Normal agent should have dynamic_task__create_task"
    );
    assert!(
        normal_has_execute_task,
        "Normal agent should have subagent__execute_task"
    );

    let subagent_has_dynamic_task = subagent_tools
        .iter()
        .any(|t| t.name == "dynamic_task__create_task");
    let subagent_has_execute_task = subagent_tools
        .iter()
        .any(|t| t.name == "subagent__execute_task");

    assert!(
        !subagent_has_dynamic_task,
        "Subagent should NOT have dynamic_task__create_task"
    );
    assert!(
        !subagent_has_execute_task,
        "Subagent should NOT have subagent__execute_task"
    );

    // Verify that subagents have fewer tools overall (missing the 2 subagent spawning tools)
    let tool_count_diff = normal_tools.len() as i32 - subagent_tools.len() as i32;
    assert_eq!(
        tool_count_diff, 2,
        "Subagent should have exactly 2 fewer tools than normal agent"
    );

    Ok(())
}
