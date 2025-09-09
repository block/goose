use std::sync::Arc;

use goose::agents::extension::ExtensionConfig;
use goose::agents::manager::{AgentManager, AgentManagerConfig};
use goose::session;
use rmcp::model::Tool;

/// Create a simple frontend extension for testing
fn create_test_extension(name: &str) -> ExtensionConfig {
    ExtensionConfig::Frontend {
        name: name.to_string(),
        tools: vec![Tool {
            name: format!("{}_tool", name).into(),
            description: Some(format!("Tool from {} extension", name).into()),
            input_schema: Arc::new(
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            output_schema: None,
            annotations: None,
        }],
        instructions: Some(format!("Instructions for {}", name)),
        bundled: Some(false),
        available_tools: vec![],
    }
}

#[tokio::test]
async fn test_extension_isolation_between_sessions() {
    // Extensions added to one session should not appear in another
    let manager = AgentManager::new(AgentManagerConfig::default()).await;

    let session1 = session::Identifier::Name("ext_test_1".to_string());
    let session2 = session::Identifier::Name("ext_test_2".to_string());

    let agent1 = manager.get_agent(session1.clone()).await.unwrap();
    let agent2 = manager.get_agent(session2.clone()).await.unwrap();

    // Add different extensions to each agent
    let ext1 = create_test_extension("extension1");
    let ext2 = create_test_extension("extension2");

    agent1.add_extension(ext1).await.unwrap();
    agent2.add_extension(ext2).await.unwrap();

    // Check tools for each agent
    let tools1 = agent1.list_tools(None).await;
    let tools2 = agent2.list_tools(None).await;

    // Agent1 should have extension1_tool but not extension2_tool
    assert!(tools1.iter().any(|t| t.name == "extension1_tool"));
    assert!(!tools1.iter().any(|t| t.name == "extension2_tool"));

    // Agent2 should have extension2_tool but not extension1_tool
    assert!(tools2.iter().any(|t| t.name == "extension2_tool"));
    assert!(!tools2.iter().any(|t| t.name == "extension1_tool"));
}

#[tokio::test]
async fn test_extension_persistence_within_session() {
    // Extensions should persist when an agent is retrieved multiple times
    let manager = AgentManager::new(AgentManagerConfig::default()).await;
    let session = session::Identifier::Name("ext_persistence".to_string());

    // Add extension to agent
    let agent1 = manager.get_agent(session.clone()).await.unwrap();
    let ext = create_test_extension("persistent");
    agent1.add_extension(ext).await.unwrap();

    // Verify extension is present
    let tools1 = agent1.list_tools(None).await;
    assert!(tools1.iter().any(|t| t.name == "persistent_tool"));

    // Get agent again (from cache)
    let agent2 = manager.get_agent(session.clone()).await.unwrap();

    // Extension should still be present
    let tools2 = agent2.list_tools(None).await;
    assert!(tools2.iter().any(|t| t.name == "persistent_tool"));

    // Verify it's the same agent instance
    assert!(Arc::ptr_eq(&agent1, &agent2));
}

#[tokio::test]
async fn test_extension_removal_isolation() {
    // Removing an extension from one session shouldn't affect others
    let manager = AgentManager::new(AgentManagerConfig::default()).await;

    let session1 = session::Identifier::Name("ext_remove_1".to_string());
    let session2 = session::Identifier::Name("ext_remove_2".to_string());

    let agent1 = manager.get_agent(session1).await.unwrap();
    let agent2 = manager.get_agent(session2).await.unwrap();

    // Add same extension to both agents
    let ext1 = create_test_extension("shared");
    let ext2 = create_test_extension("shared");

    agent1.add_extension(ext1).await.unwrap();
    agent2.add_extension(ext2).await.unwrap();

    // Verify both have the extension
    assert!(agent1
        .list_tools(None)
        .await
        .iter()
        .any(|t| t.name == "shared_tool"));
    assert!(agent2
        .list_tools(None)
        .await
        .iter()
        .any(|t| t.name == "shared_tool"));

    // Remove from agent1 only
    agent1.remove_extension("shared").await.unwrap();

    // Agent1 should not have it anymore
    assert!(!agent1
        .list_tools(None)
        .await
        .iter()
        .any(|t| t.name == "shared_tool"));

    // Agent2 should still have it
    assert!(agent2
        .list_tools(None)
        .await
        .iter()
        .any(|t| t.name == "shared_tool"));
}

#[tokio::test]
async fn test_concurrent_extension_operations() {
    // Test that concurrent extension operations on different sessions don't interfere
    let manager = Arc::new(AgentManager::new(AgentManagerConfig::default()).await);

    let mut handles = vec![];

    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);

        let handle = tokio::spawn(async move {
            let session = session::Identifier::Name(format!("concurrent_ext_{}", i));
            let agent = manager_clone.get_agent(session).await.unwrap();

            // Add multiple extensions
            for j in 0..3 {
                let ext = create_test_extension(&format!("ext_{}_{}", i, j));
                agent.add_extension(ext).await.unwrap();
            }

            // Verify all extensions are present
            let tools = agent.list_tools(None).await;
            for j in 0..3 {
                let tool_name = format!("ext_{}_{}__tool", i, j);
                assert!(
                    tools.iter().any(|t| t.name == tool_name),
                    "Missing tool: {}",
                    tool_name
                );
            }

            // Remove one extension
            agent
                .remove_extension(&format!("ext_{}_1", i))
                .await
                .unwrap();

            // Verify it's gone but others remain
            let tools_after = agent.list_tools(None).await;
            assert!(!tools_after
                .iter()
                .any(|t| t.name == format!("ext_{}_1__tool", i)));
            assert!(tools_after
                .iter()
                .any(|t| t.name == format!("ext_{}_0__tool", i)));
            assert!(tools_after
                .iter()
                .any(|t| t.name == format!("ext_{}_2__tool", i)));
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_extension_state_after_cleanup() {
    // Test that extensions are lost after agent cleanup and recreation
    let config = AgentManagerConfig {
        max_idle_duration: std::time::Duration::from_millis(1), // Very short for testing
        ..Default::default()
    };

    let manager = AgentManager::new(config).await;
    let session = session::Identifier::Name("cleanup_ext_test".to_string());

    // Add extension to agent
    let agent1 = manager.get_agent(session.clone()).await.unwrap();
    let ext = create_test_extension("temporary");
    agent1.add_extension(ext.clone()).await.unwrap();

    // Verify extension is present
    assert!(agent1
        .list_tools(None)
        .await
        .iter()
        .any(|t| t.name == "temporary_tool"));

    // Wait for idle timeout
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Trigger cleanup
    let removed = manager.cleanup().await.unwrap();
    assert_eq!(removed, 1, "Should have cleaned up one agent");

    // Get agent again (should be new instance)
    let agent2 = manager.get_agent(session.clone()).await.unwrap();

    // Extension should be gone (fresh agent)
    assert!(!agent2
        .list_tools(None)
        .await
        .iter()
        .any(|t| t.name == "temporary_tool"));

    // Verify it's a different agent instance
    assert!(!Arc::ptr_eq(&agent1, &agent2));
}
