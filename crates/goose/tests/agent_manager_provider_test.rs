use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use goose::agents::manager::{AgentManager, AgentManagerConfig};
use goose::conversation::message::Message;
use goose::conversation::Conversation;
use goose::model::ModelConfig;
use goose::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
use goose::providers::errors::ProviderError;
use goose::session;
use rmcp::model::Tool;

/// Mock provider that tracks which instance is being used
struct TrackingMockProvider {
    id: usize,
    model_config: ModelConfig,
    fail_on_complete: bool,
}

impl TrackingMockProvider {
    fn new(id: usize, fail: bool) -> Self {
        Self {
            id,
            model_config: ModelConfig::new_or_fail("mock-model"),
            fail_on_complete: fail,
        }
    }
}

#[async_trait::async_trait]
impl Provider for TrackingMockProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::empty()
    }

    async fn complete_with_model(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        if self.fail_on_complete {
            return Err(ProviderError::ServerError("Mock failure".to_string()));
        }

        Ok((
            Message::assistant().with_text(format!("Response from provider {}", self.id)),
            ProviderUsage::new("mock".to_string(), Usage::default()),
        ))
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model_config.clone()
    }
}

#[tokio::test]
async fn test_provider_isolation_between_sessions() {
    // Each session should be able to have its own provider configuration
    let manager = AgentManager::new(AgentManagerConfig::default()).await;

    let session1 = session::Identifier::Name("provider_test_1".to_string());
    let session2 = session::Identifier::Name("provider_test_2".to_string());

    // Get agents for both sessions
    let agent1 = manager.get_agent(session1.clone()).await.unwrap();
    let agent2 = manager.get_agent(session2.clone()).await.unwrap();

    // Set different providers for each agent
    let provider1 = Arc::new(TrackingMockProvider::new(1, false));
    let provider2 = Arc::new(TrackingMockProvider::new(2, false));

    agent1.update_provider(provider1).await.unwrap();
    agent2.update_provider(provider2).await.unwrap();

    // Create test conversations
    let conv = Conversation::new(vec![Message::user().with_text("test")]).unwrap();

    // Complete with each agent and verify they use different providers
    let (response1, _) = agent1
        .provider()
        .await
        .unwrap()
        .complete("system", conv.messages(), &[])
        .await
        .unwrap();

    let (response2, _) = agent2
        .provider()
        .await
        .unwrap()
        .complete("system", conv.messages(), &[])
        .await
        .unwrap();

    // Verify responses come from different providers
    assert!(response1.as_concat_text().contains("provider 1"));
    assert!(response2.as_concat_text().contains("provider 2"));
}

#[tokio::test]
async fn test_provider_initialization_failure_handling() {
    // Test that agent creation succeeds even if provider initialization fails
    // This is important because providers are initialized from environment variables
    // which might not be set correctly

    // Temporarily set invalid provider config
    std::env::set_var("GOOSE_PROVIDER", "invalid_provider");
    std::env::set_var("GOOSE_MODEL", "invalid_model");

    let manager = AgentManager::new(AgentManagerConfig::default()).await;
    let session = session::Identifier::Name("provider_fail_test".to_string());

    // This should succeed even though provider initialization will fail
    let agent = manager.get_agent(session.clone()).await;
    assert!(
        agent.is_ok(),
        "Agent creation should succeed even with invalid provider config"
    );

    // Clean up
    std::env::remove_var("GOOSE_PROVIDER");
    std::env::remove_var("GOOSE_MODEL");
}

#[tokio::test]
async fn test_concurrent_provider_updates() {
    // Test that concurrent provider updates to different sessions don't interfere
    let manager = Arc::new(AgentManager::new(AgentManagerConfig::default()).await);
    let update_counter = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];

    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let counter_clone = Arc::clone(&update_counter);

        let handle = tokio::spawn(async move {
            let session = session::Identifier::Name(format!("concurrent_provider_{}", i));
            let agent = manager_clone.get_agent(session).await.unwrap();

            // Each task updates its agent's provider multiple times
            for j in 0..5 {
                let provider = Arc::new(TrackingMockProvider::new(i * 100 + j, false));
                agent.update_provider(provider).await.unwrap();
                counter_clone.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            // Verify final provider is correct
            let conv = Conversation::new(vec![Message::user().with_text("test")]).unwrap();
            let (response, _) = agent
                .provider()
                .await
                .unwrap()
                .complete("system", conv.messages(), &[])
                .await
                .unwrap();

            // Should have the last provider for this session
            assert!(response
                .as_concat_text()
                .contains(&format!("provider {}", i * 100 + 4)));
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all updates completed
    assert_eq!(update_counter.load(Ordering::SeqCst), 50);
}

#[tokio::test]
async fn test_provider_persistence_across_agent_retrievals() {
    // Test that a provider set on an agent persists when the agent is retrieved again
    let manager = AgentManager::new(AgentManagerConfig::default()).await;
    let session = session::Identifier::Name("provider_persistence".to_string());

    // Get agent and set provider
    let agent1 = manager.get_agent(session.clone()).await.unwrap();
    let provider = Arc::new(TrackingMockProvider::new(42, false));
    agent1.update_provider(provider).await.unwrap();

    // Get the same agent again (should be cached)
    let agent2 = manager.get_agent(session.clone()).await.unwrap();

    // Verify it has the same provider
    let conv = Conversation::new(vec![Message::user().with_text("test")]).unwrap();
    let (response, _) = agent2
        .provider()
        .await
        .unwrap()
        .complete("system", conv.messages(), &[])
        .await
        .unwrap();

    assert!(response.as_concat_text().contains("provider 42"));

    // Verify they're the same agent instance
    assert!(Arc::ptr_eq(&agent1, &agent2));
}
