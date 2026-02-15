# Extension-Agent Separation — ACP-Aligned Architecture

## Status: Research Complete — Implementation Plan Ready

## Problem

Today, MCP extensions (tool providers) are tightly coupled to the `Agent` instance:

```
Agent
  └─ ExtensionManager
       └─ extensions: HashMap<String, Extension>  // MCP clients
       └─ tools_cache: Vec<Tool>                  // all tools from all extensions
```

This means:
1. **Extensions are per-agent** — if two agents share a session, they can't share extensions
2. **Tool filtering is an afterthought** — all tools are loaded, then filtered by `ToolGroupAccess` at inference time
3. **No ACP discoverability** — extensions aren't visible in `AgentManifest.metadata.dependencies`
4. **Extension lifecycle tied to agent** — adding/removing extensions requires the Agent instance

## ACP Standard: AgentDependency (Experimental)

ACP v0.2.0 defines an `AgentDependency` schema:

```yaml
AgentDependency:
  type: object
  properties:
    type:
      enum: [agent, tool, model]
    name:
      type: string
```

This maps to Goose concepts:
- `type: tool` → MCP extension (developer, computeruse, memory, etc.)
- `type: agent` → other Goose agent (for orchestrator delegation)
- `type: model` → required LLM model

## Current Architecture

```
┌─────────────────────────────────────┐
│ Agent (per session)                  │
│   ├─ ExtensionManager               │
│   │   ├─ developer (MCP client)     │
│   │   ├─ computeruse (MCP client)   │
│   │   └─ memory (MCP client)        │
│   │                                  │
│   ├─ active_tool_groups: Vec<TGA>    │ ← set per-mode at routing time
│   ├─ allowed_extensions: Vec<Str>    │ ← set per-mode at routing time
│   │                                  │
│   └─ tool_filter::filter_tools()     │ ← applied at inference time
└─────────────────────────────────────┘

AgentSlotRegistry (server-level)
  ├─ enabled_agents: {name → bool}
  └─ bound_extensions: {agent_name → Set<ext_name>}
```

Key observations:
- `ExtensionManager` lives inside `Agent` — 1:1 coupling
- `AgentSlotRegistry.bound_extensions` tracks which extensions belong to which agent slot
- `ToolGroupAccess` filters at inference time what tools are visible per mode
- Extensions are loaded at session start (`load_extensions_from_session`)

## Desired Architecture

```
┌──────────────────────────────────────────────┐
│ ExtensionRegistry (server-level singleton)    │
│   ├─ developer (MCP client, shared)           │
│   ├─ computeruse (MCP client, shared)         │
│   └─ memory (MCP client, shared)              │
│                                                │
│   Methods:                                     │
│     add_extension(config) → Result<()>         │
│     remove_extension(name) → Result<()>        │
│     list_tools(filter?) → Vec<Tool>            │
│     get_extensions_for_agent(name) → Vec<Ext>  │
└──────────────────────────────────────────────┘
           │
           │ shared reference
           ▼
┌──────────────────────────────────────────────┐
│ Agent (per session)                           │
│   ├─ extension_ref: &ExtensionRegistry        │ ← borrows, doesn't own
│   ├─ active_tool_groups: Vec<TGA>             │
│   ├─ allowed_extensions: Vec<String>          │
│   └─ tool_filter applied at inference         │
└──────────────────────────────────────────────┘

AgentManifest.metadata.dependencies:
  [
    { type: "tool", name: "developer" },
    { type: "tool", name: "computeruse" },
    { type: "model", name: "claude-sonnet-4-20250514" }
  ]
```

## Benefits

1. **Extensions shared across agents** — no duplicate MCP connections
2. **ACP-discoverable** — each agent's manifest lists its tool dependencies
3. **Lifecycle decoupled** — extensions managed at server level, not per-agent
4. **Per-agent tool scoping** — `bound_extensions` + `ToolGroupAccess` still control visibility
5. **Hot-reload** — add/remove extensions without restarting agents

## Implementation Phases

### Phase 1: Expose Dependencies in AgentManifest (P2, small)

Add `dependencies` field to `AgentManifest.metadata`:

```rust
// acp_compat/manifest.rs
pub struct AgentDependency {
    pub dep_type: String,  // "tool", "agent", "model"
    pub name: String,
}

// In AgentMetadata
pub dependencies: Option<Vec<AgentDependency>>,
```

Populate from each mode's `tool_groups` and `recommended_extensions`:
- `ToolGroupAccess::Full("developer")` → `{ type: "tool", name: "developer" }`
- Agent's provider model → `{ type: "model", name: "claude-sonnet-4-20250514" }`

**Files:** `manifest.rs`, `acp_discovery.rs`

### Phase 2: Extract ExtensionRegistry from Agent (P2, medium)

Create a server-level `ExtensionRegistry` that manages all MCP connections:

```rust
// New: crates/goose-server/src/extension_registry.rs
pub struct ExtensionRegistry {
    manager: ExtensionManager,  // reuse existing impl
}
```

- Move `ExtensionManager` creation from `Agent::with_config()` to `AppState`
- Agent gets `Arc<ExtensionRegistry>` reference instead of owning `ExtensionManager`
- `AgentManager::get_or_create_agent()` passes the shared registry

**Files:** `extension_registry.rs` (new), `state.rs`, `agent.rs`, `agent_manager.rs`

### Phase 3: Extension-per-Agent Binding via AgentSlotRegistry (P3, medium)

Enhance `AgentSlotRegistry` to be the single source of truth for which extensions each agent can access:

- `bound_extensions` already exists — promote it to primary mechanism
- When agent processes a request, filter available tools through:
  1. `bound_extensions[agent_name]` — which extensions this agent can see
  2. `active_tool_groups` — which tool groups within those extensions

**Files:** `agent_slot_registry.rs`, `reply.rs`, `runs.rs`

### Phase 4: Server-Level Extension Lifecycle (P3, large)

Add routes for managing extensions independently of agents:

- `POST /extensions` — add extension to registry
- `DELETE /extensions/{name}` — remove extension
- `GET /extensions` — list all loaded extensions
- `POST /agents/{name}/extensions` — bind extension to agent
- `DELETE /agents/{name}/extensions/{ext}` — unbind

**Files:** new `extensions.rs` route, `acp_discovery.rs` updates

### Phase 5: Hot-Reload and Extension Sharing (P4, large)

- Extensions shared across sessions (no reconnection per session)
- Hot-reload: add/remove extensions without agent restart
- Extension health monitoring at server level

**Files:** `extension_registry.rs`, `state.rs`

## Migration Strategy

1. Phase 1 is additive — no breaking changes
2. Phase 2 introduces the registry but keeps the Agent API unchanged (Agent delegates to registry)
3. Phase 3 refactors internal routing but external API unchanged
4. Phase 4 adds new routes (no changes to existing)
5. Phase 5 optimizes internals

Each phase can be shipped independently. Phase 1 is the quick win for ACP compliance.
