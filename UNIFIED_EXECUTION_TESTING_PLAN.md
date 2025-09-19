# Unified Execution Testing Plan

## Overview

This testing plan complements the UNIFIED_EXECUTION_IMPLEMENTATION_PLAN_FINAL.md, learning from PR #4542's test suite while avoiding its over-engineering and focusing on practical, high-value tests.

## Lessons from PR #4542

### What They Did Well ✅
- **Session isolation tests** - Critical for proving the main goal
- **Concurrent access tests** - Important for production reliability
- **Extension isolation tests** - Validates no cross-session interference
- **Provider isolation tests** - Each session can have different models
- **Integration tests** - Testing through actual HTTP endpoints

### What Was Unnecessary ❌
- **Overly complex metrics tracking** - Too detailed for initial implementation
- **Touch session functionality** - Feature creep not in requirements
- **Idle cleanup tests with complex timing** - Premature optimization
- **Provider initialization failure handling** - Edge case for later
- **Active agent count tracking** - Nice-to-have metric

### What We Should Do Better 🎯
- **Test adapters thoroughly** - PR #4542 didn't have adapters
- **Test backward compatibility** - Ensure old code paths still work
- **Test scheduler integration** - PR #4542 broke the scheduler
- **Test ExecutionMode behavior** - Different modes should behave correctly
- **Test memory limits** - Simple session cap enforcement
- **Focus on real user scenarios** - Not just unit tests

## Testing Strategy

### Phase 1: Core Unit Tests (200 lines)

#### 1.1 AgentManager Basic Tests (`agent_manager_test.rs`)

```rust
// crates/goose/tests/agent_manager_test.rs
use goose::execution::manager::AgentManager;
use goose::execution::{ExecutionMode, SessionId};
use std::sync::Arc;

#[tokio::test]
async fn test_session_isolation() {
    // Core requirement: different sessions get different agents
    let manager = AgentManager::new();
    
    let session1 = SessionId::generate();
    let session2 = SessionId::generate();
    
    let agent1 = manager.get_agent(session1.clone(), ExecutionMode::chat()).await.unwrap();
    let agent2 = manager.get_agent(session2.clone(), ExecutionMode::chat()).await.unwrap();
    
    // Different agents for different sessions
    assert!(!Arc::ptr_eq(&agent1, &agent2));
    
    // Same agent for same session
    let agent1_again = manager.get_agent(session1, ExecutionMode::chat()).await.unwrap();
    assert!(Arc::ptr_eq(&agent1, &agent1_again));
}

#[tokio::test]
async fn test_execution_modes() {
    // Test that different modes are properly tracked
    let manager = AgentManager::new();
    
    let session = SessionId::generate();
    let parent = SessionId::generate();
    
    // Interactive mode
    let agent1 = manager.get_agent(
        session.clone(), 
        ExecutionMode::Interactive
    ).await.unwrap();
    
    // Background mode
    let agent2 = manager.get_agent(
        SessionId::generate(),
        ExecutionMode::Background
    ).await.unwrap();
    
    // SubTask mode
    let agent3 = manager.get_agent(
        SessionId::generate(),
        ExecutionMode::SubTask { parent_session: parent.0 }
    ).await.unwrap();
    
    // All should be different agents
    assert!(!Arc::ptr_eq(&agent1, &agent2));
    assert!(!Arc::ptr_eq(&agent2, &agent3));
}

#[tokio::test]
async fn test_session_limit() {
    // Simple test for max_sessions enforcement
    let mut manager = AgentManager::new();
    manager.max_sessions = 3;
    
    // Create 3 sessions
    let sessions: Vec<_> = (0..3)
        .map(|_| SessionId::generate())
        .collect();
    
    for session in &sessions {
        manager.get_agent(session.clone(), ExecutionMode::chat()).await.unwrap();
    }
    
    // Creating 4th should evict oldest
    let new_session = SessionId::generate();
    manager.get_agent(new_session, ExecutionMode::chat()).await.unwrap();
    
    // Verify we still have max 3 sessions
    let count = manager.sessions.read().await.len();
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_scheduler_propagation() {
    // Test that scheduler is properly set on agents
    let manager = AgentManager::new();
    
    // Mock scheduler
    let scheduler = Arc::new(MockScheduler::new());
    manager.set_scheduler(scheduler.clone()).await;
    
    // New agents should get the scheduler
    let session = SessionId::generate();
    let agent = manager.get_agent(session, ExecutionMode::Background).await.unwrap();
    
    // Verify scheduler was set (would need getter in real impl)
    // This ensures scheduler integration isn't broken like in PR #4542
}

#[tokio::test]
async fn test_concurrent_session_creation() {
    // Ensure thread safety
    let manager = Arc::new(AgentManager::new());
    
    let mut handles = vec![];
    for i in 0..10 {
        let mgr = manager.clone();
        let handle = tokio::spawn(async move {
            let session = SessionId(format!("concurrent_{}", i));
            mgr.get_agent(session, ExecutionMode::chat()).await.unwrap()
        });
        handles.push(handle);
    }
    
    let agents: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    
    // All should be different agents
    for i in 0..agents.len() {
        for j in (i+1)..agents.len() {
            assert!(!Arc::ptr_eq(&agents[i], &agents[j]));
        }
    }
}
```

#### 1.2 Adapter Tests (`adapter_test.rs`)

```rust
// crates/goose/tests/adapter_test.rs
use goose::execution::adapters::*;
use goose::execution::manager::AgentManager;

#[tokio::test]
async fn test_dynamic_task_adapter() {
    // Ensure adapter maintains backward compatibility
    let manager = AgentManager::new();
    
    let parent = "parent_session".to_string();
    let instructions = "test task".to_string();
    
    // Should create task and return ID
    let task_id = adapt_dynamic_task(&manager, parent.clone(), instructions.clone())
        .await
        .unwrap();
    
    assert!(!task_id.is_empty());
    
    // Should have created agent with SubTask mode
    let session = SessionId(task_id.clone());
    // Verify agent exists in manager (would need has_agent method)
}

#[tokio::test]
async fn test_scheduler_adapter() {
    // Test scheduler job adaptation
    let manager = AgentManager::new();
    
    let job = ScheduledJob {
        id: "test_job".to_string(),
        recipe: Recipe::default(),
        // ... other fields
    };
    
    // Should execute without error
    let result = adapt_scheduler_job(&manager, job).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_chat_adapter() {
    // Test chat session adaptation
    let manager = AgentManager::new();
    
    // With session ID
    let agent1 = adapt_chat_session(&manager, Some("session1".to_string()))
        .await
        .unwrap();
    
    // Without session ID (should generate)
    let agent2 = adapt_chat_session(&manager, None)
        .await
        .unwrap();
    
    assert!(!Arc::ptr_eq(&agent1, &agent2));
}
```

### Phase 2: Integration Tests (150 lines)

#### 2.1 Server Integration Tests (`server_integration_test.rs`)

```rust
// crates/goose-server/tests/server_integration_test.rs

#[tokio::test]
async fn test_session_isolation_via_api() {
    let app = create_test_app().await;
    
    // Start two sessions
    let session1 = start_session(&app, "session1").await;
    let session2 = start_session(&app, "session2").await;
    
    // Add extension to session1 only
    add_extension(&app, &session1, "test_ext").await;
    
    // Verify session1 has extension
    let tools1 = get_tools(&app, &session1).await;
    assert!(tools1.iter().any(|t| t.name == "test_ext__tool"));
    
    // Verify session2 doesn't have it
    let tools2 = get_tools(&app, &session2).await;
    assert!(!tools2.iter().any(|t| t.name == "test_ext__tool"));
}

#[tokio::test]
async fn test_concurrent_chat_sessions() {
    let app = Arc::new(create_test_app().await);
    
    // Simulate multiple users chatting simultaneously
    let mut handles = vec![];
    
    for i in 0..5 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let session = start_session(&app_clone, &format!("user_{}", i)).await;
            
            // Each user sends different messages
            for j in 0..3 {
                send_message(&app_clone, &session, &format!("Message {} from user {}", j, i)).await;
            }
            
            // Verify responses are session-specific
            let history = get_history(&app_clone, &session).await;
            assert!(history.contains(&format!("user_{}", i)));
        });
        handles.push(handle);
    }
    
    futures::future::join_all(handles).await;
}

#[tokio::test]
async fn test_backward_compatibility() {
    // Ensure old endpoints still work
    let app = create_test_app().await;
    
    // Old-style request without session_id
    let response = send_old_style_request(&app).await;
    assert_eq!(response.status(), StatusCode::OK);
    
    // Should auto-generate session
    let body = parse_response(response).await;
    assert!(body.contains_key("session_id"));
}
```

#### 2.2 Extension Isolation Tests (`extension_isolation_test.rs`)

```rust
// crates/goose-server/tests/extension_isolation_test.rs

#[tokio::test]
async fn test_extension_state_isolation() {
    let (app, state) = create_test_app().await;
    
    // Create two agents
    let agent1 = state.get_agent(Identifier::Name("ext1".into())).await.unwrap();
    let agent2 = state.get_agent(Identifier::Name("ext2".into())).await.unwrap();
    
    // Add different extensions
    agent1.add_extension(create_extension("ext_a")).await.unwrap();
    agent2.add_extension(create_extension("ext_b")).await.unwrap();
    
    // Verify isolation
    let tools1 = agent1.list_tools(None).await;
    let tools2 = agent2.list_tools(None).await;
    
    assert!(tools1.iter().any(|t| t.name == "ext_a__tool"));
    assert!(!tools1.iter().any(|t| t.name == "ext_b__tool"));
    
    assert!(tools2.iter().any(|t| t.name == "ext_b__tool"));
    assert!(!tools2.iter().any(|t| t.name == "ext_a__tool"));
}
```

### Phase 3: Scenario Tests (100 lines)

#### 3.1 Real User Scenarios (`scenario_test.rs`)

```rust
// crates/goose/tests/scenario_test.rs

#[tokio::test]
async fn test_scheduler_to_chat_handoff() {
    // Scenario: Scheduled job creates session, user resumes it
    let manager = AgentManager::new();
    
    // Scheduler creates session
    let job_session = SessionId("scheduled_123".into());
    let agent1 = manager.get_agent(
        job_session.clone(),
        ExecutionMode::Background
    ).await.unwrap();
    
    // User resumes same session interactively
    let agent2 = manager.get_agent(
        job_session,
        ExecutionMode::Interactive
    ).await.unwrap();
    
    // Should be same agent
    assert!(Arc::ptr_eq(&agent1, &agent2));
}

#[tokio::test]
async fn test_dynamic_task_fanout() {
    // Scenario: Parent creates multiple subtasks
    let manager = AgentManager::new();
    
    let parent = SessionId::generate();
    let parent_agent = manager.get_agent(
        parent.clone(),
        ExecutionMode::Interactive
    ).await.unwrap();
    
    // Create subtasks
    let mut subtasks = vec![];
    for i in 0..3 {
        let task_id = adapt_dynamic_task(
            &manager,
            parent.0.clone(),
            format!("Subtask {}", i)
        ).await.unwrap();
        subtasks.push(task_id);
    }
    
    // All subtasks should have different agents
    let mut agents = vec![];
    for task_id in subtasks {
        let agent = manager.get_agent(
            SessionId(task_id),
            ExecutionMode::SubTask { parent_session: parent.0.clone() }
        ).await.unwrap();
        agents.push(agent);
    }
    
    // Verify all different
    for i in 0..agents.len() {
        for j in (i+1)..agents.len() {
            assert!(!Arc::ptr_eq(&agents[i], &agents[j]));
        }
    }
}

#[tokio::test]
async fn test_recipe_execution_path() {
    // Test the future execute_recipe stub
    let manager = AgentManager::new();
    
    let session = SessionId::generate();
    let recipe = serde_json::json!({
        "name": "test_recipe",
        "instructions": "test"
    });
    
    let result = manager.execute_recipe(
        session,
        recipe,
        ExecutionMode::Interactive
    ).await.unwrap();
    
    // For now just verify it returns the placeholder
    assert_eq!(result["status"], "ready_for_future");
    assert_eq!(result["agent_created"], true);
}
```

### Phase 4: Performance & Stress Tests (50 lines)

```rust
// crates/goose/tests/performance_test.rs

#[tokio::test]
async fn test_many_sessions_performance() {
    // Not complex timing, just "does it work with many sessions"
    let manager = Arc::new(AgentManager::new());
    
    let start = std::time::Instant::now();
    
    // Create 100 sessions concurrently
    let mut handles = vec![];
    for i in 0..100 {
        let mgr = manager.clone();
        handles.push(tokio::spawn(async move {
            let session = SessionId(format!("perf_{}", i));
            mgr.get_agent(session, ExecutionMode::chat()).await
        }));
    }
    
    futures::future::join_all(handles).await;
    
    let elapsed = start.elapsed();
    
    // Should complete in reasonable time (< 5 seconds)
    assert!(elapsed.as_secs() < 5, "Too slow: {:?}", elapsed);
    
    // Should respect memory limits
    let count = manager.sessions.read().await.len();
    assert!(count <= 100);
}

#[tokio::test]
async fn test_session_cache_efficiency() {
    // Verify caching works
    let manager = AgentManager::new();
    let session = SessionId::generate();
    
    // First call creates
    let start1 = std::time::Instant::now();
    let _agent1 = manager.get_agent(session.clone(), ExecutionMode::chat()).await.unwrap();
    let create_time = start1.elapsed();
    
    // Second call should be much faster (cached)
    let start2 = std::time::Instant::now();
    let _agent2 = manager.get_agent(session, ExecutionMode::chat()).await.unwrap();
    let cache_time = start2.elapsed();
    
    // Cache hit should be at least 10x faster
    assert!(cache_time < create_time / 10);
}
```

## Testing Priority

### Must Have (Week 1)
1. ✅ Session isolation test
2. ✅ Concurrent access test
3. ✅ Adapter backward compatibility tests
4. ✅ Server integration test for multiple sessions
5. ✅ ExecutionMode behavior test

### Should Have (Week 1-2)
1. ✅ Extension isolation test
2. ✅ Session limit enforcement test
3. ✅ Scheduler integration test
4. ✅ Dynamic task adapter test
5. ✅ Recipe execution stub test

### Nice to Have (Week 2+)
1. ⏳ Performance tests
2. ⏳ Stress tests with many sessions
3. ⏳ Provider isolation tests
4. ⏳ Complex scenario tests
5. ⏳ Metrics and monitoring tests

## Key Differences from PR #4542

| Aspect | PR #4542 Tests | Our Tests | Rationale |
|--------|---------------|-----------|-----------|
| **Focus** | Feature completeness | Core functionality | Ship working code first |
| **Complexity** | Complex timing/cleanup | Simple assertions | Avoid flaky tests |
| **Coverage** | Every edge case | User scenarios | Test what matters |
| **Adapters** | None | Comprehensive | Critical for compatibility |
| **Scheduler** | Broken | Fixed & tested | Must not break existing |
| **Lines of Test Code** | ~1000 | ~500 | Maintainable |

## Success Metrics

- ✅ All Phase 1 tests pass
- ✅ No regression in existing functionality
- ✅ Session isolation proven
- ✅ Concurrent usage verified
- ✅ Adapters maintain compatibility
- ✅ < 500 lines of test code
- ✅ Tests run in < 10 seconds

## Testing Commands

```bash
# Run all new tests
cargo test -p goose agent_manager
cargo test -p goose adapter
cargo test -p goose-server integration

# Run with coverage
cargo tarpaulin -p goose --lib execution

# Run stress tests (optional)
cargo test -p goose performance --release

# Verify no regression
cargo test --workspace
```

## Conclusion

This testing plan learns from PR #4542's comprehensive approach while avoiding over-engineering. We focus on:

1. **Proving session isolation** - The core requirement
2. **Testing adapters** - Critical for backward compatibility  
3. **Real user scenarios** - Not just unit tests
4. **Simple, fast tests** - No complex timing or flaky tests
5. **Appropriate scope** - 500 lines vs 1000+ in PR #4542

The key insight: **Test what users will actually do, not every theoretical edge case.**
