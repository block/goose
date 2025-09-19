# Agent Manager Requirements

Based on GitHub Issue #4389 and deep analysis of the Goose codebase.

## Problem Statement

Currently, Goose has several parallel ways to run agents:
1. **Interactive chat in goosed/goose-server** - Uses a single shared Agent for ALL sessions
2. **Scheduler (legacy and Temporal)** - Spins up fresh Agent per run
3. **Dynamic tasks** - Create subagents on demand
4. **Sub-recipes** - Execute by spawning CLI

This creates:
- **Duplicated code paths** - Extension/provider setup logic re-implemented in multiple places
- **Inconsistent behavior** - Different execution surfaces behave differently
- **Concurrency issues** - Shared Agent means sessions interfere with each other
- **Hard-to-debug issues** - Shared ExtensionManager, tool monitor, channels

## Core Requirements

### 1. Agent Per Session (CRITICAL)
- Each session MUST have its own dedicated Agent instance
- No sharing of Agent state between sessions
- Complete isolation of:
  - Extensions and their state
  - Providers and model configurations
  - Tool approvals and permissions
  - System prompts and context
  - Conversation history

### 2. Unified Execution Model
- Single execution pipeline for chat, scheduler, and recipes
- Consistent behavior across all execution surfaces
- Common agent creation and initialization logic

### 3. Session Management
- Map session IDs to Agent instances
- Cache agents for session continuity
- Clean up idle agents to manage memory
- Support session persistence and resumption

### 4. Backward Compatibility
- Existing tools must continue to work (dynamic_task, subagent_execute_task, scheduler tools)
- API endpoints must maintain compatibility (with session_id additions)
- CLI operations should continue to function

### 5. Performance & Scalability
- Efficient agent caching (cache hits for same session)
- Configurable cleanup of idle agents
- Metrics for monitoring (agents created, cleaned, cache hits/misses)
- Support for many simultaneous sessions

## Implementation Requirements

### AgentManager Class
- Central component managing agent lifecycle
- HashMap<session::Identifier, Agent> for session mapping
- Thread-safe with async locks (RwLock)
- Metrics tracking for observability

### Key Methods
- `get_agent(session_id)` - Get or create agent for session
- `cleanup_idle(duration)` - Remove agents idle longer than duration
- `get_metrics()` - Return performance metrics
- `remove_agent(session_id)` - Explicitly remove an agent

### AppState Changes
- Replace single `agent: Arc<Agent>` with `agent_manager: Arc<AgentManager>`
- Update all route handlers to get agent by session_id
- Maintain scheduler and other shared state

### Route Updates
All routes that access the agent must:
1. Extract session_id from request
2. Call `state.get_agent(session_id)` 
3. Handle potential errors

Affected routes:
- `/agent/start` - No longer resets global agent
- `/agent/tools` - Get tools for specific session
- `/agent/update_provider` - Update provider for session
- `/extensions/add` - Add extension to session's agent
- `/extensions/remove` - Remove extension from session's agent
- `/chat` - Use session's agent for replies
- `/agent/session_config` - Configure session's agent
- `/context/manage` - Manage context for session

### Session Identifier
- Use existing `session::Identifier` enum
- Support both Name and Path variants
- Must be hashable for HashMap key

### Error Handling
- New `AgentError` enum for manager-specific errors
- Graceful handling of missing sessions
- Clear error messages for debugging

## Non-Goals (Out of Scope for Initial PR)

1. **Agent pooling** - Future optimization, not needed initially
2. **Recipe/scheduler migration** - Keep using existing patterns for now
3. **Dynamic task changes** - Continue spawning as before
4. **Approval bubbling** - Future enhancement for parent-child agents
5. **Provider inheritance** - Future feature for subtasks

## Success Criteria

1. **Isolation Verified** - Tests prove sessions don't interfere
2. **Backward Compatible** - Existing functionality continues to work
3. **Performance Acceptable** - Metrics show efficient caching
4. **Clean Architecture** - Clear separation of concerns
5. **Well Tested** - Comprehensive test coverage for:
   - Session isolation
   - Extension isolation
   - Provider isolation
   - Concurrent access
   - Cleanup behavior

## Minimal Implementation Checklist

For the initial PR to be minimal yet complete:
- [ ] AgentManager with basic get/cleanup operations
- [ ] AppState using AgentManager instead of single Agent
- [ ] Route handlers updated to use session-based agents
- [ ] Tests proving isolation works
- [ ] Metrics for monitoring
- [ ] No breaking changes to external APIs
- [ ] Documentation of the new architecture
