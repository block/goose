# Alternative Unified Execution Implementation Plan

## Vision: Minimal Viable Unification

Instead of building a complete execution pipeline upfront, this plan takes a **minimalist evolutionary approach** - solve today's problem while creating natural evolution points.

## Core Philosophy

**"Make the current system unified, don't build a new unified system"**

## Implementation Strategy: Adapter Pattern

### The Key Insight

We don't need to change how things execute. We need to change how they're created and managed. Use adapters to make existing execution paths look unified.

### Phase 1: Unified Creation Layer (This PR - 1 week)

#### 1.1 Simple Agent Factory (80 lines)

```rust
// crates/goose/src/agents/factory.rs
use std::sync::Arc;
use crate::agents::Agent;

/// Unified agent creation with context
pub struct AgentFactory {
    sessions: Arc<RwLock<HashMap<String, Arc<Agent>>>>,
    scheduler: Option<Arc<dyn SchedulerTrait>>,
}

/// Context for agent creation
pub struct CreationContext {
    pub session_id: String,
    pub execution_type: ExecutionType,
    pub parent_id: Option<String>,
}

/// Simple execution type marker
#[derive(Debug, Clone)]
pub enum ExecutionType {
    Chat,       // Interactive goose-server
    Task,       // Dynamic task
    Recipe,     // Sub-recipe
    Scheduled,  // Scheduler job
}

impl AgentFactory {
    /// Create or get agent with context
    pub async fn get_or_create(
        &self,
        context: CreationContext,
    ) -> Result<Arc<Agent>> {
        // Simple cache check
        if let Some(agent) = self.sessions.read().await.get(&context.session_id) {
            return Ok(Arc::clone(agent));
        }
        
        // Create with appropriate setup
        let agent = Arc::new(Agent::new());
        
        // Configure based on type
        match context.execution_type {
            ExecutionType::Chat | ExecutionType::Scheduled => {
                if let Some(sched) = &self.scheduler {
                    agent.set_scheduler(Arc::clone(sched)).await;
                }
            }
            ExecutionType::Task | ExecutionType::Recipe => {
                // Subtasks don't need scheduler
            }
        }
        
        // Cache if appropriate
        if matches!(context.execution_type, ExecutionType::Chat) {
            self.sessions.write().await.insert(
                context.session_id.clone(),
                Arc::clone(&agent),
            );
        }
        
        Ok(agent)
    }
}
```

#### 1.2 Minimal Adapters (50 lines each)

```rust
// crates/goose/src/agents/adapters.rs

/// Adapter for dynamic tasks
pub struct DynamicTaskAdapter {
    factory: Arc<AgentFactory>,
}

impl DynamicTaskAdapter {
    pub async fn create_task(
        &self,
        instructions: String,
        parent_session: String,
    ) -> Result<String> {
        let task_id = generate_id();
        
        let context = CreationContext {
            session_id: task_id.clone(),
            execution_type: ExecutionType::Task,
            parent_id: Some(parent_session),
        };
        
        let agent = self.factory.get_or_create(context).await?;
        
        // Use existing SubAgent logic
        let subagent = SubAgent::from_agent(agent);
        subagent.execute(instructions).await?;
        
        Ok(task_id)
    }
}

/// Adapter for scheduler
pub struct SchedulerAdapter {
    factory: Arc<AgentFactory>,
}

impl SchedulerAdapter {
    pub async fn run_job(&self, job: ScheduledJob) -> Result<()> {
        let context = CreationContext {
            session_id: job.id.clone(),
            execution_type: ExecutionType::Scheduled,
            parent_id: None,
        };
        
        let agent = self.factory.get_or_create(context).await?;
        
        // Use existing scheduler execution logic
        run_with_agent(agent, job).await
    }
}
```

#### 1.3 Incremental Migration (30 lines per endpoint)

```rust
// crates/goose-server/src/state.rs
pub struct AppState {
    agent_factory: Arc<AgentFactory>,  // Single factory
    // Keep everything else the same
}

impl AppState {
    pub async fn get_agent(&self, session_id: String) -> Result<Arc<Agent>> {
        let context = CreationContext {
            session_id,
            execution_type: ExecutionType::Chat,
            parent_id: None,
        };
        self.agent_factory.get_or_create(context).await
    }
}
```

### Phase 2: Gradual Convergence (Future PRs)

#### 2.1 Extract Common Patterns

As we use the adapters, we'll naturally see patterns:

```rust
// Patterns will emerge from usage
trait Executable {
    async fn execute(&self, agent: Arc<Agent>) -> Result<ExecutionResult>;
}

// Then we can gradually move to:
impl AgentFactory {
    pub async fn execute(
        &self,
        context: CreationContext,
        executable: impl Executable,
    ) -> Result<ExecutionResult> {
        let agent = self.get_or_create(context).await?;
        executable.execute(agent).await
    }
}
```

#### 2.2 Natural Evolution

The system will naturally evolve toward unification:

1. **Adapters reveal patterns** → Extract interfaces
2. **Interfaces stabilize** → Create unified execution
3. **Unified execution works** → Remove old code paths

## Benefits of Alternative Approach

### 1. Minimal Disruption
- Existing code keeps working
- No big rewrites
- Gradual migration

### 2. Immediate Value
- Session isolation fixed immediately
- Each adapter improves that subsystem
- No waiting for "grand unification"

### 3. Natural Evolution
- Patterns emerge from real usage
- No speculative abstractions
- Refactor based on evidence

### 4. Lower Risk
- Small, reviewable changes
- Easy to rollback
- Test each piece independently

## Implementation Comparison

| Component | First Plan | Alternative Plan |
|-----------|------------|------------------|
| Core Types | 50 lines (complex enums) | 20 lines (simple enums) |
| Manager/Factory | 150 lines | 80 lines |
| Adapters | Built into manager | 50 lines each |
| Route Changes | 100 lines | 30 lines |
| **Total Phase 1** | 500 lines | ~250 lines |

## Concrete Example: Dynamic Tasks

### Current Code (Unchanged)
```rust
// This stays exactly the same
let subagent = SubAgent::new(config);
let result = subagent.execute(instructions).await?;
```

### With Adapter (Minimal Change)
```rust
// Just change creation
let adapter = DynamicTaskAdapter::new(factory);
let task_id = adapter.create_task(instructions, parent).await?;
```

### Future Evolution (Natural)
```rust
// Eventually becomes
factory.execute(
    CreationContext::task(parent),
    TaskExecutable::from(instructions),
).await?
```

## Risk Analysis

### Lower Risk Than First Plan

1. **No Big Bang** - Each adapter is independent
2. **Proven Patterns** - Adapter pattern is well-understood
3. **Incremental** - Can stop at any phase
4. **Reversible** - Easy to back out changes

### Trade-offs

**Pros:**
- Simpler to implement
- Lower risk
- Immediate value
- Natural evolution

**Cons:**
- Less "designed"
- Multiple adapters initially
- Unification happens slower
- May need more refactoring later

## Migration Path

### Week 1: Foundation
1. Create AgentFactory (80 lines)
2. Add ExecutionType enum (20 lines)
3. Update AppState (20 lines)
4. Test session isolation

### Week 2: Adapters
1. DynamicTaskAdapter (50 lines)
2. SchedulerAdapter (50 lines)
3. SubRecipeAdapter (50 lines)
4. Test each adapter

### Week 3: Migration
1. Update routes to use factory
2. Update scheduler to use adapter
3. Update dynamic tasks to use adapter

### Week 4: Observation
1. Document patterns that emerged
2. Plan next evolution step
3. Remove any dead code

## Why This Approach is Better

### 1. Follows Agile Principles
- Working software over comprehensive documentation
- Responding to change over following a plan
- Individuals and interactions over processes and tools

### 2. Respects Existing Code
- Current code isn't "wrong", just not unified
- Adapters bridge the gap
- Evolution, not revolution

### 3. Delivers Value Immediately
- Session isolation: Day 1
- Better task management: Week 1
- Unified creation: Week 2
- Full unification: When ready

## Decision Framework

Choose this alternative if:
- ✅ You want minimal risk
- ✅ You prefer evolution over design
- ✅ You want to ship quickly
- ✅ You're okay with temporary adapters

Choose the first plan if:
- ❌ You want complete unification now
- ❌ You prefer designed architecture
- ❌ You're willing to take more risk
- ❌ You have more time

## Conclusion

This alternative takes a **"make it unified"** approach rather than **"build unified"**. By using simple adapters and natural evolution, we can achieve the same end goal with less risk, less code, and more immediate value.

The key insight: **Unification is a journey, not a destination**. Start with unified creation, let unified execution emerge naturally.
