# Code Review: `feature/cli-via-goosed` Branch

**Branch:** `feature/cli-via-goosed`
**Commits:** 108 ahead of `main`
**Scope:** 155 files changed, +29,170 / -5,759 lines
**Reviewed:** 2025-01-XX
**Status:** Reviewed and partially remediated

---

## Executive Summary

This branch implements the CLI-via-goosed architecture shift, ACP (Agent Communication Protocol)
v0.2.0 compatibility, multi-agent orchestration, and significant UI improvements. The code
compiles cleanly, passes clippy with zero warnings, and all 977 tests pass.

**8 issues were fixed during this review.** 6 medium/low findings remain as tracked follow-ups.

---

## Fixes Applied

### Fix 1: RunStore Mutex Consolidation (P0 — Critical)
**File:** `crates/goose-server/src/routes/runs.rs`

**Problem:** `RunStore` used 4 separate `Arc<Mutex<HashMap>>` fields (`runs`, `events`,
`cancel_tokens`, `await_metadata`). The `create()` method acquired 3 locks sequentially,
creating a non-atomic window where a run could be partially created.

**Fix:** Consolidated into a single `Arc<Mutex<RunStoreInner>>` struct. All operations are now
atomic under one lock.

### Fix 2: TOCTOU Race in `resume_run` (P0 — Critical)
**File:** `crates/goose-server/src/routes/runs.rs`

**Problem:** `resume_run` called `store.get()` to check status, released the lock, then called
`store.take_await_metadata()` separately. Two concurrent resumes could both pass the status
check and race to take the metadata.

**Fix:** New `take_await_if_awaiting()` method atomically checks status AND takes metadata in
one lock acquisition. Returns `409 Conflict` on race instead of `500 Internal Server Error`.

### Fix 3: RunStore Memory Leak Prevention (P1 — High)
**File:** `crates/goose-server/src/routes/runs.rs`

**Problem:** Completed/failed/cancelled runs accumulated indefinitely in memory with no
eviction. Long-running servers would leak memory.

**Fix:** Added `evict_completed()` with LRU eviction of oldest completed runs when count
exceeds `MAX_COMPLETED_RUNS` (1000). Called automatically on `create()`.

### Fix 4: SSE Parser Deduplication (P2 — Medium)
**File:** `crates/goose-cli/src/goosed_client.rs`

**Problem:** `GoosedHandle::reply_with_mode()` had 25 lines of inline SSE parsing that
duplicated the `process_sse_buffer()` function used by `GoosedClient::reply()`.

**Fix:** Replaced inline parsing with a call to the shared `process_sse_buffer()` function.

### Fix 5: Dead Code Removal (P3 — Low)
**File:** `crates/goose-server/src/routes/runs.rs`

Removed unused `take_await_metadata()` after replacing it with atomic `take_await_if_awaiting()`.

### Fix 6: ACP Discovery — Agent/Mode Harmonization (P0 — Architecture)
**File:** `crates/goose-server/src/routes/acp_discovery.rs`

**Problem:** The ACP discovery endpoint flattened every mode into a separate "agent", producing
15+ agents instead of the correct 2 (Goose Agent, Coding Agent). This conflated the legacy
"1 mode = 1 agent" model with the correct A2A/ACP pattern of "1 persona = 1 agent with N modes".

**Fix:** Rewrote `build_agent_manifests()` to emit one `AgentManifest` per `AgentSlot` (persona),
with modes listed as `AgentModeInfo` entries inside each manifest. The `/agents/{name}` endpoint
now resolves agent slugs (e.g., `goose-agent`, `coding-agent`) to the correct manifest.

### Fix 7: AgentManifest Schema — Added Mode Support (P1 — Schema)
**File:** `crates/goose/src/acp_compat/manifest.rs`

**Problem:** `AgentManifest` had no way to express the modes an agent supports.

**Fix:** Added `modes: Vec<AgentModeInfo>` and `default_mode: Option<String>` fields.
New `AgentModeInfo` struct captures `id`, `name`, `description`, and `tool_groups` per mode.

### Fix 8: Test Alignment for New Agent Model
**File:** `crates/goose-server/src/routes/acp_discovery.rs`

Rewrote all 6 discovery tests to validate the new 1-persona = N-modes model:
- `test_build_agent_manifests_returns_agents_not_modes` — exactly 2 agents
- `test_goose_agent_has_modes` — 7 modes including "assistant"
- `test_coding_agent_has_modes` — 8 modes including "backend"
- `test_modes_have_tool_groups` — each mode has tool group metadata
- `test_slugify_agent_name` / `test_resolve_mode_to_agent` — slug resolution

---

## Remaining Findings

### P1 — High

| # | Finding | Location | Description |
|---|---------|----------|-------------|
| R1 | `AcpIdeSessions` no eviction | `acp_ide.rs` | Same unbounded growth pattern that `RunStore` had. Sessions accumulate in memory with no cleanup. |
| R2 | Bare 500 errors | `runs.rs`, `session.rs`, etc. | ~50 `StatusCode::INTERNAL_SERVER_ERROR` returns that discard the original error. Should use structured error responses. |

### P2 — Medium

| # | Finding | Location | Description |
|---|---------|----------|-------------|
| R3 | `goosed_client.rs` size | 1490 lines | Should split into `goosed_handle.rs`, `goosed_client.rs`, `sse_parser.rs` modules. |
| R4 | `#[allow(dead_code)]` | 4 locations | `PlanProposal` variant, `PlanTask` struct, `jsonrpc` field — wire up or remove. |

### P3 — Low

| # | Finding | Location | Description |
|---|---------|----------|-------------|
| R5 | `console.log` in UI | ~10 files | Remove or gate behind `NODE_ENV === 'development'`. |
| R6 | TODOs | 6 locations | Track as issues: summon_extension.rs (3), extension_manager.rs (1), session/mod.rs (1), apps_extension.rs (1). |

---

## Architecture Notes

### A2A / ACP Alignment

The branch now correctly implements the A2A/ACP protocol pattern:

- **1 Agent = 1 Persona** (e.g., "Goose Agent", "Coding Agent")
- **1 Agent has N SessionModes** (e.g., assistant, specialist, planner, architect, backend, etc.)
- **Modes are per-session** — switched via `SetSessionModeRequest` or `session/setMode` JSON-RPC

This aligns with:
- [A2A Protocol](https://github.com/a2aproject/A2A) — Agent Cards with capabilities
- [Agent Client Protocol](https://docs.rs/agent-client-protocol/) — `SessionModeState` with `available_modes`
- The `agent-client-protocol-schema` v0.10.8 `SessionMode` type

### Data Flow
```
IntentRouter (2 AgentSlots)
  ├── Goose Agent (7 modes: assistant, specialist, recipe_maker, app_maker, app_iterator, judge, planner)
  └── Coding Agent (8 modes: pm, architect, backend, frontend, qa, security, sre, devsecops)

ACP Discovery (/agents) → 2 AgentManifests, each with modes[]
Builtin Agents API (/agents/builtin) → 2 BuiltinAgentInfo, each with modes[]
UI AgentsView → 2 agent cards, expandable mode grids
UI BottomMenu → "2 agents · 15 modes"
```

---

## Verification

```
✅ cargo build          — clean
✅ cargo fmt --check    — clean
✅ cargo clippy --all-targets -- -D warnings  — 0 warnings
✅ cargo test -p goose         — 777 passed
✅ cargo test -p goose-server  — 25 passed (incl. 6 new discovery tests)
✅ cargo test -p goose-cli     — 133 passed
```
