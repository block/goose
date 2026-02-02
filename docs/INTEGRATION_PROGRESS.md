# MCP Agents Integration Progress

## Status: Phase 2 Complete ✅

Last Updated: Jan 30, 2026

## Completed Tasks

### Phase 1 - Core Components
- [x] Clone 5 external repos (OpenHands, Aider, PydanticAI, PraisonAI, LangGraph)
- [x] Analyze each repo for MCP integration points
- [x] Create StateGraph minimal implementation
- [x] Create ApprovalPolicy trait + 3 presets (SAFE/PARANOID/AUTOPILOT)
- [x] Create test parsers (pytest/jest/cargo/go)
- [x] Create Done Gate implementation
- [x] Create ShellGuard for dangerous ops

### Phase 2 - Integration
- [x] Integrate ShellGuard into execute_shell_command in retry.rs
- [x] Add CLI flags for --approval-policy
- [x] Wire StateGraph into agent loop (runner module)
- [x] Add exports and re-exports for new modules
- [x] Fix compilation errors (Eq trait, Idle state, method names)

## Files Created

| File | Description |
|------|-------------|
| `crates/goose/src/agents/state_graph/mod.rs` | StateGraph engine |
| `crates/goose/src/agents/state_graph/state.rs` | State types |
| `crates/goose/src/agents/state_graph/runner.rs` | Runner with callbacks |
| `crates/goose/src/approval/mod.rs` | ApprovalPolicy trait |
| `crates/goose/src/approval/presets.rs` | 3 preset policies |
| `crates/goose/src/test_parsers/mod.rs` | Parser framework |
| `crates/goose/src/test_parsers/pytest.rs` | Pytest parser |
| `crates/goose/src/test_parsers/jest.rs` | Jest parser |
| `crates/goose/src/agents/done_gate.rs` | Verification checks |
| `crates/goose/src/agents/shell_guard.rs` | Command approval |
| `docs/mcp-sidecars-config.md` | MCP sidecar guide |

## Files Modified

| File | Changes |
|------|---------|
| `crates/goose/src/agents/mod.rs` | Added done_gate, state_graph, shell_guard |
| `crates/goose/src/lib.rs` | Added approval, test_parsers modules |
| `crates/goose/src/agents/retry.rs` | Added execute_shell_command_guarded |
| `crates/goose-cli/src/cli.rs` | Added --approval-policy flag |
| `crates/goose-cli/src/session/builder.rs` | Added approval_policy field |

## Build Status

```
cargo check --package goose  ✅ PASSES (5 warnings)
cargo fmt --package goose    ✅ FORMATTED
```

## Phase 3 - TODO

- [ ] Wire approval_policy from CLI through agent configuration
- [ ] Add integration tests for StateGraph + DoneGate flow  
- [ ] Test MCP sidecars (Playwright, OpenHands, Aider)
- [ ] Add environment detection (Docker vs real filesystem)
- [ ] Connect ShellGuard to tool execution in agent loop

## CLI Usage

```bash
# Default safe mode
goose run --text "build the project"

# Paranoid mode - prompt for all commands
goose run --approval-policy paranoid --text "deploy to prod"

# Autopilot - only in Docker sandbox
goose run --approval-policy autopilot --text "run tests"
```

## External Repos

Located in `external/`:
- OpenHands
- aider
- pydantic-ai
- PraisonAI
- langgraph
