# Unified Execution Implementation Plan

## Vision: Single Execution Pipeline for All Tasks

Based on issue #4389 and UNIFICATION_REPORT.md, this plan creates a foundation for unifying all execution paths in Goose.

## Core Architecture

### The Unified Model

```
Every task is a Recipe → Every Recipe runs in a Session → Every Session has an Agent
```

## Implementation Strategy: Progressive Enhancement

### Phase 1: Foundation Layer (This PR - 2 weeks)

Create the core infrastructure that enables future unification without breaking existing code.

#### 1.1 Core Types (50 lines)

```rust
// crates/goose/src/execution/mod.rs
pub mod manager;
pub mod session;
pub mod recipe;

use serde::{Serialize, Deserialize};

/// How a task should be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Interactive chat with user
    Interactive {
        streaming: bool,
        confirmations: bool,
    },
    /// Background scheduled or one-off task
    Background {
        scheduled_id: Option<String>,
        retry_config: Option<RetryConfig>,
    },
    /// Subtask of another agent
    SubTask {
        parent_session: SessionId,
        inherit_extensions: bool,
        approval_mode: ApprovalMode,
    },
}

/// How subtask approvals are handled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalMode {
    Autonomous,      // Handle locally
    BubbleToParent,  // Ask parent
    Filtered(Vec<String>), // Selective
}

/// Represents any executable task
#[derive(Debug, Clone)]
pub enum TaskSource {
    Recipe(Recipe),           // Full recipe
    Text(String),            // Dynamic task
    Reference(String),       // Sub-recipe ID
    Prompt(String),         // Simple prompt
}
```

#### 1.2 AgentManager with Execution Pipeline (150 lines)

```rust
// crates/goose/src/execution/manager.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AgentManager {
    /// Active sessions and their agents
    sessions: Arc<RwLock<HashMap<SessionId, SessionContext>>>,
    /// Scheduler for platform tools
    scheduler: Arc<RwLock<Option<Arc<dyn SchedulerTrait>>>>,
    /// Configuration
    config: ManagerConfig,
    /// Metrics
    metrics: Arc<RwLock<ManagerMetrics>>,
}

pub struct SessionContext {
    agent: Arc<Agent>,
    session_id: SessionId,
    mode: ExecutionMode,
    created_at: DateTime<Utc>,
    last_used: DateTime<Utc>,
    parent: Option<SessionId>,
    state: SessionState,
}

#[derive(Debug)]
pub enum SessionState {
    Active,
    Executing(String), // Task ID
    Idle,
    Completed,
}

impl AgentManager {
    /// Get or create an agent for a session
    pub async fn get_agent(
        &self,
        session_id: SessionId,
        mode: ExecutionMode,
    ) -> Result<Arc<Agent>> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(ctx) = sessions.get_mut(&session_id) {
            ctx.last_used = Utc::now();
            self.metrics.write().await.cache_hits += 1;
            return Ok(Arc::clone(&ctx.agent));
        }
        
        // Create new agent with proper configuration
        let agent = self.create_configured_agent(&mode).await?;
        
        let context = SessionContext {
            agent: Arc::clone(&agent),
            session_id: session_id.clone(),
            mode,
            created_at: Utc::now(),
            last_used: Utc::now(),
            parent: None,
            state: SessionState::Active,
        };
        
        sessions.insert(session_id, context);
        self.metrics.write().await.agents_created += 1;
        
        Ok(agent)
    }
    
    /// Execute any task in a session context (future-ready)
    pub async fn execute(
        &self,
        session_id: SessionId,
        source: TaskSource,
        mode: ExecutionMode,
    ) -> Result<ExecutionResult> {
        // Convert to recipe
        let recipe = self.task_to_recipe(source).await?;
        
        // Get or create agent
        let agent = self.get_agent(session_id.clone(), mode.clone()).await?;
        
        // Update session state
        self.update_session_state(session_id.clone(), SessionState::Executing(recipe.id())).await;
        
        // Execute based on mode
        let result = match mode {
            ExecutionMode::Interactive { streaming, .. } => {
                self.execute_interactive(agent, recipe, streaming).await?
            }
            ExecutionMode::Background { .. } => {
                self.execute_background(agent, recipe).await?
            }
            ExecutionMode::SubTask { parent_session, .. } => {
                self.execute_subtask(agent, recipe, parent_session).await?
            }
        };
        
        // Update state
        self.update_session_state(session_id, SessionState::Completed).await;
        
        Ok(result)
    }
    
    /// Convert any task source to a recipe
    async fn task_to_recipe(&self, source: TaskSource) -> Result<Recipe> {
        match source {
            TaskSource::Recipe(r) => Ok(r),
            TaskSource::Text(text) => Ok(Recipe::from_text(text)),
            TaskSource::Reference(id) => self.load_recipe(id).await,
            TaskSource::Prompt(prompt) => Ok(Recipe::from_prompt(prompt)),
        }
    }
}
```

#### 1.3 Backward Compatible Routes (100 lines)

```rust
// crates/goose-server/src/routes/reply.rs
async fn reply_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<impl IntoResponse> {
    let session_id = request.session_id
        .map(SessionId::from)
        .unwrap_or_else(|| SessionId::generate());
    
    // Use new execution pipeline
    let mode = ExecutionMode::Interactive {
        streaming: true,
        confirmations: true,
    };
    
    let source = TaskSource::Prompt(request.messages.last()?.text());
    
    // This now goes through unified pipeline
    let result = state.agent_manager
        .execute(session_id, source, mode)
        .await?;
    
    // Stream results back
    Ok(result.into_response())
}
```

### Phase 2: Recipe Unification (Next PR - 1 week)

#### 2.1 Recipe Trait System

```rust
pub trait IntoRecipe {
    fn into_recipe(self) -> Result<Recipe>;
}

impl IntoRecipe for String {
    fn into_recipe(self) -> Result<Recipe> {
        Ok(Recipe {
            instructions: Some(self),
            ..Default::default()
        })
    }
}

impl IntoRecipe for SubRecipe {
    fn into_recipe(self) -> Result<Recipe> {
        // Convert sub-recipe to full recipe
    }
}

impl IntoRecipe for ScheduledJob {
    fn into_recipe(self) -> Result<Recipe> {
        Recipe::from_file(self.source)
    }
}
```

#### 2.2 Unified Task Execution

```rust
impl AgentManager {
    /// All dynamic tasks go through here
    pub async fn create_dynamic_task(
        &self,
        parent_session: SessionId,
        instructions: String,
        extensions: Vec<String>,
    ) -> Result<String> {
        let task_id = generate_task_id();
        let session_id = SessionId::from(task_id.clone());
        
        let mode = ExecutionMode::SubTask {
            parent_session,
            inherit_extensions: true,
            approval_mode: ApprovalMode::Autonomous,
        };
        
        let source = TaskSource::Text(instructions);
        
        // Execute through unified pipeline
        self.execute(session_id, source, mode).await?;
        
        Ok(task_id)
    }
}
```

### Phase 3: Scheduler Integration (Week 3)

```rust
impl Scheduler {
    async fn run_job(&self, job: ScheduledJob) -> Result<()> {
        let session_id = SessionId::from(job.id.clone());
        
        let mode = ExecutionMode::Background {
            scheduled_id: Some(job.id),
            retry_config: job.retry_config,
        };
        
        let source = TaskSource::Reference(job.recipe_id);
        
        // Scheduler now uses unified pipeline
        self.agent_manager.execute(session_id, source, mode).await?;
        
        Ok(())
    }
}
```

## Benefits of This Approach

### 1. Progressive Enhancement
- Start with foundation, add features incrementally
- Each phase delivers value
- No big-bang migration

### 2. Backward Compatible
- Existing APIs continue to work
- Adapter layers for smooth transition
- No breaking changes

### 3. Future Ready
- ExecutionMode enables all future scenarios
- TaskSource abstracts all input types
- Recipe becomes universal representation

### 4. Clean Architecture
- Clear separation of concerns
- Single execution pipeline
- Consistent error handling

## Implementation Metrics

### Phase 1 Deliverables
- [ ] Core types defined (50 lines)
- [ ] AgentManager with execute method (150 lines)
- [ ] Route adapters (100 lines)
- [ ] Tests for session isolation (200 lines)
- **Total: ~500 lines**

### Success Criteria
- All existing tests pass
- Session isolation proven
- No performance regression
- ExecutionMode properly propagated

## Comparison with PR #4542

| Aspect | PR #4542 | This Plan |
|--------|----------|-----------|
| Lines of Code | 1,897 | 500 |
| Execution Pipeline | No | Yes |
| Recipe Framework | No | Yes (Phase 2) |
| ExecutionMode | Defined but unused | Actively used |
| Backward Compatible | Partial | Full |
| Scheduler Fix | Broken | Properly integrated |

## Risk Mitigation

### Technical Risks
1. **Performance** - Mitigated by caching and lazy loading
2. **Complexity** - Phased approach reduces complexity
3. **Migration** - Adapter layers ensure smooth transition

### Process Risks
1. **Scope Creep** - Clear phase boundaries
2. **Testing** - Comprehensive test suite per phase
3. **Documentation** - Update as we go

## Next Steps

1. **Week 1**: Implement Phase 1 foundation
2. **Week 2**: Test and refine
3. **Week 3**: Begin Phase 2 (recipe unification)
4. **Week 4**: Complete integration

This plan provides a clear path from the current state to the unified execution model envisioned in issue #4389, while remaining practical and implementable.
