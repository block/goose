# Extension-Agent Separation — Implementation Plan

## Status: Phase A Ready — Phases B-C Planned

## Problem Analysis

`ExtensionManager` (2157 lines) is tightly coupled to `Agent`:
- Agent owns `Arc<ExtensionManager>` — lifecycle tied to agent
- 535 outgoing call chains — massive dependency surface
- Tool caching, filtering, dispatch, resources, prompts all in one module
- No extension sharing between agents/sessions

## Current Coupling Map

```
Agent (agent.rs:2108L)
  └─ extension_manager: Arc<ExtensionManager>  (extension_manager.rs:2157L)
       ├─ extensions: Arc<Mutex<HashMap<String, Extension>>>
       ├─ tools_cache: Arc<Mutex<Option<Arc<Vec<Tool>>>>>
       ├─ tool_ownership: Arc<Mutex<HashMap<String, String>>>
       ├─ version: AtomicUsize (cache invalidation counter)
       └─ config_store: Arc<Mutex<HashMap<String, ExtensionConfig>>>
```

Files with `ExtensionManager` references:
- `agent.rs` — owns Arc<ExtensionManager>, 20+ usages
- `extension_manager_extension.rs` — manage_extensions tool
- `apps_extension.rs` — app storage via resources
- `moim.rs` — context collection
- `mcp_client.rs` — MCP client creation
- `tool_filter.rs` — tool group filtering
- `extension.rs` — Extension struct definition

Server files:
- `state.rs` — AppState holds agent sessions
- Routes that access extensions through agent

## Phase A: Extract ToolRegistry (Safe, No API Changes)

**Goal**: Split tool-related logic from ExtensionManager into a separate `ToolRegistry`.

**What moves**:
- `get_all_tools_cached()` → `ToolRegistry::all_tools()`
- `get_prefixed_tools()` → `ToolRegistry::prefixed_tools()`
- `filter_tools()` → `ToolRegistry::filtered_tools()`
- `resolve_tool()` → `ToolRegistry::resolve()`
- `dispatch_tool_call()` → `ToolRegistry::dispatch()`
- `tools_cache` + `version` + `tool_ownership` fields

**What stays** in ExtensionManager:
- Extension lifecycle (add, remove, list)
- Resource management (list, read)
- Prompt management (list, get)
- Config management
- MCP client connections

**New struct**:
```rust
pub struct ToolRegistry {
    extensions: Arc<Mutex<HashMap<String, Extension>>>, // shared ref
    tools_cache: Arc<Mutex<Option<Arc<Vec<Tool>>>>>,
    tool_ownership: Arc<Mutex<HashMap<String, String>>>,
    version: AtomicUsize,
}

impl ToolRegistry {
    pub fn new(extensions: Arc<Mutex<HashMap<String, Extension>>>) -> Self;
    pub async fn all_tools(&self) -> Result<Arc<Vec<Tool>>>;
    pub async fn prefixed_tools(&self, groups: &[ToolGroupAccess], exts: Option<&[String]>) -> Result<Vec<Tool>>;
    pub async fn resolve(&self, tool_name: &str) -> Result<(String, Arc<Mutex<McpClientExt>>)>;
    pub async fn dispatch(&self, call: &ToolCall, ...) -> Result<ToolCallResult>;
}
```

**Files to modify**:
- NEW: `crates/goose/src/agents/tool_registry.rs`
- `extension_manager.rs` — delegate tool methods to ToolRegistry
- `agent.rs` — no changes (ExtensionManager still delegates)

**Risk**: LOW — no API changes, just internal decomposition.
**Estimated effort**: 2-3 hours.

## Phase B: Extract ExtensionRegistry (Server-Level Singleton)

**Goal**: Move extension storage from Agent to server-level AppState.

**What changes**:
- `AppState` gets `extension_registry: Arc<ExtensionRegistry>`
- `ExtensionRegistry` wraps `Arc<Mutex<HashMap<String, Extension>>>`
- Agent receives `&ExtensionRegistry` instead of creating its own extensions
- `ExtensionManager` becomes a thin facade that borrows from `ExtensionRegistry`

**New struct**:
```rust
pub struct ExtensionRegistry {
    extensions: Arc<Mutex<HashMap<String, Extension>>>,
    configs: Arc<Mutex<HashMap<String, ExtensionConfig>>>,
}

impl ExtensionRegistry {
    pub async fn add(&self, config: ExtensionConfig) -> Result<()>;
    pub async fn remove(&self, name: &str) -> Result<()>;
    pub fn list(&self) -> Vec<String>;
    pub fn get_tools_for_agent(&self, bound: &[String]) -> Vec<Tool>;
}
```

**Files to modify**:
- NEW: `crates/goose/src/agents/extension_registry.rs`
- `extension_manager.rs` — take ExtensionRegistry ref instead of owning extensions
- `agent.rs` — pass ExtensionRegistry when creating ExtensionManager
- `crates/goose-server/src/state.rs` — add ExtensionRegistry to AppState

**Risk**: MEDIUM — changes Agent construction, session creation paths.
**Estimated effort**: 4-6 hours.

## Phase C: Per-Agent Extension Binding + Server Routes

**Goal**: Each agent only sees its bound extensions. New REST routes for extension management.

**Changes**:
- `AgentSlotRegistry.bound_extensions` drives tool scoping
- New routes: `POST /extensions`, `DELETE /extensions/{name}`, `GET /extensions`
- Extension lifecycle independent of agent/session lifecycle
- Hot-reload: add/remove extensions without restarting sessions

**Risk**: MEDIUM-HIGH — new API surface, session management changes.
**Estimated effort**: 4-6 hours.

## Migration Strategy

1. Phase A is pure internal refactoring — no tests need to change
2. Phase B changes construction but keeps the same API — integration tests may need updates
3. Phase C adds new routes — needs new tests

Each phase ships independently. No breaking changes between phases.

## Testing Strategy

- Phase A: All existing tests must pass unchanged
- Phase B: Update session creation tests, add ExtensionRegistry unit tests
- Phase C: Add REST route integration tests

## Success Criteria

- [ ] Agent no longer owns extensions directly
- [ ] Multiple agents can share extensions (no duplicate MCP connections)
- [ ] Per-agent tool scoping works
- [ ] Extension hot-reload (add/remove without restart)
- [ ] All existing tests pass
- [ ] New extension routes have >80% test coverage
