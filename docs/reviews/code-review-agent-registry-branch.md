# Code Review: feature/agent_registry Branch
## Author: Jonathan Mercier | Reviewer: goose | Date: 2025-02-14

---

## Executive Summary

**Branch:** `feature/agent_registry` (aliased as `feature/reasoning-detail-panel`)
**Scope:** 113 files changed, +17,244 / -865 lines across 30 commits
**Verdict:** ✅ **Compiles clean, Clippy clean (-D warnings), all 256+ tests pass, cargo fmt passes.**

This is a massive feature branch that transforms Goose from a single-agent architecture into a multi-agent meta-orchestrator. The code quality is generally **high** — well-structured modules, comprehensive test coverage, and thoughtful API design. However, there are several architectural concerns, dead code paths, security issues, and design smells that should be addressed before merge.

---

## 1. Quality Gates ✅

| Gate | Status |
|------|--------|
| `cargo build` | ✅ Clean |
| `cargo fmt --check` | ✅ Clean |
| `cargo clippy --all-targets -- -D warnings` | ✅ Clean |
| `cargo test -p goose` | ✅ All pass |
| `cargo test -p goose-acp` | ✅ 10/10 pass |
| `cargo test -p goose-server` | ✅ 12/12 pass |
| `cargo test -p goose-cli` | ✅ 117/117 pass |
| Dead code warnings | ✅ None |

---

## 2. Critical Issues (Must Fix Before Merge)

### 🔴 C1: `unsafe { std::env::set_var() }` in Production Code
**File:** `crates/goose-cli/src/commands/registry.rs:619`
```rust
unsafe { std::env::set_var("GOOSE_ORCHESTRATOR_DISABLED", "true") };
```
**Problem:** `std::env::set_var` is `unsafe` since Rust 1.83 because it's not thread-safe. Using it in non-test production code is a soundness issue — any concurrent thread reading env vars will have undefined behavior.
**Fix:** Use a thread-safe config mechanism (e.g., `Config::global()` or an `AtomicBool`).

### 🔴 C2: `std::env::set_var` / `remove_var` in Tests Without Synchronization
**File:** `crates/goose/src/agents/orchestrator_agent.rs:726-728`
```rust
std::env::set_var("GOOSE_ORCHESTRATOR_DISABLED", "true");
assert!(!is_orchestrator_enabled());
std::env::remove_var("GOOSE_ORCHESTRATOR_DISABLED");
```
**Problem:** Tests run in parallel. This mutates global process state without synchronization. Other tests calling `is_orchestrator_enabled()` concurrently will get wrong results. On Rust 1.83+, this is UB.
**Fix:** Use `#[serial_test::serial]` or replace with a testable config injection pattern.

### 🔴 C3: ACP `/runs` Endpoint Is a Stub — Exposes Fake Functionality
**File:** `crates/goose-server/src/routes/runs.rs:298-337`
```rust
// For now, create a simple response acknowledging the run
// Full integration with Agent.reply() will be added when
// the orchestrator is fully wired
store.append_output(&run_id, RunMessage {
    role: "agent".to_string(),
    content: format!("Run {run_id} processed: {user_text}"),
}).await;
```
**Problem:** `POST /runs` accepts requests and returns fake responses. Any client integrating against this will think it works, then break when the real implementation ships. This is **deceptive API behavior**.
**Fix:** Either:
  - (a) Return `501 Not Implemented` with a clear message, or
  - (b) Gate behind a feature flag (`#[cfg(feature = "acp-runs")]`), or
  - (c) Don't register the routes until implemented.

### 🔴 C4: `now_iso()` Returns Unix Timestamp, Not ISO 8601
**File:** `crates/goose-server/src/routes/runs.rs:122-129`
```rust
fn now_iso() -> String {
    let duration = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    let secs = duration.as_secs();
    format!("{secs}")
}
```
**Problem:** Function is named `now_iso` but returns a raw seconds-since-epoch number as a string (e.g. `"1739494640"`). The ACP spec expects ISO 8601 timestamps. Field names `created_at` and `updated_at` suggest ISO format to consumers.
**Fix:** Use `chrono::Utc::now().to_rfc3339()` or equivalent.

---

## 3. High Severity Issues

### 🟠 H1: `RunStore` Is In-Memory with Unbounded Growth
**File:** `crates/goose-server/src/routes/runs.rs:84-120`
**Problem:** `RunStore` is `HashMap<String, Run>` with no eviction, TTL, or size limit. In a long-running server, completed runs accumulate forever. There's no `prune_completed()` equivalent (unlike `TaskManager`).
**Fix:** Add a max-size or TTL eviction. At minimum, add a `/runs` limit parameter and document the in-memory nature.

### 🟠 H2: Global Singleton `acp_manager()` in Server Routes
**File:** `crates/goose-server/src/routes/agent_management.rs:19-22`
```rust
fn acp_manager() -> &'static Arc<Mutex<AgentClientManager>> {
    static INSTANCE: OnceLock<Arc<Mutex<AgentClientManager>>> = OnceLock::new();
    INSTANCE.get_or_init(|| Arc::new(Mutex::new(AgentClientManager::default())))
}
```
**Problem:** This is a process-wide singleton, separate from `AppState`. The `AgentSlotRegistry` lives in `AppState`, but ACP agent connections live in this separate singleton. This creates two sources of truth for agent state. The singleton is also untestable (can't inject a mock).
**Fix:** Move `AgentClientManager` into `AppState`, matching the pattern used for `AgentSlotRegistry` and `RunStore`.

### 🟠 H3: Inconsistent Permission Model in `OrchestratorClient`
**File:** `crates/goose/src/agent_manager/client.rs:125-142`
```rust
async fn request_permission(&self, args: ...) -> ... {
    let option_id = args.options.first()
        .map(|o| o.option_id.clone())
        .unwrap_or_else(|| "allow_once".into());
    Ok(RequestPermissionResponse::new(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id)),
    ))
}
```
**Problem:** Auto-approves every permission request from external ACP agents. An external agent could request destructive operations (file deletion, shell execution) and the orchestrator would silently approve. This bypasses Goose's existing permission model (Auto/Approve/Chat modes).
**Fix:** Integrate with the existing `GoosePermission` / approval system so external agent permissions respect the session's `GooseMode`.

### 🟠 H4: `Restricted` Tool Group Access Doesn't Apply `file_regex`
**File:** `crates/goose/src/agents/tool_filter.rs:35-38`
```rust
ToolGroupAccess::Restricted { group, file_regex } => {
    if tool_matches_group(tool, group) {
        let _ = file_regex;  // ← IGNORED
        return true;
    }
}
```
**Problem:** The `Restricted` variant with `file_regex` is defined in the manifest schema, used in tests, and advertised in modes (e.g., architect mode restricts edits to `.md` files). But the actual filtering ignores the regex — it just matches the group name. This is **misleading** since the manifest promises file-level restrictions that don't exist.
**Fix:** Either implement the regex filtering or remove `Restricted` from the schema until it's implemented, and document it as "planned".

---

## 4. Medium Severity Issues

### 🟡 M1: Repeated Agent Construction in `get_tool_groups_for_routing` and `get_recommended_extensions_for_routing`
**File:** `crates/goose/src/agents/orchestrator_agent.rs:382-434`
```rust
pub fn get_tool_groups_for_routing(&self, agent_name: &str, mode_slug: &str) -> ... {
    match agent_name {
        "Goose Agent" => {
            let goose = GooseAgent::new();  // ← New allocation every call
```
**Problem:** `GooseAgent::new()` and `CodingAgent::new()` are called on every routing decision (and again for extensions). These construct `HashMap`s with all modes each time.
**Fix:** Store the agents in `OrchestratorAgent` (they're already used in `catalog` construction) and look up directly.

### 🟡 M2: `AgentHealth` Mixes `AtomicU32` with `Mutex`
**File:** `crates/goose/src/agent_manager/health.rs:22-28`
```rust
pub struct AgentHealth {
    last_activity: Mutex<Instant>,
    consecutive_failures: AtomicU32,
}
```
**Problem:** Using both `AtomicU32` and `Mutex<Instant>` means you can't get a consistent snapshot. `consecutive_failures()` reads the atomic without the lock, so `state()` could see a stale failure count relative to `last_activity`. This is unlikely to cause bugs at current usage levels but is a design smell.
**Fix:** Either use all-atomics (store `last_activity` as `AtomicU64` epoch millis) or all-mutex for consistency.

### 🟡 M3: `create_run_stream` Awaits Full Processing Before Streaming
**File:** `crates/goose-server/src/routes/runs.rs:339-393`
```rust
async fn create_run_stream(...) -> impl Stream<...> {
    process_run(state_clone, run_id.clone(), req).await;  // ← blocks
    let run = store.get(&run_id).await;
    let events: Vec<...> = ...;
    stream::iter(events)  // ← emits all at once
}
```
**Problem:** The "stream" mode processes the entire run synchronously, then emits all events at once. This defeats the purpose of SSE streaming — clients won't see intermediate progress.
**Fix:** When the stub is replaced with real Agent integration, use a channel-based stream that emits events as they happen.

### 🟡 M4: `resolve_names` Constructs a Full `AgentDetail` Just to Call `resolve_dependencies`
**File:** `crates/goose/src/agent_manager/service_broker.rs:128-161`
**Problem:** Creates a 20-field struct just to pass a `Vec<AgentDependency>` to the resolver.
**Fix:** Extract a `resolve_dependency_list(&self, deps: &[AgentDependency])` method.

### 🟡 M5: `generate_run_id()` Uses Millisecond Timestamp — Collision Risk
**File:** `crates/goose-server/src/routes/runs.rs:131-138`
```rust
fn generate_run_id() -> String {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)...as_millis();
    format!("run_{ts:x}")
}
```
**Problem:** Two concurrent requests within the same millisecond get the same run ID. In contrast, `TaskManager` correctly uses `Uuid::new_v4()`.
**Fix:** Use `Uuid::new_v4()` or add a random suffix.

### 🟡 M6: Hardcoded Agent Names as Strings Throughout
**Files:** Multiple (`orchestrator_agent.rs`, `agent_management.rs`, `intent_router.rs`)
```rust
"Goose Agent"  // appears 20+ times across files
"Coding Agent" // appears 15+ times
let valid_names = ["Goose Agent", "Coding Agent"];
```
**Problem:** Agent names are string literals scattered across the codebase. A rename or addition requires touching many files. The validation in routes uses a hardcoded array.
**Fix:** Define constants (`const GOOSE_AGENT: &str = "Goose Agent"`) in a central location, or use an enum.

---

## 5. Low Severity / Style Issues

### 🟢 L1: `#[allow(clippy::too_many_lines)]` on `reply` Handler
**File:** `crates/goose-server/src/routes/reply.rs:199`
**Note:** The reply handler is complex but the suppression suggests the function should be factored. The routing decision handling, message streaming, and telemetry could be separate functions.

### 🟢 L2: Inconsistent `Default` Implementation Patterns
Some structs use `impl Default { fn default() -> Self { Self::new() } }` while others derive it. The manual implementations are correct but inconsistent — `AgentHealth`, `TaskManager`, `ServiceBroker`, `IntentRouter`, `AgentClientManager` all follow the same pattern where `new()` and `default()` are identical.

### 🟢 L3: 3 TODOs in Production Code
**File:** delegation logic in summon extension
```rust
false, // TODO: detect custom extensions from agent frontmatter
false, // TODO: detect model override from agent frontmatter
false, // TODO: detect modes from agent frontmatter
```
**Note:** These mean `DelegationStrategy::choose()` always picks `InProcessSpecialist { simple }` for non-external agents, even when they have custom extensions. The delegation logic works but can't optimize.

### 🟢 L4: `apps` Tool Group in GooseAgent Modes Not Mapped
**File:** `crates/goose/src/agents/goose_agent.rs:115,124`
```rust
tool_groups: vec![ToolGroupAccess::Full("apps".into())],
```
**Note:** The `tool_filter.rs` doesn't have a special case for `"apps"` — it falls through to `other => owner == other`, which works if an extension is named "apps". Document this or add an explicit mapping.

---

## 6. Architecture Review

### What's Done Well ✅
1. **Clean module separation** — `agent_manager`, `registry`, `agents` are well-bounded modules
2. **Comprehensive test coverage** — ~90 new unit tests covering edge cases
3. **Protocol alignment** — ACP and A2A formats are correctly implemented per their specs
4. **Fallback chains** — LLM router → keyword router → default mode is a solid pattern
5. **Typed manifest system** — `RegistryEntry` as a superset of ACP/A2A/Kilo Code is well-designed
6. **Serde roundtrip tests** — All manifest types have parse/generate/roundtrip tests
7. **Health monitoring** — Circuit breaker pattern with configurable thresholds is production-ready
8. **ACP bridge** — The `goose-acp` crate's stdio transport with `!Send` handling via dedicated threads is correct

### Architecture Concerns ⚠️

1. **Dual State Management** — `AgentSlotRegistry` (in AppState) vs `OrchestratorAgent` (in-memory) vs `acp_manager()` (global singleton) — three separate places tracking agent state. These should converge.

2. **Orchestrator Not Wired Into Main Loop** — The `OrchestratorAgent` exists and is tested, but `reply.rs` still delegates directly to `agent.reply()`. The routing decision is emitted as an SSE event (`AgentEvent::RoutingDecision`) but doesn't actually change which agent handles the request. This is the biggest gap.

3. **Tool Filter Not Connected** — `tool_filter.rs` is comprehensive but there's no call to `filter_tools()` in the agent's main loop (`agent.rs`). The mode's `tool_groups` are looked up but not applied to actual tool lists.

---

## 7. Security Assessment

| Check | Result |
|-------|--------|
| Hardcoded secrets | ✅ None found (all "secret" references are test fixtures) |
| `unsafe` usage | 🔴 1 instance in production (`set_var`) — see C1 |
| Permission bypass | 🟠 External agents auto-approved — see H3 |
| Input validation | ✅ Agent names validated in routes |
| SQL injection | N/A (no SQL) |
| Path traversal | ✅ Handled by existing developer extension |
| Dependency audit | ✅ `agent-client-protocol` crate is from BeeAI (reputable) |

---

## 8. Recommendations

### Before Merge (Critical)
1. Fix `unsafe { set_var() }` in production code (C1)
2. Fix test thread-safety for env vars (C2)
3. Gate or remove stub `/runs` endpoint (C3)
4. Fix `now_iso()` to return actual ISO timestamps (C4)

### Short-term Follow-up
5. Integrate permission model for external agents (H3)
6. Implement or remove `Restricted` file_regex filtering (H4)
7. Move `AgentClientManager` into `AppState` (H2)
8. Fix run ID generation to use UUIDs (M5)
9. Define agent name constants (M6)

### Before Production
10. Wire `OrchestratorAgent` routing into the main agent loop
11. Connect `tool_filter::filter_tools()` to actual tool resolution
12. Add eviction to `RunStore`
13. Implement real `process_run` with `Agent.reply()` integration

---

## 9. Test Summary

| Crate | Tests | Status |
|-------|-------|--------|
| goose | ~120+ | ✅ Pass (5 tetrate tests ignored — require live API) |
| goose-acp | 10 | ✅ Pass |
| goose-server | 12 | ✅ Pass |
| goose-cli | 117 | ✅ Pass |
| **Total** | **~260+** | **✅ All pass** |

---

## 10. Files Reviewed

### New Files (Deep Review)
- `crates/goose/src/agent_manager/` — all 6 files (spawner, health, task, client, service_broker, acp_mcp_adapter)
- `crates/goose/src/agents/orchestrator_agent.rs` (751 LOC)
- `crates/goose/src/agents/intent_router.rs` (300 LOC)
- `crates/goose/src/agents/coding_agent.rs` (~400 LOC)
- `crates/goose/src/agents/goose_agent.rs` (~300 LOC)
- `crates/goose/src/agents/tool_filter.rs` (237 LOC)
- `crates/goose/src/agents/delegation.rs` (169 LOC)
- `crates/goose/src/registry/manifest.rs` (952 LOC)
- `crates/goose/src/registry/formats.rs` (823 LOC)
- `crates/goose-server/src/routes/agent_management.rs` (536 LOC)
- `crates/goose-server/src/routes/runs.rs` (393 LOC)
- `crates/goose-server/src/routes/agent_card.rs` (84 LOC)
- `crates/goose-server/src/agent_slot_registry.rs` (105 LOC)
- `crates/goose-acp/src/` — all files (server, bridge, transport, notification, adapters, server_factory)
- `crates/goose-server/src/routes/reply.rs` (routing integration)

### Checks Performed
- ✅ Compilation (`cargo build`)
- ✅ Formatting (`cargo fmt --check`)
- ✅ Lint (`cargo clippy --all-targets -- -D warnings`)
- ✅ Unit tests (all crates)
- ✅ Security scan (hardcoded secrets, unsafe, permission model)
- ✅ Dead code analysis
- ✅ unwrap/expect audit (160 instances, most in tests)
- ✅ Thread-safety review (env vars, atomics, mutexes)
- ✅ API stub detection
- ✅ Architecture coherence review
