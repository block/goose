# PR #4542 Review: Initial POC of Agent Manager

## Summary
This PR attempts to solve the critical shared agent problem in goosed/goose-server by introducing an AgentManager that provides per-session agent isolation. While the core concept is correct and necessary, the implementation has significant issues that need to be addressed.

## ✅ What Works Well

### Core Architecture
- **Correct approach**: Using AgentManager to map sessions to agents is the right solution
- **Session isolation**: Tests prove that extensions, providers, and state are properly isolated
- **Backward compatibility**: Session IDs are made optional in some places to maintain compatibility
- **Comprehensive tests**: Good test coverage for the new functionality

### Implementation Strengths
- Thread-safe with async RwLock
- Metrics tracking for observability
- Clean separation of concerns in manager.rs
- Proper Arc usage for shared ownership

## ❌ Critical Issues

### 1. Scheduler Integration Broken
The PR removes scheduler integration without replacement:
```rust
// TODO: Once we have per-session agents, each agent will need scheduler access
```
Every agent needs access to the scheduler for the `platform__manage_schedule` tool to work. This is a regression.

**Fix Required**: Pass scheduler to each agent during creation in `create_agent_for_session`.

### 2. Provider Initialization Flaw
AgentManager hardcodes provider initialization from environment variables:
```rust
let provider_name = config.get_param::<String>("GOOSE_PROVIDER")?;
```
This defeats the purpose of per-session configuration. Sessions should be able to have different providers.

**Fix Required**: Remove automatic provider initialization, let routes set providers as needed.

### 3. Missing Agent Limit Enforcement
Config defines `max_agents: 100` but never enforces it. This could lead to unbounded memory growth.

**Fix Required**: Check limit in `get_agent` before creating new agents.

## ⚠️ Design Issues

### 4. Over-Engineering
The PR adds unnecessary complexity:
- **Unused enums**: ExecutionMode, ApprovalMode, InheritConfig, SessionState
- **Placeholder code**: AgentPool with PhantomData
- **Unused dependency**: LRU crate added but never used

**Recommendation**: Remove all unused code. Keep it minimal.

### 5. Inconsistent Error Handling
Mix of error handling approaches across routes:
- Some use simple `StatusCode::INTERNAL_SERVER_ERROR`
- Others provide detailed error messages
- Inconsistent session_id extraction

**Recommendation**: Standardize error handling with consistent patterns.

### 6. Session Creation Logic Duplicated
`reply_handler` creates session metadata inline instead of using existing session storage patterns.

**Recommendation**: Use session::storage functions consistently.

## 🔧 Minor Issues

### 7. Double-Checked Locking
The pattern used could cause unnecessary lock upgrades:
```rust
{
    let agents = self.agents.read().await;
    // check
}
let mut agents = self.agents.write().await;
// double-check
```

**Consider**: Using `parking_lot::RwLock` with `upgradable_read()` or accepting the race condition.

### 8. Metrics Not Actionable
Metrics are tracked but there's no way to use them for automatic cleanup or alerts.

**Consider**: Add periodic cleanup task or expose metrics endpoint.

## 📊 Assessment Against Requirements

| Requirement | Status | Notes |
|------------|--------|-------|
| Agent per session | ✅ | Core functionality works |
| Session isolation | ✅ | Tests prove isolation |
| Backward compatibility | ⚠️ | Partially - some breaking changes |
| Minimal implementation | ❌ | Too much unused code |
| Performance | ⚠️ | No agent reuse, always creates new |
| Clean architecture | ⚠️ | Good structure but over-engineered |

## Verdict: **Needs Major Revision**

### Must Fix Before Merge:
1. Restore scheduler integration
2. Remove provider initialization from manager
3. Enforce max_agents limit
4. Remove all unused code (enums, AgentPool, LRU)
5. Standardize error handling

### Should Fix:
1. Improve lock patterns
2. Add periodic cleanup
3. Consistent session handling

### Nice to Have (Future):
1. Agent pooling for reuse
2. Metrics endpoint
3. Graceful shutdown

## Code Quality Score: 6/10

**Positives:**
- Solves the critical problem
- Good test coverage
- Correct core architecture

**Negatives:**
- Incomplete implementation (scheduler)
- Over-engineered with unused code
- Inconsistent patterns
- Breaking changes not fully handled

## Recommendation

This PR should **not be merged as-is**. While it solves the core problem, it introduces regressions (scheduler) and adds unnecessary complexity. With the fixes outlined above, this could become a solid foundation for multi-session support.

The author should:
1. Fix critical issues (1-3)
2. Remove unused code (4)
3. Clean up patterns (5-6)
4. Then re-submit for review

The core idea is sound, but the execution needs refinement to meet production standards.
