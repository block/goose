# Alternative Agent Manager Implementation Plan

## Core Concept: Lazy Agent Creation with Configuration Inheritance

Instead of creating agents eagerly, we create them lazily and apply configuration from the session context. This approach minimizes memory usage and simplifies the implementation.

## Key Insight

The current code always follows this pattern:
1. Create empty agent
2. Configure provider
3. Add extensions  
4. Set scheduler (if needed)

We can encapsulate this pattern in the AgentManager.

## Implementation Strategy

### Approach: Session-Aware Agent Factory

Instead of a HashMap cache, use a factory pattern that creates configured agents on demand.

```rust
pub struct AgentManager {
    scheduler: Option<Arc<dyn SchedulerTrait>>,
    default_provider_factory: Option<Box<dyn Fn() -> Result<Arc<dyn Provider>>>>,
}

impl AgentManager {
    pub async fn create_agent_for_session(
        &self,
        session_id: session::Identifier,
    ) -> Result<Arc<Agent>> {
        // Always create fresh agent
        let agent = Arc::new(Agent::new());
        
        // Set scheduler if available
        if let Some(scheduler) = &self.scheduler {
            agent.set_scheduler(Arc::clone(scheduler)).await;
        }
        
        // Load session metadata to get configuration
        if let Ok(path) = session::storage::get_path(session_id) {
            if let Ok(metadata) = session::storage::read_metadata(&path) {
                // Apply session-specific configuration
                self.configure_from_metadata(&agent, &metadata).await?;
            }
        }
        
        Ok(agent)
    }
}
```

### Key Differences from Original Plan

1. **No Caching** - Always create fresh agents
   - Pros: No memory management, no cleanup needed, simpler
   - Cons: Slight performance hit on agent creation

2. **Session-Driven Configuration** - Load config from session metadata
   - Pros: Agents always match session state
   - Cons: Requires session metadata to exist

3. **Factory Pattern** - Encapsulate creation logic
   - Pros: Clean separation of concerns
   - Cons: Less flexible than direct cache access

## Detailed Implementation

### 1. Minimal AgentManager (50 lines)
```rust
use std::sync::Arc;
use crate::agents::Agent;
use crate::session;
use crate::scheduler_trait::SchedulerTrait;

pub struct AgentManager {
    scheduler: Option<Arc<dyn SchedulerTrait>>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self { scheduler: None }
    }
    
    pub fn with_scheduler(scheduler: Arc<dyn SchedulerTrait>) -> Self {
        Self { 
            scheduler: Some(scheduler)
        }
    }
    
    pub async fn get_or_create_agent(
        &self,
        session_id: session::Identifier,
    ) -> Result<Arc<Agent>, anyhow::Error> {
        // Always create new agent (no caching)
        let agent = Arc::new(Agent::new());
        
        // Apply scheduler if available
        if let Some(ref scheduler) = self.scheduler {
            agent.set_scheduler(Arc::clone(scheduler)).await;
        }
        
        // Future: Load session-specific config here
        // For now, just return the agent
        
        Ok(agent)
    }
}
```

### 2. Ultra-Minimal AppState Changes
```rust
pub struct AppState {
    agent_manager: AgentManager,  // Not Arc, embedded directly
    // ... rest unchanged
}

impl AppState {
    pub async fn new(secret_key: String) -> Arc<AppState> {
        Arc::new(Self {
            agent_manager: AgentManager::new(),
            secret_key,
            // ... rest
        })
    }
    
    pub async fn set_scheduler(&self, scheduler: Arc<dyn SchedulerTrait>) {
        // Store in both places for now
        self.scheduler.write().await.replace(scheduler.clone());
        // Future: set on agent_manager
    }
    
    pub async fn get_agent(&self, session_id: session::Identifier) -> Result<Arc<Agent>> {
        self.agent_manager.get_or_create_agent(session_id).await
    }
}
```

### 3. Route Pattern - Even Simpler
```rust
// Generate session ID if missing
let session_id = request.session_id
    .map(|s| session::Identifier::Name(s))
    .unwrap_or_else(|| session::Identifier::Name(session::generate_session_id()));

// Get agent (created fresh each time for now)
let agent = state.get_agent(session_id).await?;
```

## Comparison with HashMap Approach

| Aspect | HashMap Cache | Factory Pattern |
|--------|--------------|-----------------|
| Lines of Code | ~100 | ~50 |
| Memory Usage | Grows over time | Minimal |
| Performance | Fast after first access | Consistent (slower) |
| Complexity | Medium | Low |
| Cleanup Needed | Yes | No |
| State Management | Complex | Simple |

## Migration Path

### Phase 1: No Cache (This PR)
- Fresh agent per request
- Simple, works, proves isolation
- ~50 lines of code

### Phase 2: Smart Caching (Future)
```rust
struct AgentManager {
    recent: Arc<RwLock<Option<(session::Identifier, Arc<Agent>)>>>,
    scheduler: Option<Arc<dyn SchedulerTrait>>,
}
```
- Cache only the most recent agent
- 90% of requests are sequential from same session
- Still simple, much better performance

### Phase 3: Full Cache (If Needed)
- Add HashMap back
- Add LRU eviction
- Only if performance requires it

## Alternative: Hybrid Approach

Combine both strategies:

```rust
pub struct AgentManager {
    // Cache for active sessions only
    active: Arc<RwLock<HashMap<session::Identifier, Arc<Agent>>>>,
    // Factory for creating new agents
    scheduler: Option<Arc<dyn SchedulerTrait>>,
    // Simple limit
    max_active: usize,
}

impl AgentManager {
    pub async fn get_agent(&self, session_id: session::Identifier) -> Result<Arc<Agent>> {
        // Try cache first
        if let Some(agent) = self.active.read().await.get(&session_id) {
            return Ok(Arc::clone(agent));
        }
        
        // Create new
        let agent = Arc::new(Agent::new());
        if let Some(ref sched) = self.scheduler {
            agent.set_scheduler(Arc::clone(sched)).await;
        }
        
        // Cache if under limit
        let mut active = self.active.write().await;
        if active.len() < self.max_active {
            active.insert(session_id, Arc::clone(&agent));
        }
        
        Ok(agent)
    }
    
    pub async fn end_session(&self, session_id: &session::Identifier) {
        self.active.write().await.remove(session_id);
    }
}
```

## Benefits of Alternative Approach

1. **Simpler** - No complex state management
2. **Cleaner** - Clear separation of concerns
3. **Safer** - No memory leaks possible
4. **Testable** - Easy to unit test
5. **Extensible** - Easy to add caching later

## Drawbacks

1. **Performance** - Creates agent on each request (mitigated by caching in phase 2)
2. **No Persistence** - Agents don't persist across requests (may be a feature)

## Code Size Comparison

| Component | Original PR | HashMap Plan | Factory Plan | Hybrid |
|-----------|------------|--------------|--------------|--------|
| manager.rs | 403 lines | 100 lines | 50 lines | 80 lines |
| state.rs changes | 30 lines | 20 lines | 15 lines | 20 lines |
| route changes | ~200 lines | ~200 lines | ~200 lines | ~200 lines |
| **Total** | 633 lines | 320 lines | 265 lines | 300 lines |

## Recommendation

Start with the **Factory Pattern** (Phase 1) because:
1. Simplest possible implementation
2. Proves session isolation works
3. No memory management needed
4. Easy to add caching later if needed
5. Half the code of HashMap approach

Then measure performance and add caching only if needed.

## Example Complete Implementation

```rust
// crates/goose/src/agents/manager.rs (entire file)
use std::sync::Arc;
use anyhow::Result;
use crate::agents::Agent;
use crate::session;
use crate::scheduler_trait::SchedulerTrait;

/// Manages agent creation for sessions
pub struct AgentManager {
    scheduler: Option<Arc<dyn SchedulerTrait>>,
}

impl AgentManager {
    /// Create manager without scheduler
    pub fn new() -> Self {
        Self { scheduler: None }
    }
    
    /// Set the scheduler for all agents
    pub fn set_scheduler(&mut self, scheduler: Arc<dyn SchedulerTrait>) {
        self.scheduler = Some(scheduler);
    }
    
    /// Get or create an agent for a session
    pub async fn get_agent(
        &self,
        _session_id: session::Identifier,  // Reserved for future use
    ) -> Result<Arc<Agent>> {
        let agent = Arc::new(Agent::new());
        
        if let Some(ref scheduler) = self.scheduler {
            agent.set_scheduler(Arc::clone(scheduler)).await;
        }
        
        Ok(agent)
    }
}
```

**Total: 35 lines of actual code**

This is the absolute minimum that solves the problem.
