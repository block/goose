mod execution_tests {
    use goose::execution::adapters::*;
    use goose::execution::manager::AgentManager;
    use goose::execution::{ExecutionMode, SessionId};
    use std::sync::Arc;

    // ===== Type Tests =====

    #[test]
    fn test_execution_mode_helpers() {
        assert_eq!(ExecutionMode::chat(), ExecutionMode::Interactive);
        assert_eq!(ExecutionMode::scheduled(), ExecutionMode::Background);

        let parent = "parent-123".to_string();
        assert_eq!(
            ExecutionMode::task(parent.clone()),
            ExecutionMode::SubTask {
                parent_session: parent
            }
        );
    }

    #[test]
    fn test_session_id_generation() {
        let id1 = SessionId::generate();
        let id2 = SessionId::generate();

        // Should be unique
        assert_ne!(id1, id2);

        // Should be valid UUIDs
        assert_eq!(id1.0.len(), 36); // UUID string length
        assert_eq!(id2.0.len(), 36);
    }

    #[test]
    fn test_session_id_from_string() {
        let id_str = "test-session-123";
        let session_id = SessionId::from_string(id_str.to_string());
        assert_eq!(session_id.as_str(), id_str);

        // Test From trait
        let session_id2: SessionId = id_str.into();
        assert_eq!(session_id, session_id2);
    }

    #[test]
    fn test_display_traits() {
        let session = SessionId::from_string("display-test".to_string());
        assert_eq!(format!("{}", session), "display-test");

        let mode = ExecutionMode::Interactive;
        assert_eq!(format!("{}", mode), "interactive");

        let mode2 = ExecutionMode::task("parent-456".to_string());
        assert_eq!(format!("{}", mode2), "subtask(parent: parent-456)");
    }

    // ===== AgentManager Tests =====

    #[tokio::test]
    async fn test_session_isolation() {
        let manager = AgentManager::new();

        let session1 = SessionId::generate();
        let session2 = SessionId::generate();

        // Get agents for different sessions
        let agent1 = manager
            .get_agent(session1.clone(), ExecutionMode::chat())
            .await
            .unwrap();
        let agent2 = manager
            .get_agent(session2.clone(), ExecutionMode::chat())
            .await
            .unwrap();

        // Should be different agents
        assert!(!Arc::ptr_eq(&agent1, &agent2));

        // Getting same session should return same agent
        let agent1_again = manager
            .get_agent(session1, ExecutionMode::chat())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&agent1, &agent1_again));
    }

    #[tokio::test]
    async fn test_session_limit() {
        let manager = AgentManager::with_max_sessions(3);

        // Create 3 sessions
        let sessions: Vec<_> = (0..3)
            .map(|i| SessionId::from(format!("session-{}", i)))
            .collect();

        for session in &sessions {
            manager
                .get_agent(session.clone(), ExecutionMode::chat())
                .await
                .unwrap();
        }

        assert_eq!(manager.session_count().await, 3);

        // Creating 4th should evict oldest
        let new_session = SessionId::from("session-new");
        manager
            .get_agent(new_session, ExecutionMode::chat())
            .await
            .unwrap();

        // Should still have only 3 sessions
        assert_eq!(manager.session_count().await, 3);

        // First session should have been evicted
        assert!(!manager.has_session(&sessions[0]).await);
    }

    #[tokio::test]
    async fn test_remove_session() {
        let manager = AgentManager::new();
        let session = SessionId::from("remove-test");

        // Create session
        manager
            .get_agent(session.clone(), ExecutionMode::chat())
            .await
            .unwrap();
        assert!(manager.has_session(&session).await);

        // Remove it
        manager.remove_session(&session).await.unwrap();
        assert!(!manager.has_session(&session).await);

        // Removing again should error
        assert!(manager.remove_session(&session).await.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let manager = Arc::new(AgentManager::new());
        let session = SessionId::from("concurrent-test");

        // Spawn multiple tasks accessing the same session
        let mut handles = vec![];
        for _ in 0..10 {
            let mgr = Arc::clone(&manager);
            let sess = session.clone();
            let handle =
                tokio::spawn(
                    async move { mgr.get_agent(sess, ExecutionMode::chat()).await.unwrap() },
                );
            handles.push(handle);
        }

        // Collect all agents
        let agents: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All should be the same agent
        for agent in &agents[1..] {
            assert!(Arc::ptr_eq(&agents[0], agent));
        }

        // Only one session should exist
        assert_eq!(manager.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_different_modes_same_session() {
        let manager = AgentManager::new();
        let session_id = SessionId::from("mode-test");

        // Get agent with Interactive mode
        let agent1 = manager
            .get_agent(session_id.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        // Get same session with different mode - should return same agent
        // (mode is stored but agent is reused)
        let agent2 = manager
            .get_agent(session_id.clone(), ExecutionMode::Background)
            .await
            .unwrap();

        assert!(Arc::ptr_eq(&agent1, &agent2));
    }

    // ===== Adapter Tests =====

    #[tokio::test]
    async fn test_dynamic_task_adapter() {
        let manager = AgentManager::new();

        let parent = "parent-session-123".to_string();
        let instructions = "test task instructions".to_string();

        // Should create task and return ID
        let task_id = adapt_dynamic_task(&manager, parent.clone(), instructions.clone())
            .await
            .unwrap();

        assert!(!task_id.is_empty());

        // Should have created agent with SubTask mode
        let session = SessionId::from(task_id.clone());
        assert!(manager.has_session(&session).await);
    }

    #[tokio::test]
    async fn test_scheduler_adapter() {
        let manager = AgentManager::new();

        let job_id = "test-job-456".to_string();

        // Should execute without error
        let result = adapt_scheduler_job(&manager, job_id.clone()).await;
        assert!(result.is_ok());

        // Should have created session
        let session = SessionId::from(job_id);
        assert!(manager.has_session(&session).await);
    }

    #[tokio::test]
    async fn test_chat_adapter_with_session() {
        let manager = AgentManager::new();

        // With existing session ID
        let session_str = "existing-chat-789";
        let agent = adapt_chat_session(&manager, Some(session_str.to_string()))
            .await
            .unwrap();

        // Should have created the session
        let session = SessionId::from(session_str);
        assert!(manager.has_session(&session).await);

        // Getting same session should return same agent
        let agent2 = adapt_chat_session(&manager, Some(session_str.to_string()))
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&agent, &agent2));
    }

    #[tokio::test]
    async fn test_chat_adapter_without_session() {
        let manager = AgentManager::new();

        // Without session ID (should generate)
        let agent1 = adapt_chat_session(&manager, None).await.unwrap();
        let agent2 = adapt_chat_session(&manager, None).await.unwrap();

        // Should create different agents (different generated sessions)
        assert!(!Arc::ptr_eq(&agent1, &agent2));
    }

    #[tokio::test]
    async fn test_session_id_adapter() {
        // With existing ID
        let id = adapt_session_id(Some("test-123".to_string()));
        assert_eq!(id.as_str(), "test-123");

        // Without ID (should generate valid UUID)
        let generated = adapt_session_id(None);
        // Verify it's a valid UUID v4 by trying to parse it
        let uuid_result = uuid::Uuid::parse_str(generated.as_str());
        assert!(uuid_result.is_ok(), "Generated ID should be valid UUID");
        let uuid = uuid_result.unwrap();
        assert_eq!(uuid.get_version(), Some(uuid::Version::Random));
    }

    #[tokio::test]
    async fn test_multiple_adapters_isolation() {
        let manager = AgentManager::new();

        // Create different sessions through different adapters
        let task_id = adapt_dynamic_task(&manager, "parent".to_string(), "task".to_string())
            .await
            .unwrap();

        let job_id = "job-123".to_string();
        adapt_scheduler_job(&manager, job_id.clone()).await.unwrap();

        let _chat_agent = adapt_chat_session(&manager, Some("chat-456".to_string()))
            .await
            .unwrap();

        // Should have 3 different sessions
        assert_eq!(manager.session_count().await, 3);

        // Each should be isolated
        assert!(manager.has_session(&SessionId::from(task_id)).await);
        assert!(manager.has_session(&SessionId::from(job_id)).await);
        assert!(manager.has_session(&SessionId::from("chat-456")).await);
    }

    // ===== New Tests for Missing Coverage =====

    #[tokio::test]
    async fn test_concurrent_session_creation_race_condition() {
        // Test that concurrent attempts to create the same new session ID
        // result in only one agent being created (tests double-check pattern)
        let manager = Arc::new(AgentManager::new());
        let session_id = SessionId::from("race-condition-test");

        // Spawn multiple tasks trying to create the same NEW session simultaneously
        let mut handles = vec![];
        for _ in 0..20 {
            let mgr = Arc::clone(&manager);
            let sess = session_id.clone();
            let handle = tokio::spawn(async move {
                mgr.get_agent(sess, ExecutionMode::Interactive)
                    .await
                    .unwrap()
            });
            handles.push(handle);
        }

        // Collect all agents
        let agents: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All should be the same agent (double-check pattern should prevent duplicates)
        for agent in &agents[1..] {
            assert!(
                Arc::ptr_eq(&agents[0], agent),
                "All concurrent requests should get the same agent"
            );
        }

        // Only one session should exist
        assert_eq!(manager.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_edge_case_max_sessions_zero() {
        // Test behavior with max_sessions = 0 (should still allow creating sessions)
        let manager = AgentManager::with_max_sessions(0);

        let session1 = SessionId::from("session-1");
        let result = manager
            .get_agent(session1.clone(), ExecutionMode::Interactive)
            .await;

        // Should succeed even with max_sessions = 0
        assert!(result.is_ok());
        assert_eq!(manager.session_count().await, 1);

        // Creating another should evict the first immediately
        let session2 = SessionId::from("session-2");
        manager
            .get_agent(session2.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        // Should have evicted session1
        assert!(!manager.has_session(&session1).await);
        assert!(manager.has_session(&session2).await);
        assert_eq!(manager.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_edge_case_max_sessions_one() {
        // Test behavior with max_sessions = 1
        let manager = AgentManager::with_max_sessions(1);

        let session1 = SessionId::from("only-session");
        manager
            .get_agent(session1.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 1);

        // Creating second session should evict the first
        let session2 = SessionId::from("new-session");
        manager
            .get_agent(session2.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        assert!(!manager.has_session(&session1).await);
        assert!(manager.has_session(&session2).await);
        assert_eq!(manager.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_configure_default_provider() {
        use std::env;

        // Save original env vars
        let original_provider = env::var("GOOSE_DEFAULT_PROVIDER").ok();
        let original_model = env::var("GOOSE_DEFAULT_MODEL").ok();

        // Set test env vars
        env::set_var("GOOSE_DEFAULT_PROVIDER", "openai");
        env::set_var("GOOSE_DEFAULT_MODEL", "gpt-4o-mini");

        let manager = AgentManager::new();
        let result = manager.configure_default_provider().await;

        // Should succeed (though provider creation might fail without API key)
        // We're testing the configuration logic, not the provider itself
        assert!(result.is_ok());

        // Restore original env vars
        if let Some(val) = original_provider {
            env::set_var("GOOSE_DEFAULT_PROVIDER", val);
        } else {
            env::remove_var("GOOSE_DEFAULT_PROVIDER");
        }
        if let Some(val) = original_model {
            env::set_var("GOOSE_DEFAULT_MODEL", val);
        } else {
            env::remove_var("GOOSE_DEFAULT_MODEL");
        }
    }

    #[tokio::test]
    async fn test_set_default_provider() {
        use goose::providers::testprovider::TestProvider;
        use std::sync::Arc;

        // Test the set methods work
        let manager = AgentManager::new();

        // Create a test provider for replaying (doesn't need inner provider)
        let temp_file = format!(
            "{}/test_provider_{}.json",
            std::env::temp_dir().display(),
            std::process::id()
        );

        // Create an empty test provider (will fail on actual use but that's ok for this test)
        let test_provider = TestProvider::new_replaying(&temp_file)
            .unwrap_or_else(|_| TestProvider::new_replaying("/tmp/dummy.json").unwrap());

        manager.set_default_provider(Arc::new(test_provider)).await;

        // Create a session and verify it gets created
        let session = SessionId::from("provider-test");
        let _agent = manager
            .get_agent(session.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        // Agent should be created and session should exist
        assert!(manager.has_session(&session).await);
    }

    #[tokio::test]
    async fn test_eviction_updates_last_used() {
        // Test that accessing a session updates its last_used timestamp
        // and affects eviction order
        let manager = AgentManager::with_max_sessions(2);

        // Create two sessions
        let session1 = SessionId::from("session-1");
        let session2 = SessionId::from("session-2");

        manager
            .get_agent(session1.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        manager
            .get_agent(session2.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        // Access session1 again to update its last_used
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        manager
            .get_agent(session1.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        // Now create a third session - should evict session2 (least recently used)
        let session3 = SessionId::from("session-3");
        manager
            .get_agent(session3.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        // session1 should still exist (recently accessed)
        // session2 should be evicted (least recently used)
        assert!(manager.has_session(&session1).await);
        assert!(!manager.has_session(&session2).await);
        assert!(manager.has_session(&session3).await);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_session_error() {
        // Test that removing a non-existent session returns an error
        let manager = AgentManager::new();
        let session = SessionId::from("never-created");

        let result = manager.remove_session(&session).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
