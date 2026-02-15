# Multi-Layer Orchestrator Architecture

> **RFC** — Transform Goose from intent-router-based routing to a true multi-layer orchestrator
>
> **Author:** Jonathan Mercier  
> **Status:** Draft  
> **Date:** 2026-02-13  
> **Branch:** `feature/agent_registry`

---

## 1. Problem Statement

Goose currently uses a **keyword-based IntentRouter** (in reply.rs) to select an agent/mode before
delegating to a single `Agent::reply()` call. This has several limitations:

1. **No compound request splitting** — "Build the API and write tests" goes to one agent as-is
2. **No LLM-based understanding** — keyword matching is brittle (see `words_match` workaround)
3. **No task coordination** — single request → single agent → single response
4. **Compactor is misplaced** — it's a GooseAgent mode but compaction is a cross-cutting orchestration concern
5. **Service wiring is manual** — extensions are bound via UI toggle, not from agent manifest dependencies

## 2. Target Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    Layer 1: Interface                             │
│                                                                  │
│  ┌──────────┐    ┌──────────────┐    ┌──────────────────┐       │
│  │   CLI    │    │   Desktop    │    │    Web (future)  │       │
│  │  goose   │    │  (Electron)  │    │   (React SPA)    │       │
│  └────┬─────┘    └──────┬───────┘    └────────┬─────────┘       │
│       │                 │                     │                  │
│       └────────────┬────┴─────────────────────┘                  │
│                    │  REST / SSE / WebSocket                     │
└────────────────────┼─────────────────────────────────────────────┘
                     │
┌────────────────────▼─────────────────────────────────────────────┐
│                    Layer 2: Middleware (goose-server)             │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                   OrchestratorAgent                        │  │
│  │                                                            │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │  │
│  │  │ Intent       │  │ Task         │  │ Result         │  │  │
│  │  │ Classifier   │  │ Splitter     │  │ Aggregator     │  │  │
│  │  │ (LLM-based)  │  │ (compound    │  │ (merge sub-    │  │  │
│  │  │              │  │  requests)   │  │  task outputs)  │  │  │
│  │  └──────────────┘  └──────────────┘  └────────────────┘  │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │  │
│  │  │ Compactor    │  │ Session      │  │ Plan Executor  │  │  │
│  │  │ (moved from  │  │ Manager      │  │ (sequential/   │  │  │
│  │  │  GooseAgent) │  │              │  │  parallel)     │  │  │
│  │  └──────────────┘  └──────────────┘  └────────────────┘  │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────────┐  ┌──────────────────────────────────────┐  │
│  │  ServiceBroker   │  │  Agent Discovery                     │  │
│  │  (manifest deps  │  │  (ACP /agents, A2A agent-card.json,  │  │
│  │   → MCP binding) │  │   registry, embedded)                │  │
│  └──────────────────┘  └──────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────────┐  ┌──────────────────────────────────────┐  │
│  │  AgentSpawner    │  │  Health Monitor                      │  │
│  │  (bin/npx/uvx/   │  │  (heartbeat, circuit breaker,        │  │
│  │   cargo/docker)  │  │   auto-prune)                        │  │
│  └──────────────────┘  └──────────────────────────────────────┘  │
└────────┬───────────────────────┬──────────────────┬──────────────┘
         │ in-process            │ ACP/stdio         │ ACP/HTTP
         ▼                       ▼                   ▼
┌────────────────────────────────────────────────────────────────────┐
│                    Layer 3: Agents & Services                      │
│                                                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐ │
│  │ GooseAgent   │  │ CodingAgent  │  │ External ACP Agents      │ │
│  │ (generalist) │  │ (SDLC modes) │  │ (spawned or remote)      │ │
│  │              │  │              │  │                           │ │
│  │ Modes:       │  │ Modes:       │  │ Examples:                │ │
│  │ • assistant  │  │ • pm         │  │ • Translation agent      │ │
│  │ • specialist │  │ • architect  │  │ • Data analysis agent    │ │
│  │ • recipe_mkr │  │ • backend    │  │ • Custom enterprise agent│ │
│  │ • app_maker  │  │ • frontend   │  │                          │ │
│  │ • app_iter   │  │ • qa         │  │ Discovered via:          │ │
│  │ • judge      │  │ • security   │  │ • GET /agents            │ │
│  │ • planner    │  │ • sre        │  │ • /.well-known/agent.json│ │
│  │              │  │ • devsecops  │  │ • Registry search        │ │
│  └──────┬───────┘  └──────┬───────┘  └────────────┬─────────────┘ │
│         │                 │                       │               │
│         └─────────┬───────┴───────────────────────┘               │
│                   │  MCP Extensions (tools/resources)              │
│                   ▼                                                │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ developer, computercontroller, memory, fetch, context7,     │  │
│  │ beads-mcp, custom MCP servers, ACP-MCP adapter bridges      │  │
│  └─────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────┘
```

## 3. Key Design Decisions

### 3.1 OrchestratorAgent replaces IntentRouter

| Aspect | Current (IntentRouter) | Target (OrchestratorAgent) |
|--------|----------------------|---------------------------|
| Classification | Keyword matching (`words_match`) | LLM-based intent classification |
| Compound requests | Not supported | Split into sub-tasks |
| Coordination | Single agent/mode per request | Parallel/sequential task execution |
| Compaction | GooseAgent mode | Orchestrator responsibility |
| Location | `intent_router.rs` (stateless fn) | `orchestrator_agent.rs` (Agent impl) |

The OrchestratorAgent uses the same `Agent` trait but has a special system prompt
that gives it knowledge of available agents, their modes, and their capabilities.
It uses **tool calls** to delegate work:

```
delegate_to_agent(agent_name, mode, instructions, input) → output
compact_conversation(session_id, threshold) → compacted_history
split_request(user_message) → [sub_tasks]
```

### 3.2 Service Wiring via ACP Manifest Dependencies

**Your intuition is correct.** In the ACP protocol:

1. Agent declares dependencies in its manifest:
   ```json
   {
     "metadata": {
       "dependencies": [
         { "type": "tool", "name": "developer" },
         { "type": "tool", "name": "fetch" }
       ]
     }
   }
   ```

2. The **ACP server** (goose-server) reads these dependencies and **binds** the
   corresponding MCP extensions to the agent's session.

3. This is exactly what `AgentSlotRegistry` + `ExtensionManager` already do — the
   missing piece is **automatic binding from manifest dependencies** instead of
   manual UI toggles.

The `ServiceBroker` component:
- Reads agent manifest `metadata.dependencies`
- Resolves each dependency to an available MCP extension
- Binds via `ExtensionManager::add_extension()`
- Falls back to ACP-MCP adapter for agent-as-tool bridging

### 3.3 ACP Dual Role

Goose-server acts as **both** ACP server and ACP client:

```
                    ACP Server                    ACP Client
                    ──────────                    ──────────
Exposes:            GET /agents                   Connects to:
                    POST /runs                    External ACP servers
                    /.well-known/agent.json       Remote agents

GooseAgent ────────→ ACP manifest  ←──────────── Client discovers
CodingAgent ───────→ ACP manifest  ←──────────── Client runs
OrchestratorAgent ─→ ACP manifest  ←──────────── Client coordinates

                    ACP-MCP Adapter
                    ───────────────
External ACP agents exposed as MCP tools to OrchestratorAgent
```

### 3.4 Compactor Migration

Compaction is currently in two places:
- `GooseAgent` has a `compactor` mode (compaction.md template)
- `Agent::reply()` has auto-compaction logic (agent.rs:970-1035)

**Migration:**
1. Move `compactor` mode from GooseAgent to OrchestratorAgent
2. Expose `compact_conversation` as an orchestrator tool
3. Auto-compaction stays in `Agent::reply()` (it's the inner loop safety net)
4. OrchestratorAgent can proactively compact before delegating to agents with smaller context windows

## 4. Phased Implementation Plan

### Phase 1: OrchestratorAgent Foundation
> **Goal:** Replace IntentRouter with LLM-based orchestrator

**Files:**
- `crates/goose/src/agents/orchestrator_agent.rs` — new
- `crates/goose/src/prompts/orchestrator/system.md` — new
- `crates/goose/src/prompts/orchestrator/routing.md` — new
- `crates/goose-server/src/routes/reply.rs` — modify (use OrchestratorAgent)

**Tasks:**
1. Create `OrchestratorAgent` struct with LLM-based routing
2. Build system prompt with agent catalog (from registry manifests)
3. Implement `delegate_to_agent` tool (wraps current DelegationStrategy)
4. Wire into reply.rs replacing IntentRouter
5. Keep IntentRouter as fallback for non-LLM providers

**Success criteria:**
- OrchestratorAgent correctly routes "implement a REST API" → CodingAgent/backend
- OrchestratorAgent correctly routes "hello" → GooseAgent/assistant
- Existing tests pass + new orchestrator tests

### Phase 2: Compound Request Splitting
> **Goal:** Handle multi-intent requests

**Files:**
- `crates/goose/src/agents/orchestrator_agent.rs` — extend
- `crates/goose/src/prompts/orchestrator/splitting.md` — new

**Tasks:**
1. Add `split_request` tool to OrchestratorAgent
2. Implement sequential execution (task A → task B)
3. Implement parallel execution (task A ∥ task B)
4. Result aggregation from sub-task outputs

**Success criteria:**
- "Build the API and write tests" → CodingAgent/backend + CodingAgent/qa
- "Translate this to French and Spanish" → parallel agent execution

### Phase 3: Service Broker
> **Goal:** Automatic agent↔service wiring from manifest dependencies

**Files:**
- `crates/goose/src/agent_manager/service_broker.rs` — new
- `crates/goose/src/registry/manifest.rs` — extend (dependency resolution)
- `crates/goose-server/src/routes/agent_management.rs` — modify

**Tasks:**
1. Parse `metadata.dependencies` from AgentManifest
2. Resolve tool dependencies to available MCP extensions
3. Auto-bind on agent session creation
4. Add fallback: ACP-MCP adapter for agent-as-tool bridging
5. Update AgentsView.tsx to show auto-bound vs manually-bound extensions

**Success criteria:**
- Agent with `dependencies: [{type: "tool", name: "developer"}]` auto-gets developer extension
- External ACP agent's tools visible in OrchestratorAgent's tool catalog

### Phase 4: Compactor Migration & Proactive Management
> **Goal:** Move compaction to orchestrator layer

**Files:**
- `crates/goose/src/agents/orchestrator_agent.rs` — extend
- `crates/goose/src/agents/goose_agent.rs` — remove compactor mode
- `crates/goose/src/prompts/orchestrator/compaction.md` — new (from compaction.md)

**Tasks:**
1. Add `compact_conversation` tool to OrchestratorAgent
2. Move compaction.md template to orchestrator prompts
3. Remove compactor mode from GooseAgent
4. Implement proactive compaction (compact before delegating to small-context agents)
5. Keep auto-compaction in Agent::reply() as safety net

**Success criteria:**
- OrchestratorAgent compacts proactively when delegating to constrained agents
- GooseAgent no longer has compactor mode
- Auto-compaction still works as fallback

### Phase 5: End-to-End Integration & Desktop UI
> **Goal:** Full pipeline working through all interfaces

**Files:**
- `crates/goose-server/src/routes/reply.rs` — finalize
- `crates/goose-cli/src/commands/agents.rs` — update
- `ui/desktop/src/components/agents/AgentsView.tsx` — extend
- `ui/desktop/src/components/OrchestratorPanel.tsx` — new

**Tasks:**
1. Orchestrator events in SSE stream (TaskDelegation, SubTaskResult)
2. CLI: `goose agents orchestrate` command
3. Desktop: OrchestratorPanel showing task graph, delegation flow
4. Desktop: Service dependency visualization
5. E2E tests across all interfaces

**Success criteria:**
- Desktop shows orchestration flow in real-time
- CLI shows delegation decisions
- All existing functionality preserved

## 5. ACP Protocol Alignment

### What ACP says about orchestration (from spec):

> "The Router Agent pattern is a common design where a central agent:
> Decomposes complex requests into specialized sub-tasks,
> Routes tasks to appropriate specialist agents,
> Aggregates responses into cohesive results,
> Uses its own tools and those exposed by downstream agents via the MCP extension"

Our OrchestratorAgent IS the Router Agent pattern.

### ACP endpoints our server already exposes:
| Endpoint | Status | Maps to |
|----------|--------|---------|
| `GET /agents` | ✅ via agent_management routes | List builtin + external agents |
| `POST /agents/{name}/sessions` | ✅ via AcpBridge | Create session |
| `POST /agents/{name}/sessions/{id}/prompt` | ✅ via AcpBridge | Send message |
| `/.well-known/agent.json` | ✅ via agent_card route | Discovery |
| `GET /agents/{name}/modes` | ✅ via AcpBridge | List modes |

### What's missing for full ACP compliance:
| Endpoint | Status | Needed for |
|----------|--------|-----------|
| `POST /runs` | ❌ | Standard ACP run lifecycle |
| `GET /runs/{run_id}` | ❌ | Run status polling |
| `POST /runs/{run_id}` | ❌ | Resume awaiting runs |
| `GET /runs/{run_id}/events` | ❌ | Event history |

### ACP-MCP Adapter integration:
```
External ACP Agent ← uvx acp-mcp http://external:8000 → MCP tools → OrchestratorAgent
                                                                    └→ delegates via tool call
```

## 6. Migration Strategy

### Backward Compatibility
- IntentRouter kept as fallback for simple/fast routing
- GooseAgent retains all modes except compactor
- Existing SSE events preserved + new ones added
- Desktop UI remains functional during migration

### Feature Flags
```
GOOSE_ORCHESTRATOR_ENABLED=true    # Use OrchestratorAgent (default: false)
GOOSE_INTENT_SPLITTING=true        # Enable compound request splitting
GOOSE_AUTO_SERVICE_BROKER=true     # Auto-bind from manifest dependencies
```

### Rollback
Each phase is independently deployable. If OrchestratorAgent fails,
IntentRouter takes over seamlessly.

## 7. Open Questions

1. **OrchestratorAgent model:** Should it use the same model as user's configured provider,
   or a dedicated fast model (e.g., gpt-4o-mini) for routing decisions?
2. **Token budget:** How to distribute context window budget across orchestrator + sub-agents?
3. **ACP /runs endpoint:** Should we implement the full ACP run lifecycle now or defer?
4. **Streaming through orchestrator:** How to stream sub-agent responses back through the
   orchestrator without losing the orchestrator's aggregation ability?

## 8. References

- [ACP Architecture](https://agentcommunicationprotocol.dev/core-concepts/architecture)
- [ACP Agent Manifest](https://agentcommunicationprotocol.dev/core-concepts/agent-manifest)
- [ACP Compose Agents](https://agentcommunicationprotocol.dev/how-to/compose-agents)
- [ACP Discovery](https://agentcommunicationprotocol.dev/core-concepts/agent-discovery)
- [ACP-MCP Adapter](https://agentcommunicationprotocol.dev/integrations/mcp-adapter)
- [A2A AgentCard](https://agent2agent.info/docs/concepts/agentcard/)
- [ACP now part of A2A under Linux Foundation](https://agentcommunicationprotocol.dev/introduction/welcome)
- [agent-client-protocol Rust crate](https://docs.rs/agent-client-protocol/latest/agent_client_protocol/)
- [Goose Meta-Orchestrator RFC](./meta-orchestrator-architecture.md)
- [Goose Agent Observability UX](./agent-observability-ux.md)
