# Agent Manager Review Log

## Session Start: Initial Analysis

### PR Overview
- PR #4542: "Initial POC of Agent Manager"
- Author: tlongwell-block
- Status: OPEN (created Sep 5, 2025)
- Description: "Rough POC. Do not review/merge"
- Changes: +1897 lines, -74 lines

### Issue #4389 Summary
The discussion outlines the need for an Agent Manager to solve:
1. **Multiple parallel execution paths** - Interactive chat, scheduler, dynamic tasks, sub-recipes all handle agents differently
2. **Code duplication** - Extension/provider setup logic re-implemented in multiple places
3. **Session interference** - Shared Agent in server means sessions interfere with each other
4. **Inconsistent behavior** - Different execution surfaces behave differently

**Goals:**
- Agent per session in goosed/goose-server with isolation
- Unify execution for recipes, dynamic tasks, and scheduled jobs
- Allow agents to create ad-hoc dynamic tasks with clear extension scoping
- Keep existing tools usable but route through same backend

### Files Modified in PR
Key files:
- `crates/goose/src/agents/manager.rs` - NEW (+403 lines) - The core Agent Manager
- `crates/goose-server/src/routes/*.rs` - Multiple route files updated
- `crates/goose-server/src/state.rs` - State management changes
- Multiple test files added for agent manager

## Next Steps
1. Deep dive into current architecture
2. Analyze the new manager.rs implementation
3. Check how routes are modified
4. Verify test coverage

## Requirements Analysis Complete
Created AGENT_MANAGER_REQUIREMENTS.md based on:
- GitHub issue #4389 requirements
- Current codebase analysis showing Agent::new() usage in:
  - goose-cli (6 places): session builder, commands (acp, configure, web), scenario tests
  - goose core (10+ places): tests, scheduler
  - goose-server: Previously created single shared agent

Key insights:
- Current state has ONE shared agent for ALL sessions in server
- CLI creates new agents for each operation
- Scheduler creates fresh agents per run
- This PR aims to fix only goosed/goose-server, not CLI or scheduler

## Deep Dive for Alternative Implementation
Analyzing Agent usage patterns:
1. **goose-server**: Single shared agent in AppState, needs scheduler access
2. **goose-cli session builder**: Creates agent, configures provider, adds extensions
3. **scheduler**: Creates fresh agent per job, needs provider setup
4. **Pattern**: Agent::new() → update_provider() → add_extension() → set_scheduler()

Key observations:
- Agent is always created empty then configured
- Provider is always set before use
- Extensions are added after provider
- Scheduler is optional (only for server agents)

## Unified Execution Implementation Analysis

Created three implementation approaches:

### First Approach: Full Pipeline (500 lines)
- Complete ExecutionMode with all features
- Full recipe framework from start
- Comprehensive but complex

### Alternative: Adapter Pattern (250 lines)
- Minimal changes, maximum compatibility
- Adapters bridge old and new
- Natural evolution over design

### Final: Pragmatic Hybrid (400 lines)
- Right-sized ExecutionMode enum
- Smart primitives that can grow
- Adapters for compatibility
- Clear evolution path

**Winner: Pragmatic Hybrid**
- Balances immediate needs with future vision
- 75% less code than PR #4542
- Fixes scheduler integration
- Enables true unification path
