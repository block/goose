use goose::agents::manager::AgentManager;
use goose::session;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_agent_per_session() {
    // Verify each session gets unique agent
    let manager = AgentManager::new(Default::default()).await;
    let session1 = session::Identifier::Name("session1".to_string());
    let session2 = session::Identifier::Name("session2".to_string());

    let agent1 = manager.get_agent(session1.clone()).await.unwrap();
    let agent2 = manager.get_agent(session2.clone()).await.unwrap();

    // Different sessions get different agents
    assert!(!Arc::ptr_eq(&agent1, &agent2));

    // Same session gets same agent
    let agent1_again = manager.get_agent(session1).await.unwrap();
    assert!(Arc::ptr_eq(&agent1, &agent1_again));
}

#[tokio::test]
async fn test_cleanup_idle_agents() {
    // Verify idle agents are cleaned up
    let manager = AgentManager::new(Default::default()).await;
    let session = session::Identifier::Name("cleanup_test".to_string());

    let _agent = manager.get_agent(session.clone()).await.unwrap();

    // Verify agent exists
    assert!(manager.has_agent(&session).await);

    // Immediately cleanup with 0 idle time
    let removed = manager.cleanup_idle(Duration::from_secs(0)).await.unwrap();
    assert_eq!(removed, 1);

    // Verify agent was removed
    assert!(!manager.has_agent(&session).await);

    // Agent should be recreated on next access
    let _agent_new = manager.get_agent(session.clone()).await.unwrap();
    assert!(manager.has_agent(&session).await);
}

#[tokio::test]
async fn test_metrics_tracking() {
    let manager = AgentManager::new(Default::default()).await;

    // Create some agents
    let session1 = session::Identifier::Name("metrics1".to_string());
    let session2 = session::Identifier::Name("metrics2".to_string());

    let _agent1 = manager.get_agent(session1.clone()).await.unwrap();
    let _agent2 = manager.get_agent(session2.clone()).await.unwrap();

    // Access same session again (cache hit)
    let _agent1_again = manager.get_agent(session1).await.unwrap();

    let metrics = manager.get_metrics().await;
    assert_eq!(metrics.agents_created, 2);
    assert_eq!(metrics.cache_hits, 1);
    assert_eq!(metrics.cache_misses, 2);
    assert_eq!(metrics.active_agents, 2);

    // Cleanup
    let removed = manager.cleanup_idle(Duration::from_secs(0)).await.unwrap();
    assert_eq!(removed, 2);

    let metrics = manager.get_metrics().await;
    assert_eq!(metrics.agents_cleaned, 2);
    assert_eq!(metrics.active_agents, 0);
}

#[tokio::test]
async fn test_concurrent_access() {
    use tokio::task;

    let manager = Arc::new(AgentManager::new(Default::default()).await);
    let session = session::Identifier::Name("concurrent".to_string());

    // Spawn multiple tasks accessing the same session
    let mut handles = vec![];
    for _ in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let session_clone = session.clone();
        handles.push(task::spawn(async move {
            manager_clone.get_agent(session_clone).await.unwrap()
        }));
    }

    // Collect all agents
    let mut agents = vec![];
    for handle in handles {
        agents.push(handle.await.unwrap());
    }

    // All should be the same agent
    for agent in &agents[1..] {
        assert!(Arc::ptr_eq(&agents[0], agent));
    }

    // Only one agent should have been created
    let metrics = manager.get_metrics().await;
    assert_eq!(metrics.agents_created, 1);
    assert_eq!(metrics.cache_hits, 9);
}

#[tokio::test]
async fn test_remove_specific_agent() {
    let manager = AgentManager::new(Default::default()).await;
    let session = session::Identifier::Name("remove_test".to_string());

    // Create agent
    let _agent = manager.get_agent(session.clone()).await.unwrap();
    assert!(manager.has_agent(&session).await);

    // Remove specific agent
    manager.remove_agent(&session).await.unwrap();
    assert!(!manager.has_agent(&session).await);

    // Removing again should error
    let result = manager.remove_agent(&session).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execution_mode_update() {
    use goose::agents::manager::{ApprovalMode, ExecutionMode, InheritConfig};

    let manager = AgentManager::new(Default::default()).await;
    let session = session::Identifier::Name("exec_mode".to_string());
    let parent_session = session::Identifier::Name("parent".to_string());

    // Get agent with specific mode
    let mode = ExecutionMode::SubTask {
        parent: parent_session,
        inherit: InheritConfig::default(),
        approval_mode: ApprovalMode::default(),
    };

    let _agent = manager
        .get_agent_with_mode(session.clone(), mode.clone())
        .await
        .unwrap();

    // Verify the mode was set (would need access to internal state in real implementation)
    assert!(manager.has_agent(&session).await);
}

#[tokio::test]
async fn test_touch_session() {
    let manager = AgentManager::new(Default::default()).await;
    let session = session::Identifier::Name("touch_test".to_string());

    // Create agent
    let _agent = manager.get_agent(session.clone()).await.unwrap();

    // Touch should succeed
    manager.touch_session(&session).await.unwrap();

    // Touch non-existent session should fail
    let non_existent = session::Identifier::Name("nonexistent".to_string());
    let result = manager.touch_session(&non_existent).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_active_agent_count() {
    let manager = AgentManager::new(Default::default()).await;

    assert_eq!(manager.active_agent_count().await, 0);

    // Create some agents
    let session1 = session::Identifier::Name("count1".to_string());
    let session2 = session::Identifier::Name("count2".to_string());
    let session3 = session::Identifier::Name("count3".to_string());

    let _agent1 = manager.get_agent(session1.clone()).await.unwrap();
    assert_eq!(manager.active_agent_count().await, 1);

    let _agent2 = manager.get_agent(session2).await.unwrap();
    assert_eq!(manager.active_agent_count().await, 2);

    let _agent3 = manager.get_agent(session3).await.unwrap();
    assert_eq!(manager.active_agent_count().await, 3);

    // Remove one
    manager.remove_agent(&session1).await.unwrap();
    assert_eq!(manager.active_agent_count().await, 2);
}
