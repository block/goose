# Improved Agent Manager Implementation Plan

Based on the review of PR #4542 and lessons learned from the POC.

## Core Principle: Minimal Viable Implementation

The goal is to solve ONLY the shared agent problem with the least possible changes.

## Implementation Strategy

### Phase 1: Minimal Agent Manager (This PR)

#### 1. Simplified AgentManager
```rust
pub struct AgentManager {
    agents: Arc<RwLock<HashMap<session::Identifier, Arc<Agent>>>>,
    scheduler: Arc<dyn SchedulerTrait>,
    max_agents: usize,
}

impl AgentManager {
    pub async fn get_agent(&self, session_id: session::Identifier) -> Result<Arc<Agent>> {
        // Check cache first
        if let Some(agent) = self.agents.read().await.get(&session_id) {
            return Ok(Arc::clone(agent));
        }
        
        // Check limit
        if self.agents.read().await.len() >= self.max_agents {
            self.cleanup_lru().await?;
        }
        
        // Create new agent
        let agent = Arc::new(Agent::new());
        agent.set_scheduler(Arc::clone(&self.scheduler)).await;
        
        self.agents.write().await.insert(session_id, Arc::clone(&agent));
        Ok(agent)
    }
    
    async fn cleanup_lru(&self) -> Result<()> {
        // Simple: remove oldest accessed agent
        // Future: use proper LRU
    }
}
```

**Key Points:**
- No unused enums or complex types
- No provider initialization (let routes handle it)
- Pass scheduler to every agent
- Simple limit enforcement
- ~100 lines instead of 400

#### 2. Minimal AppState Changes
```rust
pub struct AppState {
    agent_manager: Arc<AgentManager>,
    // ... rest unchanged
}

impl AppState {
    pub async fn new(secret_key: String) -> Arc<AppState> {
        let agent_manager = Arc::new(AgentManager::new());
        // ... rest unchanged
    }
    
    pub async fn get_agent(&self, session_id: session::Identifier) -> Result<Arc<Agent>> {
        self.agent_manager.get_agent(session_id).await
    }
}
```

#### 3. Route Updates - Backward Compatible
```rust
// In reply_handler
let session_id = request.session_id
    .unwrap_or_else(|| session::generate_session_id());

let agent = state.get_agent(session::Identifier::Name(session_id)).await?;
```

**Pattern for all routes:**
1. Extract session_id (generate if missing for compatibility)
2. Get agent for that session
3. Use agent normally

#### 4. No Extra Dependencies
- Remove LRU crate
- Use only existing dependencies
- Keep Cargo.lock changes minimal

### Phase 2: Future Enhancements (Separate PRs)

#### Agent Pooling (Later)
```rust
struct AgentPool {
    available: Vec<Arc<Agent>>,
    max_pool_size: usize,
}
```
- Reuse agents with same configuration
- Reset state between uses
- Reduce initialization overhead

#### Metrics & Monitoring (Later)
```rust
impl AgentManager {
    pub fn metrics_endpoint(&self) -> MetricsResponse {
        // Expose Prometheus-style metrics
    }
}
```

#### Advanced Features (Much Later)
- Approval bubbling for subtasks
- Provider inheritance
- Distributed session support
- Graceful shutdown with state persistence

## File Changes Summary

### New Files (1)
- `crates/goose/src/agents/manager.rs` (~100 lines)

### Modified Files (Minimal)
- `crates/goose/src/agents/mod.rs` - Add `pub mod manager`
- `crates/goose-server/src/state.rs` - Use AgentManager
- `crates/goose-server/src/routes/*.rs` - Add session_id extraction
- `crates/goose/src/session/storage.rs` - Add Hash derive for Identifier

### Test Files (Essential)
- `crates/goose/tests/agent_manager_test.rs` - Core functionality
- `crates/goose-server/tests/session_isolation_test.rs` - Prove isolation works

## Success Metrics

1. **Line Count**: < 500 lines added (currently 1897)
2. **Dependencies**: 0 new dependencies (currently adds LRU)
3. **Breaking Changes**: 0 (currently has several)
4. **Test Coverage**: > 90% of new code
5. **Performance**: < 1ms overhead for agent retrieval

## Implementation Checklist

### Must Have (This PR)
- [x] Simple HashMap-based AgentManager
- [x] Session to Agent mapping
- [x] Scheduler passed to each agent
- [x] Backward compatible routes
- [x] Tests proving isolation
- [ ] No unused code
- [ ] No new dependencies

### Nice to Have (Future)
- [ ] Agent pooling
- [ ] LRU eviction
- [ ] Metrics endpoint
- [ ] Graceful shutdown
- [ ] Provider inheritance
- [ ] Approval bubbling

## Key Lessons from PR #4542

### What to Keep
1. Core AgentManager concept
2. Session-based isolation
3. Test structure
4. Arc<Agent> pattern

### What to Remove
1. All unused enums (ExecutionMode, ApprovalMode, etc.)
2. AgentPool placeholder
3. Provider initialization in manager
4. LRU dependency
5. Complex error types
6. Overly detailed metrics

### What to Fix
1. Scheduler integration
2. Agent limit enforcement
3. Error handling consistency
4. Session ID handling

## Example: Simplified manager.rs

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::agents::Agent;
use crate::session;
use crate::scheduler_trait::SchedulerTrait;

pub struct AgentManager {
    agents: Arc<RwLock<HashMap<session::Identifier, Arc<Agent>>>>,
    scheduler: Arc<dyn SchedulerTrait>,
    max_agents: usize,
}

impl AgentManager {
    pub fn new(scheduler: Arc<dyn SchedulerTrait>, max_agents: usize) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            scheduler,
            max_agents,
        }
    }

    pub async fn get_agent(&self, session_id: session::Identifier) -> Result<Arc<Agent>> {
        // Try cache first
        {
            let agents = self.agents.read().await;
            if let Some(agent) = agents.get(&session_id) {
                return Ok(Arc::clone(agent));
            }
        }

        // Create new agent
        let mut agents = self.agents.write().await;
        
        // Check again (double-check pattern)
        if let Some(agent) = agents.get(&session_id) {
            return Ok(Arc::clone(agent));
        }

        // Enforce limit
        if agents.len() >= self.max_agents {
            // Remove random agent (simple for now)
            if let Some(key) = agents.keys().next().cloned() {
                agents.remove(&key);
            }
        }

        // Create and store
        let agent = Arc::new(Agent::new());
        agent.set_scheduler(Arc::clone(&self.scheduler)).await;
        agents.insert(session_id.clone(), Arc::clone(&agent));
        
        Ok(agent)
    }

    pub async fn remove_agent(&self, session_id: &session::Identifier) -> bool {
        self.agents.write().await.remove(session_id).is_some()
    }

    pub async fn agent_count(&self) -> usize {
        self.agents.read().await.len()
    }
}
```

**Total: ~70 lines of actual implementation**

## Conclusion

The current PR tries to do too much at once. By focusing ONLY on the core problem (shared agent causing session interference) and implementing the minimal solution, we can:

1. Ship faster
2. Reduce bugs
3. Maintain backward compatibility
4. Build a foundation for future enhancements

The improved implementation would be:
- **5x smaller** (500 vs 1897 lines)
- **No new dependencies**
- **Fully backward compatible**
- **Easier to review and maintain**
- **Still solves the core problem completely**

This is what "minimal viable implementation" looks like.
