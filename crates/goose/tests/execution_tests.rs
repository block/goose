mod execution_tests {
    use goose::execution::manager::AgentManager;
    use goose::execution::SessionExecutionMode;
    use serial_test::serial;
    use std::sync::Arc;

    #[test]
    fn test_execution_mode_constructors() {
        assert_eq!(
            SessionExecutionMode::chat(),
            SessionExecutionMode::Interactive
        );
        assert_eq!(
            SessionExecutionMode::scheduled(),
            SessionExecutionMode::Background
        );

        let parent = "parent-123".to_string();
        assert_eq!(
            SessionExecutionMode::task(parent.clone()),
            SessionExecutionMode::SubTask {
                parent_session: parent
            }
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_session_isolation() {
        AgentManager::reset_for_test();
        let manager = AgentManager::instance().await.unwrap();

        let session1 = uuid::Uuid::new_v4().to_string();
        let session2 = uuid::Uuid::new_v4().to_string();

        let agent1 = manager
            .get_or_create_agent(session1.clone(), SessionExecutionMode::Interactive)
            .await
            .unwrap();

        let agent2 = manager
            .get_or_create_agent(session2.clone(), SessionExecutionMode::Interactive)
            .await
            .unwrap();

        // Different sessions should have different agents
        assert!(!Arc::ptr_eq(&agent1, &agent2));

        // Getting the same session should return the same agent
        let agent1_again = manager
            .get_or_create_agent(session1, SessionExecutionMode::chat())
            .await
            .unwrap();

        assert!(Arc::ptr_eq(&agent1, &agent1_again));

        AgentManager::reset_for_test();
    }

    #[tokio::test]
    #[serial]
    async fn test_session_limit() {
        AgentManager::reset_for_test();
        let manager = AgentManager::instance().await.unwrap();

        let sessions: Vec<_> = (0..100).map(|i| format!("session-{}", i)).collect();

        for session in &sessions {
            manager
                .get_or_create_agent(session.clone(), SessionExecutionMode::chat())
                .await
                .unwrap();
        }

        // Create a new session after cleanup
        let new_session = "new-session".to_string();
        let _new_agent = manager
            .get_or_create_agent(new_session, SessionExecutionMode::chat())
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 100);
    }

    #[tokio::test]
    #[serial]
    async fn test_remove_session() {
        AgentManager::reset_for_test();
        let manager = AgentManager::instance().await.unwrap();
        let session = String::from("remove-test");

        manager
            .get_or_create_agent(session.clone(), SessionExecutionMode::chat())
            .await
            .unwrap();
        assert!(manager.has_session(&session).await);

        manager.remove_session(&session).await.unwrap();
        assert!(!manager.has_session(&session).await);

        assert!(manager.remove_session(&session).await.is_err());

        AgentManager::reset_for_test();
    }

    #[tokio::test]
    #[serial]
    async fn test_concurrent_access() {
        AgentManager::reset_for_test();
        let manager = AgentManager::instance().await.unwrap();
        let session = String::from("concurrent-test");

        let mut handles = vec![];
        for _ in 0..10 {
            let mgr = Arc::clone(&manager);
            let sess = session.clone();
            handles.push(tokio::spawn(async move {
                mgr.get_or_create_agent(sess, SessionExecutionMode::chat())
                    .await
                    .unwrap()
            }));
        }

        let agents: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        for agent in &agents[1..] {
            assert!(Arc::ptr_eq(&agents[0], agent));
        }

        assert_eq!(manager.session_count().await, 1);

        AgentManager::reset_for_test();
    }

    #[tokio::test]
    #[serial]
    async fn test_different_modes_same_session() {
        AgentManager::reset_for_test();
        let manager = AgentManager::instance().await.unwrap();
        let session_id = String::from("mode-test");

        // Create initial agent
        let agent1 = manager
            .get_or_create_agent(session_id.clone(), SessionExecutionMode::chat())
            .await
            .unwrap();

        // Get same session with different mode - should return same agent
        // (mode is stored but agent is reused)
        let agent2 = manager
            .get_or_create_agent(session_id.clone(), SessionExecutionMode::Background)
            .await
            .unwrap();

        assert!(Arc::ptr_eq(&agent1, &agent2));

        AgentManager::reset_for_test();
    }

    #[tokio::test]
    #[serial]
    async fn test_concurrent_session_creation_race_condition() {
        // Test that concurrent attempts to create the same new session ID
        // result in only one agent being created (tests double-check pattern)
        AgentManager::reset_for_test();
        let manager = AgentManager::instance().await.unwrap();
        let session_id = String::from("race-condition-test");

        // Spawn multiple tasks trying to create the same NEW session simultaneously
        let mut handles = vec![];
        for _ in 0..20 {
            let sess = session_id.clone();
            let mgr_clone = Arc::clone(&manager);
            handles.push(tokio::spawn(async move {
                mgr_clone
                    .get_or_create_agent(sess, SessionExecutionMode::Interactive)
                    .await
                    .unwrap()
            }));
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

        AgentManager::reset_for_test();
    }

    #[tokio::test]
    #[serial]
    async fn test_configure_default_provider() {
        use std::env;

        AgentManager::reset_for_test();

        let original_provider = env::var("GOOSE_DEFAULT_PROVIDER").ok();
        let original_model = env::var("GOOSE_DEFAULT_MODEL").ok();

        env::set_var("GOOSE_DEFAULT_PROVIDER", "openai");
        env::set_var("GOOSE_DEFAULT_MODEL", "gpt-4o-mini");

        let manager = AgentManager::instance().await.unwrap();
        let result = manager.configure_default_provider().await;

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

        AgentManager::reset_for_test();
    }

    #[tokio::test]
    #[serial]
    async fn test_set_default_provider() {
        use goose::providers::testprovider::TestProvider;
        use std::sync::Arc;

        AgentManager::reset_for_test();
        let manager = AgentManager::instance().await.unwrap();

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

        let session = String::from("provider-test");
        let _agent = manager
            .get_or_create_agent(session.clone(), SessionExecutionMode::Interactive)
            .await
            .unwrap();

        assert!(manager.has_session(&session).await);

        AgentManager::reset_for_test();
    }

    #[tokio::test]
    #[serial]
    async fn test_eviction_updates_last_used() {
        AgentManager::reset_for_test();
        // Test that accessing a session updates its last_used timestamp
        // and affects eviction order
        let manager = AgentManager::instance().await.unwrap();

        let sessions: Vec<_> = (0..100).map(|i| format!("session-{}", i)).collect();

        for session in &sessions {
            manager
                .get_or_create_agent(session.clone(), SessionExecutionMode::chat())
                .await
                .unwrap();
            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Access the first session again to update its last_used
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        manager
            .get_or_create_agent(sessions[0].clone(), SessionExecutionMode::Interactive)
            .await
            .unwrap();

        // Now create a 101st session - should evict session2 (least recently used)
        let session101 = String::from("session-101");
        manager
            .get_or_create_agent(session101.clone(), SessionExecutionMode::Interactive)
            .await
            .unwrap();

        // session1 should still exist (recently accessed)
        // session2 should be evicted (least recently used)
        assert!(manager.has_session(&sessions[0]).await);
        assert!(!manager.has_session(&sessions[1]).await);
        assert!(manager.has_session(&session101).await);
        AgentManager::reset_for_test();
    }

    #[tokio::test]
    #[serial]
    async fn test_remove_nonexistent_session_error() {
        // Test that removing a non-existent session returns an error
        AgentManager::reset_for_test();
        let manager = AgentManager::instance().await.unwrap();
        let session = String::from("never-created");

        let result = manager.remove_session(&session).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        AgentManager::reset_for_test();
    }
}
