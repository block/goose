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

    #[test]
    fn test_session_id_equality() {
        let id1 = SessionId::from("test-id");
        let id2 = SessionId::from("test-id");
        let id3 = SessionId::from("different-id");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_execution_mode_equality() {
        let mode1 = ExecutionMode::Interactive;
        let mode2 = ExecutionMode::Interactive;
        let mode3 = ExecutionMode::Background;

        assert_eq!(mode1, mode2);
        assert_ne!(mode1, mode3);

        let subtask1 = ExecutionMode::SubTask {
            parent_session: "parent".to_string(),
        };
        let subtask2 = ExecutionMode::SubTask {
            parent_session: "parent".to_string(),
        };
        let subtask3 = ExecutionMode::SubTask {
            parent_session: "other".to_string(),
        };

        assert_eq!(subtask1, subtask2);
        assert_ne!(subtask1, subtask3);
    }

    #[test]
    fn test_session_id_clone() {
        let original = SessionId::from("clone-test");
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.as_str(), cloned.as_str());
    }

    #[test]
    fn test_execution_mode_clone() {
        let original = ExecutionMode::SubTask {
            parent_session: "parent".to_string(),
        };
        let cloned = original.clone();

        assert_eq!(original, cloned);
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
    async fn test_execution_modes() {
        let manager = AgentManager::new();

        // Create agents with different modes
        let interactive = manager
            .get_agent(SessionId::generate(), ExecutionMode::Interactive)
            .await
            .unwrap();

        let background = manager
            .get_agent(SessionId::generate(), ExecutionMode::Background)
            .await
            .unwrap();

        let subtask = manager
            .get_agent(
                SessionId::generate(),
                ExecutionMode::SubTask {
                    parent_session: "parent-123".to_string(),
                },
            )
            .await
            .unwrap();

        // All should be different agents
        assert!(!Arc::ptr_eq(&interactive, &background));
        assert!(!Arc::ptr_eq(&background, &subtask));
        assert!(!Arc::ptr_eq(&interactive, &subtask));
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
    async fn test_execute_recipe_stub() {
        let manager = AgentManager::new();
        let session = SessionId::generate();

        let recipe = serde_json::json!({
            "name": "test_recipe",
            "instructions": "test"
        });

        let result = manager
            .execute_recipe(session.clone(), recipe, ExecutionMode::Interactive)
            .await
            .unwrap();

        // Verify stub response
        assert_eq!(result["status"], "ready_for_future");
        assert_eq!(result["agent_created"], true);
        assert_eq!(result["session_id"], session.to_string());
    }

    #[tokio::test]
    async fn test_session_persistence_across_requests() {
        let manager = AgentManager::new();
        let session_id = SessionId::from("persistent-session");

        // First request creates agent
        let agent1 = manager
            .get_agent(session_id.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        // Simulate time passing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Second request should get same agent
        let agent2 = manager
            .get_agent(session_id.clone(), ExecutionMode::Interactive)
            .await
            .unwrap();

        assert!(Arc::ptr_eq(&agent1, &agent2));
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

        // Without ID (should generate)
        let generated = adapt_session_id(None);
        assert_eq!(generated.as_str().len(), 36); // UUID length
    }

    #[tokio::test]
    async fn test_adapter_session_reuse() {
        let manager = AgentManager::new();

        // Create a session through chat adapter
        let session_id = "shared-session".to_string();
        let chat_agent = adapt_chat_session(&manager, Some(session_id.clone()))
            .await
            .unwrap();

        // Verify session exists
        assert!(
            manager
                .has_session(&SessionId::from(session_id.as_str()))
                .await
        );

        // Use same session through another adapter call
        let chat_agent2 = adapt_chat_session(&manager, Some(session_id.clone()))
            .await
            .unwrap();

        // Should be the same agent
        assert!(Arc::ptr_eq(&chat_agent, &chat_agent2));
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
}
