# Goose Multi-Agent Architecture — Complete Reference

**Author:** Jonathan Mercier  
**Date:** 2026-02-14  
**Branch:** `feature/reasoning-detail-panel`  
**Status:** Living Document  
**Version:** 1.0

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Architecture Overview](#2-architecture-overview)
3. [Protocol Landscape: ACP, A2A, and MCP](#3-protocol-landscape-acp-a2a-and-mcp)
4. [Crate Map & Source Structure](#4-crate-map--source-structure)
5. [Layer 1 — Interfaces (CLI / Desktop / Web)](#5-layer-1--interfaces)
6. [Layer 2 — Middleware (Goose Server & Orchestration)](#6-layer-2--middleware)
7. [Layer 3 — Agents & Services](#7-layer-3--agents--services)
8. [The ACP Bridge (`goose-acp` Crate)](#8-the-acp-bridge-goose-acp-crate)
9. [Agent Manager Subsystem](#9-agent-manager-subsystem)
10. [Registry System](#10-registry-system)
11. [Routing & Intent Classification](#11-routing--intent-classification)
12. [Tool Filtering & Extension Scoping](#12-tool-filtering--extension-scoping)
13. [Extension-Agent Separation (ACP-Aligned)](#13-extension-agent-separation-acp-aligned)
14. [Agent Observability & UX](#14-agent-observability--ux)
15. [Data Flow: End-to-End Request Lifecycle](#15-data-flow-end-to-end-request-lifecycle)
16. [Design Decisions & ADRs](#16-design-decisions--adrs)
17. [Testing Strategy](#17-testing-strategy)
18. [Risks & Open Items](#18-risks--open-items)
19. [References & Sources](#19-references--sources)

---

## 1. Executive Summary

Goose has been transformed from a **single-agent-with-extensions** architecture into a
**multi-agent meta-orchestrator**. The system now supports:

- **16 built-in agent modes** (7 GooseAgent + 8 CodingAgent + 1 OrchestratorAgent compactor)
- **LLM-based intent routing** with keyword fallback
- **Compound request splitting** for multi-task messages
- **External agent support** via ACP (Agent Communication Protocol) over stdio, HTTP, and WebSocket
- **A2A discovery** via `/.well-known/agent-card.json` endpoints
- **Mode-scoped tool filtering** — security boundaries enforced per agent mode
- **Manifest-driven service wiring** — agents declare dependencies, the server resolves them
- **Multi-source registry** — local filesystem, GitHub, HTTP, and A2A discovery
- **Agent health monitoring** with circuit breaker (Healthy → Degraded → Dead)

The work spans **113 files changed**, **~17,244 insertions**, across **30 commits** on the
`feature/agent_registry` and `feature/reasoning-detail-panel` branches.

### Metrics

| Metric | Value |
|--------|-------|
| Total Rust LOC (all crates) | ~128,820 |
| Core `goose` crate LOC | ~75,852 |
| `goose-acp` crate LOC | ~2,568 |
| Builtin agent modes | 16 (7 generalist + 8 SDLC + 1 orchestrator) |
| Server route modules | 23 |
| Registry sources | 4 (local, GitHub, HTTP, A2A) |
| Design RFCs | 4 |

---

## 2. Architecture Overview

The system follows a **3-layer architecture** aligned with the Agent Communication Protocol (ACP):

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Layer 1: Interface                               │
│  ┌──────────┐    ┌──────────────┐    ┌──────────────────┐           │
│  │   CLI    │    │   Desktop    │    │    Web (future)  │           │
│  │  goose   │    │  (Electron)  │    │   (React SPA)    │           │
│  └────┬─────┘    └──────┬───────┘    └────────┬─────────┘           │
│       └────────────┬────┴─────────────────────┘                     │
│                    │  REST / SSE / WebSocket                        │
└────────────────────┼────────────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────────────┐
│                    Layer 2: Middleware (goose-server)               │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                   OrchestratorAgent                            │ │
│  │  Intent Classifier (LLM) → Task Splitter → Result Aggregator   │ │
│  │  Compactor                         · Session Manager           │ │
│  └────────────────────────────────────────────────────────────────┘ │
│  ┌──────────────────┐  ┌──────────────────────────────────────────┐ │
│  │  ServiceBroker   │  │  Agent Discovery (ACP, A2A, registry)    │ │
│  └──────────────────┘  └──────────────────────────────────────────┘ │
│  ┌──────────────────┐  ┌──────────────────────────────────────────┐ │
│  │  AgentSpawner    │  │  Health Monitor (circuit breaker)        │ │
│  └──────────────────┘  └──────────────────────────────────────────┘ │
└────────┬───────────────────────┬──────────────────┬─────────────────┘
         │ in-process            │ ACP/stdio         │ ACP/HTTP or A2A
         ▼                       ▼                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Layer 3: Agents & Services                       │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────────────┐  │
│  │ GooseAgent   │  │ CodingAgent  │  │ External ACP/A2A Agents   │  │
│  │ 7 modes      │  │ 8 SDLC modes │  │ (spawned or remote)       │  │
│  └──────┬───────┘  └──────┬───────┘  └────────────┬──────────────┘  │
│         └─────────┬───────┴───────────────────────┘                 │
│                   │  MCP Extensions (tools/resources)               │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ developer · memory · computercontroller · fetch · context7  │    │
│  │ beads-mcp · custom MCP servers · ACP-MCP adapter bridges    │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Protocol Landscape: ACP, A2A, and MCP

Three complementary protocols form the communication backbone. All are under
**Linux Foundation** governance.

### 3.1 ACP — Agent Communication Protocol (Primary)

| Aspect | Detail |
|--------|--------|
| **Origin** | BeeAI / IBM, first commit April 2025 |
| **Governance** | Linux Foundation, Apache 2.0 |
| **Transport** | RESTful HTTP (OpenAPI 3.1.1, spec v0.2.0) |
| **Paradigm** | Run-centric: create → monitor → resume → cancel |
| **Discovery** | `GET /agents` returns `AgentManifest[]` |
| **Modes** | `sync`, `async`, `stream` (SSE) |
| **Content** | MIME-type based via `MessagePart` — text, images, binary |
| **Offline Discovery** | Metadata embedded in distribution packages |
| **SDKs** | Python (`acp-sdk`), TypeScript, **Rust** (`agent-client-protocol` crate) |
| **Spec URL** | <https://agentcommunicationprotocol.dev> |

**ACP OpenAPI Endpoints:**

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/ping` | Health check |
| `GET` | `/agents` | List agent manifests (paginated) |
| `GET` | `/agents/{name}` | Get single agent manifest |
| `POST` | `/runs` | Create a new run (sync/async/stream) |
| `GET` | `/runs/{run_id}` | Get run status |
| `POST` | `/runs/{run_id}` | Resume an awaiting run |
| `POST` | `/runs/{run_id}/cancel` | Cancel a run |
| `GET` | `/runs/{run_id}/events` | List run events |
| `GET` | `/session/{session_id}` | Get session details |

**ACP AgentManifest (key fields):**

```json
{
  "name": "coding-agent",
  "description": "SDLC specialist with 8 modes",
  "input_content_types": ["text/plain", "application/json"],
  "output_content_types": ["text/plain", "application/json"],
  "metadata": {
    "framework": "goose",
    "capabilities": [
      {"name": "Code Generation", "description": "Generates code from requirements"}
    ],
    "domains": ["software-development"],
    "tags": ["Code", "Orchestrator"],
    "dependencies": [
      {"type": "tool", "name": "developer"},
      {"type": "tool", "name": "memory"}
    ],
    "recommended_models": ["gpt-4o", "claude-sonnet-4"]
  },
  "status": {
    "avg_run_tokens": 2500,
    "avg_run_time_seconds": 12.5,
    "success_rate": 94.2
  }
}
```

**ACP Run Status Lifecycle:**

```
created → in-progress → completed
                      → awaiting → (resume) → in-progress → ...
                      → failed
         → cancelling → cancelled
```

**ACP Event Types (SSE streaming):**

| Event | Payload |
|-------|---------|
| `message.created` | Full `Message` object |
| `message.part` | Single `MessagePart` (streaming chunk) |
| `message.completed` | Finalized `Message` |
| `run.created` | `Run` with initial status |
| `run.in-progress` | `Run` state update |
| `run.awaiting` | `Run` + `AwaitRequest` |
| `run.completed` | `Run` with output |
| `run.failed` | `Run` with error |
| `run.cancelled` | `Run` cancellation confirmed |
| `error` | `Error` payload |

**ACP Rust Crate (`agent-client-protocol` v0.10.8):**

The Rust crate provides full protocol types:

| Category | Key Types |
|----------|-----------|
| Connection | `AgentSideConnection`, `ClientSideConnection` |
| Session | `NewSessionRequest/Response`, `SessionMode`, `SessionId` |
| Content | `TextContent`, `ImageContent`, `AudioContent`, `ContentBlock`, `ContentChunk` |
| Prompting | `PromptRequest/Response`, `PromptCapabilities` |
| Tools | `ToolCall`, `ToolCallUpdate`, `ToolCallLocation`, `ToolKind` |
| MCP | `McpServer` (Stdio/Http/Sse), `McpCapabilities` |
| Permissions | `RequestPermissionRequest/Response`, `PermissionOption` |
| Messaging | `JsonRpcMessage`, `StreamMessage`, `StreamReceiver` |
| Plans | `Plan`, `PlanEntry`, `PlanEntryStatus`, `PlanEntryPriority` |
| Auth | `AuthMethod`, `AuthenticateRequest/Response` |
| Config | `SessionConfigOption`, `ConfigOptionUpdate` |

### 3.2 A2A — Agent-to-Agent Protocol (Discovery)

| Aspect | Detail |
|--------|--------|
| **Origin** | Google, April 2025 |
| **Governance** | Linux Foundation (joined June 2025) |
| **Transport** | JSON-RPC 2.0 |
| **Paradigm** | Task-centric with explicit state machine |
| **Discovery** | `/.well-known/agent-card.json` (AgentCard) |
| **Backing** | 100+ companies: AWS, Microsoft, Salesforce, SAP, Cisco, ServiceNow |
| **Rust Impl** | `a2a-rs` by EmilLindfors |
| **Spec URL** | <https://agent2agent.info> |

**A2A AgentCard Structure:**

```typescript
interface AgentCard {
  name: string;
  description: string;
  url: string;
  provider?: { organization: string; url: string };
  version: string;
  capabilities: {
    streaming?: boolean;
    pushNotifications?: boolean;
    stateTransitionHistory?: boolean;
  };
  authentication: { schemes: string[]; credentials?: string };
  defaultInputModes: string[];   // MIME types
  defaultOutputModes: string[];  // MIME types
  skills: {
    id: string;
    name: string;
    description: string;
    tags: string[];
    examples?: string[];
    inputModes?: string[];
    outputModes?: string[];
  }[];
}
```

**A2A Task State Machine:**

```
submitted → working → completed
                    → input-required → (client provides input) → working
                    → failed
                    → canceled
```

**A2A Artifacts:** Immutable, multi-part output objects. Support streaming via
`append: true`. A single task can produce multiple artifacts (e.g., HTML + images).

### 3.3 MCP — Model Context Protocol (Tool Layer)

| Aspect | Detail |
|--------|--------|
| **Origin** | Anthropic |
| **Purpose** | Standardize LLM ↔ tool/data interactions |
| **Transport** | stdio, Streamable HTTP, SSE |
| **Relationship to ACP** | ACP agents consume MCP tools. ACP Router Agents expose downstream tools via MCP. |

### 3.4 Protocol Comparison & Design Decision

| Dimension       | ACP                          | A2A                            |
|-----------------|------------------------------|--------------------------------|
| Transport       | REST / OpenAPI               | JSON-RPC 2.0                   |
| Unit of work    | **Run** (stateless-friendly) | **Task** (stateful machine)    |
| Discovery       | `/agents` endpoint           | `/.well-known/agent-card.json` |
| Optimized for   | Local-first, low-latency     | Enterprise cross-platform      |
| MCP integration | Native (MCP extension)       | Not built-in                   |
| Content model   | MIME-typed MessageParts      | Multi-part Artifacts           |

**Decision: ACP primary, A2A for discovery.**

Both protocols converge on key concepts (agent manifests, skills, modes, capabilities).
Goose uses ACP as the primary communication protocol for local and stdio-based agents,
while supporting A2A's `/.well-known/agent-card.json` endpoint for broader discovery
and interoperability with the Google/enterprise ecosystem.

### 3.5 ACP Composition Patterns

ACP defines four primary composition patterns for multi-agent systems:

1. **Prompt Chaining** — Sequential processing where each agent builds on the previous
   output. Agent A → Agent B → Agent C.

2. **Routing** — A router agent dynamically selects the best downstream agent based on
   the request content (our OrchestratorAgent implements this).

3. **Parallelization** — Independent tasks processed simultaneously via
   `asyncio.gather()` (or Tokio `join!`).

4. **Hierarchical** — High-level planning agent coordinates specialized execution
   agents. The planner decomposes goals, delegates, and aggregates.

### 3.6 Agent Registry Concepts

Agent registries (centralized or federated) provide:

- **Agent registration** via REST endpoint with AgentCard/manifest payload
- **Discovery/search** by skill, tag, domain, or capability
- **Health monitoring** via periodic heartbeats (e.g., 30s intervals)
- **RBAC policies** for access control
- **Audit logging** for compliance

Goose's `RegistryManager` implements multi-source discovery (local, GitHub, HTTP, A2A)
with priority-based merging and deduplication.

---

## 4. Crate Map & Source Structure

### 4.1 Workspace Overview

```
goose4/
├── Cargo.toml                    # Workspace root (v1.23.0, edition 2021)
├── crates/
│   ├── goose/                    # Core logic (75.8K LOC)
│   │   ├── src/agents/           # Agent loop, modes, routing, extensions
│   │   ├── src/agent_manager/    # Spawner, client, health, task, service broker
│   │   ├── src/registry/         # Manifest, formats, sources, install, publish
│   │   ├── src/providers/        # 30+ LLM provider implementations
│   │   ├── src/conversation/     # Message model, conversation history
│   │   ├── src/session/          # SQLite session persistence (schema v7)
│   │   ├── src/config/           # YAML + env + keyring config system
│   │   ├── src/context_mgmt/     # Auto-compaction at 80% context limit
│   │   ├── src/security/         # Classification, pattern scanning
│   │   └── src/recipe/           # YAML task definitions
│   ├── goose-acp/                # ACP bridge (2.6K LOC)
│   │   ├── src/server.rs         # GooseAcpAgent (1,543 LOC) — full protocol
│   │   ├── src/bridge.rs         # AcpBridge — !Send adapter for Agent trait
│   │   ├── src/server_factory.rs # Factory: config → GooseAcpAgent
│   │   ├── src/transport/        # HTTP + WebSocket transports
│   │   ├── src/notification.rs   # NotificationSender trait + channel impl
│   │   └── src/adapters.rs       # mpsc ↔ AsyncRead/AsyncWrite converters
│   ├── goose-cli/                # CLI entry (18.8K LOC)
│   ├── goose-server/             # HTTP backend "goosed" (12.2K LOC)
│   │   └── src/routes/           # 23 route modules
│   ├── goose-mcp/                # 5 builtin MCP servers (10.9K LOC)
│   ├── goose-test/               # Test utilities
│   ├── goose-test-support/       # MCP test fixtures
│   ├── mcp-client/               # MCP client library
│   ├── mcp-core/                 # MCP shared types
│   └── mcp-server/               # MCP server framework
├── ui/desktop/                   # Electron app (React + TypeScript + Vite)
├── docs/design/                  # 4 RFC documents
│   ├── meta-orchestrator-architecture.md
│   ├── multi-layer-orchestrator.md
│   ├── extension-agent-separation.md
│   └── agent-observability-ux.md
└── services/                     # External services
```

### 4.2 Key File Index

| What                    | Path | LOC |
|-------------------------|------------------------------------------------------|--------|
| Main agent loop         | `crates/goose/src/agents/agent.rs`                   | ~2,000 |
| GooseAgent (7 modes)    | `crates/goose/src/agents/goose_agent.rs`             | ~370   |
| CodingAgent (8 modes)   | `crates/goose/src/agents/coding_agent.rs`            | ~425   |
| OrchestratorAgent       | `crates/goose/src/agents/orchestrator_agent.rs`      | ~750   |
| IntentRouter            | `crates/goose/src/agents/intent_router.rs`           | ~300   |
| ToolFilter              | `crates/goose/src/agents/tool_filter.rs`             | ~210   |
| DelegationStrategy      | `crates/goose/src/agents/delegation.rs`              | ~170   |
| ExtensionManager        | `crates/goose/src/agents/extension_manager.rs`       | ~2,100 |
| ACP Server              | `crates/goose-acp/src/server.rs`                     | ~1,543 |
| ACP Bridge              | `crates/goose-acp/src/bridge.rs`                     | ~76    |
| ACP Transport           | `crates/goose-acp/src/transport.rs`                  | ~127   |
| Registry Manifest       | `crates/goose/src/registry/manifest.rs`              | ~950   |
| Registry Formats        | `crates/goose/src/registry/formats.rs`               | ~820   |
| AgentSpawner            | `crates/goose/src/agent_manager/spawner.rs`          | ~244   |
| AgentHealth             | `crates/goose/src/agent_manager/health.rs`           | ~165   |
| ServiceBroker           | `crates/goose/src/agent_manager/service_broker.rs`   | ~250   |
| TaskManager             | `crates/goose/src/agent_manager/task.rs`             | ~280   |
| Server reply route      | `crates/goose-server/src/routes/reply.rs`            | ~450   |
| Agent management routes | `crates/goose-server/src/routes/agent_management.rs` | ~450   |
| Desktop chat stream     | `ui/desktop/src/hooks/useChatStream.ts`              | ~860   |
| Desktop agents view     | `ui/desktop/src/components/agents/AgentsView.tsx`    | ~516   |

---

## 5. Layer 1 — Interfaces

### 5.1 CLI (`goose-cli`)

- **Entry:** `crates/goose-cli/src/main.rs`
- **Commands:** `session` (interactive), `run` (recipe), `configure`, `info`, `project`,
  `schedule`, `registry`, `update`, `web`
- **Agent commands:** `agents list`, `agents search`, `agents show`, `agents add`, `agents install`
- **Extension flags:** `--with-extension`, `--with-builtin`, `--with-streamable-http`
- **Routing display:** Dim `─── gpt-4o · auto ───` attribution line before responses

### 5.2 Desktop (`ui/desktop`)

- **Stack:** Electron Forge + React + TypeScript + Vite
- **Entry:** `ui/desktop/src/main.ts` (~2.4K LOC)
- **Views:** Hub, Chat (Pair), Settings, Sessions, Schedules, Extensions, Recipes, Agents, Apps
- **API:** Auto-generated from `openapi.json` via `openapi-ts`
- **Key components:**
  - `AgentsView.tsx` — Unified agent browser (builtin + external)
  - `GooseMessage.tsx` — Model attribution badge per message
  - `ToolCallWithResponse.tsx` — Extension → tool prefix display
  - `RoutingIndicator.tsx` — Mode badge, confidence, fallback warning
  - `BottomMenuExtensionSelection.tsx` — Extension enable/disable toggles
- **State management:** `useReducer` with `StreamState` actions (SET_MESSAGES, START_STREAMING, ADD_NOTIFICATION, SESSION_LOADED)
- **SSE pipeline:** Server → Parse → Route/Model → Attach to Message → React dispatch

### 5.3 Web (Future)

Planned as a React SPA consuming the same SSE stream as Desktop.

---

## 6. Layer 2 — Middleware

### 6.1 Goose Server (`goose-server`)

- **Binary:** `goosed` — Axum HTTP server
- **Entry:** `crates/goose-server/src/main.rs`
- **Dual mode:** Agent server (normal) or standalone MCP server (`goosed mcp developer`)
- **AppState:** AgentManager, TunnelManager, RunStore, AgentSlotRegistry

**Route Modules (23):**

| Module                 | Endpoints                                                           |
|------------------------|---------------------------------------------------------------------|
| `agent.rs`             | Agent lifecycle                                                     |
| `agent_card.rs`        | `/.well-known/agent-card.json` (A2A discovery)                      |
| `agent_management.rs`  | Builtin toggle, bind/unbind extensions, external connect/disconnect |
| `reply.rs`             | `POST /reply` — SSE streaming with routing decisions                |
| `session.rs`           | Session CRUD                                                        |
| `config_management.rs` | Configuration API                                                   |
| `recipe.rs`            | Recipe execution                                                    |
| `registry.rs`          | Registry browsing                                                   |
| `runs.rs`              | ACP run management                                                  |
| `schedule.rs`          | Scheduled tasks                                                     |
| `dictation.rs`         | Voice input                                                         |
| `tunnel.rs`            | Tunnel management                                                   |
| `telemetry.rs`         | Telemetry events                                                    |
| `mcp_app_proxy.rs`     | MCP app proxy                                                       |
| `mcp_ui_proxy.rs`      | MCP UI proxy                                                        |
| `prompts.rs`           | Prompt templates                                                    |
| `setup.rs`             | Initial setup flow                                                  |
| `status.rs`            | Server status                                                       |
| `action_required.rs`   | Frontend tool execution requests                                    |
| `errors.rs`            | Error handling                                                      |
| `utils.rs`             | Shared utilities                                                    |

### 6.2 AgentSlotRegistry

```rust
// crates/goose-server/src/agent_slot_registry.rs
pub struct AgentSlotRegistry {
    enabled: HashMap<String, bool>,
    bound_extensions: HashMap<String, Vec<String>>,
}
```

- Tracks which builtin agents are enabled/disabled
- Extension binding: which extensions are bound to which agent
- Thread-safe via `Arc<RwLock<_>>`
- REST API: `POST /agents/builtin/{name}/toggle`, bind/unbind extensions

### 6.3 OrchestratorAgent

The primary routing layer (`orchestrator_agent.rs`, ~750 LOC):

```rust
pub struct OrchestratorPlan {
    pub is_compound: bool,
    pub tasks: Vec<SubTask>,       // Each with RoutingDecision + description
}
```

**Responsibilities:**
1. **LLM-based intent classification** — uses the configured provider
2. **Compound request splitting** — detects "fix login AND add dark theme" as 2 tasks
3. **Compaction ownership** — proactive context compaction before delegation
4. **Catalog building** — registers GooseAgent + CodingAgent modes into text catalog

**Feature flag:** Enabled by default. Set `GOOSE_ORCHESTRATOR_DISABLED=true` to fall
back to keyword-only routing.

**Current limitation:** While compound splitting is parsed, only the primary (first)
task is currently executed. Parallel multi-task execution is future work.

---

## 7. Layer 3 — Agents & Services

### 7.1 GooseAgent — 7 Generalist Modes

```rust
// crates/goose/src/agents/goose_agent.rs
pub struct GooseAgent {
    modes: HashMap<String, BuiltinMode>,
    default_mode: String,  // "assistant"
}
```

| Mode            | Category   | Prompt              | Tool Groups | Purpose                        |
|-----------------|------------|---------------------|-------------|--------------------------------|
| `assistant`     | Session    | system.md           | `mcp` (all) | Default personality            |
| `specialist`    | Session    | specialist.md       | scoped      | Bounded task execution         |
| `recipe_maker`  | PromptOnly | recipe.md           | `none`      | Recipe generation              |
| `app_maker`     | LlmOnly    | apps_create.md      | `apps`      | Generate new apps              |
| `app_iterator`  | LlmOnly    | apps_iterate.md     | `apps`      | Update existing apps           |
| `judge`         | LlmOnly    | permission_judge.md | `none`      | Read-only permission analysis  |
| `planner`       | PromptOnly | plan.md             | `none`      | Step-by-step planning          |

**Mode Categories:**
- **Session** — Overrides the main agent's system prompt
- **LlmOnly** — Direct `provider.complete()` with specialized prompt (no tool loop)
- **PromptOnly** — Returns rendered template string (no LLM call)

### 7.2 CodingAgent — 8 SDLC Modes

```rust
// crates/goose/src/agents/coding_agent.rs
pub struct CodingAgent {
    modes: HashMap<String, CodingMode>,
    default_mode: String,  // "backend"
}
```

| Mode        | Role               | Tool Groups                            | Security              |
|-------------|--------------------|----------------------------------------|-----------------------|
| `pm`        | Product Manager    | memory, fetch, **read**                | Read-only             |
| `architect` | Software Architect | developer, memory, fetch, read         | Can read, not execute |
| `backend`   | Backend Engineer   | developer, edit, command, mcp, memory  | **Full write**        |
| `frontend`  | Frontend Engineer  | developer, edit, command, browser, mcp | Write + browser       |
| `qa`        | Quality Assurance  | developer, command, browser, read      | Run tests             |
| `security`  | Security Champion  | developer, **read**, fetch, memory     | **No edit/command**   |
| `sre`       | Site Reliability   | developer, command, fetch, read        | Execute commands      |
| `devsecops` | DevSecOps          | developer, edit, command, mcp          | Full CI/CD            |

Each mode has `recommended_extensions` (e.g., backend recommends `developer`,
`github`, `jetbrains`) and `when_to_use` hints for the router.

### 7.3 Extension System (MCP)

| Type               | Transport                    | Examples                              |
|--------------------|------------------------------|---------------------------------------|
| **Builtin**        | DuplexStream (in-process)    | developer, memory, computercontroller |
| **External stdio** | spawn process + stdin/stdout | Custom MCP servers                    |
| **External HTTP**  | Streamable HTTP              | Remote MCP endpoints                  |
| **Platform**       | In-process Rust              | todo, apps, chatrecall, summon, tom   |

**Builtin MCP Servers (`goose-mcp`, 10.9K LOC):**

| Server                       | Key Tools                                                        |
|------------------------------|------------------------------------------------------------------|
| `DeveloperServer` (3.6K LOC) | `shell`, `text_editor` (6 commands), `analyze`, `screen_capture` |
| `MemoryServer`               | Persistent knowledge storage                                     |
| `ComputerControllerServer`   | Web scraping, automation, docx/pdf/xlsx tools                    |
| `AutoVisualiserRouter`       | Visualization                                                    |
| `TutorialServer`             | Guided tutorials                                                 |

---

## 8. The ACP Bridge (`goose-acp` Crate)

The `goose-acp` crate makes Goose available as a remote ACP-compatible agent.

### 8.1 Module Structure

```
crates/goose-acp/
├── src/
│   ├── lib.rs              # Module declarations
│   ├── server.rs           # GooseAcpAgent — 1,543 LOC protocol implementation
│   ├── bridge.rs           # AcpBridge — !Send adapter for Agent trait
│   ├── server_factory.rs   # AcpServerFactory — config → GooseAcpAgent
│   ├── notification.rs     # NotificationSender trait + channel impl
│   ├── adapters.rs         # mpsc ↔ AsyncRead/AsyncWrite converters
│   ├── transport.rs        # Router: HTTP + WebSocket
│   ├── transport/http.rs   # HTTP transport (POST/GET/DELETE /acp)
│   ├── transport/websocket.rs # WebSocket transport
│   └── bin/server.rs       # Standalone binary: goose-acp-server
└── tests/
    ├── server_test.rs      # Integration tests
    ├── fixtures/            # Test server setup
    └── common_tests/        # Shared test utilities
```

### 8.2 GooseAcpAgent

```rust
pub struct GooseAcpAgent {
    sessions: Arc<Mutex<HashMap<String, GooseAcpSession>>>,
    provider_factory: ProviderConstructor,
    session_manager: Arc<SessionManager>,
    permission_manager: Arc<PermissionManager>,
    modes: Vec<AgentMode>,           // 16 combined modes
    default_mode: Option<String>,    // "assistant"
    notification_sender: Arc<RwLock<Option<Arc<dyn NotificationSender>>>>,
    // ...
}
```

**ACP ↔ Goose mappings:**

| ACP Concept           | Goose Implementation                          |
|-----------------------|-----------------------------------------------|
| `McpServer::Stdio`    | `ExtensionConfig::Stdio`                      |
| `McpServer::Http`     | `ExtensionConfig::StreamableHttp`             |
| `SessionMode`         | `AgentMode` (from GooseAgent/CodingAgent)     |
| `ContentBlock::Text`  | `Message::with_text()`                        |
| `ToolCallUpdate`      | `ToolRequest` + `ToolResponse` with locations |
| `SessionNotification` | `AgentEvent` stream → notification sender     |

**Protocol operations implemented:**

| ACP Method          | Handler                                               |
|---------------------|-------------------------------------------------------|
| `initialize`        | `on_initialize()` — capabilities, modes, model info   |
| `new_session`       | `on_new_session()` — create Agent + extensions        |
| `load_session`      | `on_load_session()` — restore from SessionManager     |
| `prompt`            | `on_prompt()` — run agent reply loop, stream content  |
| `cancel`            | `on_cancel()` — cancellation token                    |
| `set_session_mode`  | `on_set_mode()` — switch behavioral mode              |
| `set_session_model` | `on_set_model()` — switch LLM model                   |

### 8.3 AcpBridge

```rust
pub struct AcpBridge {
    pub agent: Arc<GooseAcpAgent>,
}

#[async_trait(?Send)]
impl agent_client_protocol::Agent for AcpBridge {
    // Delegates all methods to GooseAcpAgent
    // Lives on a LocalSet (not Send)
}
```

### 8.4 Transport Layer

The transport router handles both HTTP and WebSocket on the same `/acp` endpoint:

```rust
Router::new()
    .route("/health", get(health))
    .route("/acp", post(http::handle_post))
    .route("/acp", get(handle_get))       // HTTP SSE or WebSocket upgrade
    .route("/acp", delete(http::handle_delete))
    .layer(cors)
```

- **HTTP:** JSON-RPC over POST, SSE streaming on GET
- **WebSocket:** Bidirectional JSON-RPC framing
- **Session ID:** Carried in `Acp-Session-Id` header

### 8.5 Tool Location Extraction

The ACP server extracts file locations from `developer__text_editor` results for IDE
integration:

```rust
fn extract_tool_locations(
    tool_request: &ToolRequest,
    tool_response: &ToolResponse,
) -> Vec<ToolCallLocation>
```

Parses `view`, `str_replace`, `insert`, `write` commands to create
`ToolCallLocation(path, line)`.

### 8.6 Dependencies

```toml
agent-client-protocol = { version = "0.9.4", features = ["unstable"] }
agent-client-protocol-schema = { version = "0.10", features = ["unstable_session_model"] }
```

---

## 9. Agent Manager Subsystem

Located in `crates/goose/src/agent_manager/`:

### 9.1 AgentSpawner

```rust
pub async fn spawn_agent(dist: &AgentDistribution) -> Result<SpawnedAgent>
```

Tries distribution strategies in priority order:
1. **Binary** — Platform-specific (`darwin-aarch64`, `linux-x86_64`, etc.)
2. **npx** — Node.js package runner
3. **uvx** — Python package runner
4. **cargo** — Rust `cargo run`
5. **docker** — Container execution

Returns `SpawnedAgent { child, stdin, stdout }` for ACP communication.
Graceful shutdown with 5-second kill timeout.

### 9.2 AgentClientManager

```rust
pub struct AgentClientManager {
    // Command-channel pattern: AgentCommand via mpsc, responses via oneshot
}
```

Operations: `connect_with_distribution`, `new_session`, `prompt_agent_text`,
`set_mode`, `disconnect_agent`, `shutdown_all`.

### 9.3 AgentHealth (Circuit Breaker)

```rust
pub enum AgentState { Healthy, Degraded, Dead }

pub struct AgentHealth {
    consecutive_failures: AtomicU32,
    max_failures_before_degraded: u32,  // default: 3
    max_failures_before_dead: u32,      // default: 10
    stale_timeout: Duration,            // default: 300s
}
```

State transitions:
- `record_success()` → reset failures, update activity timestamp
- `record_failure()` → increment failures
- `state()` → check failures + staleness → Healthy/Degraded/Dead

### 9.4 TaskManager (A2A Task Lifecycle)

```
Submitted → Working → Completed
                    → Failed
                    → Canceled
                    → InputRequired → (client input) → Working
```

UUID-based task tracking with timestamps.

### 9.5 ServiceBroker (Manifest-Driven Wiring)

```rust
pub struct ServiceBroker {
    loaded_extensions: HashSet<String>,
    session_extensions: HashMap<String, ExtensionConfig>,
}
```

Resolution order:
1. Already loaded in session → skip
2. Platform extension → load from `PLATFORM_EXTENSIONS`
3. Builtin extension → load from builtin registry
4. Session config → load from user's extension config
5. Unresolved → report as missing

### 9.6 ACP-MCP Adapter

`crates/goose/src/agent_manager/acp_mcp_adapter.rs` (~213 LOC)

Bridges ACP agents as MCP tools and vice versa, enabling bidirectional
interoperability between the two protocols.

---

## 10. Registry System

Located in `crates/goose/src/registry/`:

### 10.1 RegistryEntry (Superset Schema)

```rust
pub struct RegistryEntry {
    pub name: String,
    pub kind: RegistryEntryKind,  // Tool | Skill | Agent | Recipe
    pub description: String,
    pub version: Option<String>,
    pub author: Option<AuthorInfo>,
    pub tags: Vec<String>,
    pub detail: RegistryEntryDetail,  // Kind-specific payload
    pub metadata: HashMap<String, String>,
    // ...
}
```

For agents, `RegistryEntryDetail::Agent(AgentDetail)` includes:
- `modes: Vec<AgentMode>` — behavioral modes with tool groups
- `skills: Vec<AgentSkill>` — structured capability descriptors
- `distribution: Option<AgentDistribution>` — how to spawn
- `security: Option<SecurityScheme>` — auth requirements
- `default_mode: Option<String>`
- `dependencies: Vec<AgentDependency>`

### 10.2 AgentMode

```rust
pub struct AgentMode {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub instructions: Option<String>,
    pub tool_groups: Vec<ToolGroupAccess>,
    pub when_to_use: String,
}
```

Maps 1:1 to ACP's `SessionMode`.

### 10.3 Formats Module (Bidirectional Conversion)

| Direction | From             | To                     |
|-----------|------------------|------------------------|
| Parse     | ACP `agent.json` | `RegistryEntry`        |
| Generate  | `RegistryEntry`  | ACP `agent.json`       |
| Generate  | `RegistryEntry`  | A2A `agent-card.json`  |

### 10.4 Registry Sources

| Source     | File                       | Discovery Mechanism                                      |
|------------|----------------------------|----------------------------------------------------------|
| **Local**  | `sources/local.rs` (583L)  | Scan `~/.config/goose/` for skills, agents, recipes      |
| **A2A**    | `sources/a2a.rs` (179L)    | Fetch `.well-known/agent.json` from configured endpoints |
| **GitHub** | `sources/github.rs` (403L) | Fetch from GitHub repos                                  |
| **HTTP**   | `sources/http.rs` (358L)   | Generic HTTP registry endpoint                           |

### 10.5 Install & Publish

- **Install** (`install.rs`, 390L): Download → verify → extract agent distributions
- **Publish** (`publish.rs`, 417L): Package → validate → upload to registries

---

## 11. Routing & Intent Classification

### 11.1 Two-Tier Routing Architecture

```
User Message
    │
    ▼
┌─ OrchestratorAgent ─────────────────────────┐
│                                              │
│  1. LLM Classification (splitting.md prompt) │
│     └─ confidence ≥ 0.5? ──YES──► return     │
│                     │                        │
│                    NO                        │
│                     │                        │
│  2. IntentRouter (keyword fallback)          │
│     └─ mark fell_back=true                   │
│     └─ return                                │
└──────────────────────────────────────────────┘
    │
    ▼
  RoutingDecision → Agent/Mode → filtered tools → reply
```

### 11.2 IntentRouter (Keyword Fallback)

```rust
pub struct IntentRouter {
    slots: Vec<AgentSlot>,  // GooseAgent + CodingAgent
}
```

**Scoring algorithm:**

| Factor | Weight | Mechanism |
|--------|--------|-----------|
| `when_to_use` keywords | 60% | Fuzzy match against mode hints |
| `description` keywords | 30% | Fuzzy match against description |
| Mode `name` | 10% | Substring match in message |

**Fuzzy matching:** Prefix matching (≥3 chars). "implement" matches "implementation".
Shared prefix ≥4 chars covering most of the shorter word.

**Threshold:** Score ≥ 0.2 triggers routing. Below → default agent (assistant mode).

### 11.3 RoutingDecision

```rust
pub struct RoutingDecision {
    pub agent_name: String,   // "Coding Agent"
    pub mode_slug: String,    // "backend"
    pub confidence: f32,      // 0.85
    pub reasoning: String,    // "API implementation task"
}
```

### 11.4 Compound Request Splitting

The `splitting.md` prompt instructs the LLM to:

```json
{
  "is_compound": true,
  "tasks": [
    {"agent_name": "Coding Agent", "mode_slug": "backend", "confidence": 0.9,
     "reasoning": "API endpoint", "sub_task": "Fix the login bug"},
    {"agent_name": "Coding Agent", "mode_slug": "frontend", "confidence": 0.8,
     "reasoning": "UI theming", "sub_task": "Add dark theme"}
  ]
}
```

---

## 12. Tool Filtering & Extension Scoping

### 12.1 The 4-Stage Filtering Pipeline

```
All tools from ExtensionManager
    │
    ├── 1. Code Execution Filter
    │   └─ If code_execution active: keep only first-class extension tools
    │
    ├── 2. Mode-Based Tool Groups (from RoutingDecision)
    │   └─ ToolGroupAccess::Full("developer") → keep developer tools
    │   └─ ToolGroupAccess::Full("none") → keep nothing
    │   └─ ToolGroupAccess::Full("mcp") → keep ALL tools
    │
    ├── 3. Scope-Based Filtering
    │   └─ If NOT orchestrator context:
    │      hide orchestrator-only tools (summon, extensionmgr, chatrecall, tom)
    │
    ├── 4. Allowed Extensions Filter
    │   └─ Retain only tools from recommended_extensions list
    │
    └── Sort alphabetically (stable prompt caching)
```

### 12.2 Tool Group Mapping

| Group Name | Matches |
|------------|---------|
| `mcp` | **All tools** (wildcard) |
| `none` | **No tools** |
| `developer` | Tools owned by `developer` extension |
| `memory` | Tools owned by `memory` extension |
| `command` | `developer__shell`, `developer__terminal`, etc. |
| `edit` | `developer__text_editor`, `developer__write`, etc. |
| `read` | `developer__read_file`, `developer__list_directory`, etc. |
| `fetch` | Tools containing "fetch" or "http" |
| `browser` | `computercontroller` tools |
| `orchestrator` | summon, extensionmanager, chatrecall, tom |

---

## 13. Extension-Agent Separation (ACP-Aligned)

### 13.1 The Problem (Anti-Patterns)

1. **All extensions loaded for all modes** — judge mode (read-only) has shell access
2. **Orchestration concerns as extensions** — summon, extensionmanager, chatrecall, tom
3. **No manifest-driven wiring** — extensions loaded from config, not agent declarations

### 13.2 Extension Classification

| Extension                                                       | Scope                 | Should Belong To   |
|-----------------------------------------------------------------|-----------------------|--------------------|
| developer, memory, computercontroller, autovisualiser, tutorial | **MCP Service**       | Extension pool     |
| summon, extensionmanager, chatrecall, tom                       | **Orchestrator**      | OrchestratorAgent  |
| apps, todo                                                      | **Agent-specific**    | Bound via manifest |
| code_execution                                                  | **Meta-optimization** | Owning agent       |

### 13.3 Implementation (4 Phases — All Complete)

| Phase | What                           | Key Change                                               |
|-------|--------------------------------|----------------------------------------------------------|
| **1** | GooseAgent `tool_groups`       | Security fix: judge → `none`, app_maker → `apps`         |
| **2** | Manifest-based binding         | `recommended_extensions` wired into `reply.rs`           |
| **3** | Extension scope classification | `Orchestrator`/`AgentSpecific` scopes                    |
| **4** | Scope-based filtering          | Hide orchestrator tools when not in orchestrator context |

---

## 14. Agent Observability & UX

### 14.1 The Data Pipeline

```
Agent (Rust)                    Server (SSE)              UI (React/CLI)
─────────────                   ────────────              ─────────────
AgentEvent::Message         →   MessageEvent::Message     → ✅ Rendered
AgentEvent::ModelChange     →   MessageEvent::ModelChange → ✅ Attribution badge
AgentEvent::McpNotification →   MessageEvent::Notification→ ✅ Progress bars
AgentEvent::RoutingDecision →   MessageEvent::Routing     → ✅ Mode badge
AgentEvent::HistoryReplaced →   MessageEvent::UpdateConv  → ✅ Applied
```

### 14.2 UI Components

**Desktop:**
- Model attribution badge on every assistant message
- Extension name prefix (`developer › shell`) in tool call headers
- Routing indicator: agent name + mode + confidence + fallback warning
- ChatGPT-style collapsible reasoning/thinking panel
- Clean response style (hide tool call panels)
- Agent count in bottom menu bar

**CLI:**
- Dim `─── gpt-4o · auto ───` attribution line before responses
- Routing indicator inline

### 14.3 TypeScript Types

```typescript
interface RoutingInfo {
  agentName: string;   // "Coding Agent"
  modeSlug: string;    // "backend"
  confidence: number;  // 0.85
  reasoning: string;   // "API implementation task"
}

type MessageWithAttribution = Message & {
  _modelInfo?: ModelAttribution;
  _routingInfo?: RoutingInfo;
};
```

---

## 15. Data Flow: End-to-End Request Lifecycle

```
User submits message (Desktop or CLI)
    │
    ▼
POST /reply { user_message, session_id }
    │
    ▼
┌─ goose-server/routes/reply.rs ──────────────────────────────┐
│  1. Sync AgentSlotRegistry (enabled agents + bound exts)     │
│  2. Create OrchestratorAgent                                 │
│  3. OrchestratorAgent.route(user_text)                       │
│     ├─ LLM classifier → RoutingDecision (or compound plan)  │
│     └─ Fallback: IntentRouter keyword matching               │
│  4. Apply routing: set_active_tool_groups(), allowed_exts    │
│  5. Emit SSE: RoutingDecision event                          │
│  6. Agent.reply(message, config, cancel_token)               │
└──────────────────────────────────────────────────────────────┘
    │
    ▼
┌─ goose/agents/agent.rs — reply_internal() ──────────────────┐
│  LOOP (max 1000 turns):                                      │
│    a. inject_moim() — contextual hints                       │
│    b. filter tools by mode groups + allowed extensions       │
│    c. stream_response_from_provider()                        │
│       → Provider.stream() or .complete()                     │
│       → ONE LLM call per turn                                │
│    d. Categorize tool requests:                              │
│       → frontend_requests (UI tools)                         │
│       → remaining_requests (MCP tools)                       │
│    e. Permission check (auto-approve / ask / deny)           │
│    f. dispatch_tool_call() → ExtensionManager                │
│       → resolve_tool() → correct MCP extension               │
│    g. Collect tool responses                                 │
│    h. yield AgentEvent::Message                              │
│    i. Loop back to (a) if more tool calls                    │
└──────────────────────────────────────────────────────────────┘
    │
    ▼
SSE stream → Desktop/CLI renders incrementally
    │
    ▼
SSE: Finish { reason: "stop" }
```

---

## 16. Design Decisions & ADRs

### ADR-1: ACP as Primary Protocol

**Context:** Both ACP and A2A emerged in April 2025 under Linux Foundation governance.

**Decision:** Use ACP for all agent communication (local + remote). Use A2A only for
discovery (`/.well-known/agent-card.json`).

**Rationale:**
- ACP is REST-native (aligns with goose-server's Axum stack)
- ACP has a mature Rust crate (`agent-client-protocol`)
- ACP's Run model maps naturally to goose's session/reply loop
- A2A's JSON-RPC adds protocol complexity without clear local-first benefit

### ADR-2: LLM-Based Routing with Keyword Fallback

**Decision:** OrchestratorAgent uses LLM classification (primary) with IntentRouter
keywords (fallback when LLM confidence < 0.5 or when disabled).

**Rationale:** Keywords are brittle — "implement a REST API" doesn't match "backend"
keyword patterns. LLM understands context, domain, and compound requests.

### ADR-3: Extension-Agent Separation

**Decision:** Extensions are services requested via manifest. Orchestration tools
(summon, extensionmanager, chatrecall, tom) belong to the OrchestratorAgent, not the
global extension pool.

**Rationale:** ACP mandates that agents declare dependencies. A judge mode shouldn't
have shell access. Tool visibility must be enforced structurally, not by prompting.

### ADR-4: Delegation Strategy

**Decision:** Two delegation paths:
1. `InProcessSpecialist` — same process, shared provider
2. `ExternalAcpAgent` — spawned process, ACP over stdio

**Rationale:** In-process is fast for simple tasks. External is necessary for agents
with different runtimes, models, or extensions.

### ADR-5: Stay in One Crate (Module Boundaries)

**Decision:** GooseAgent and CodingAgent remain in the `goose` crate with module
boundaries, not separate crates.

**Rationale:** Both share `Agent`, `Provider`, `ExtensionManager` types. Separate
crates would require extensive trait extraction. Split when an agent needs a different
runtime (Python, container).

---

## 17. Testing Strategy

| Layer         | Approach                         | Location                                    |
|---------------|----------------------------------|---------------------------------------------|
| Core agent    | Integration tests                | `crates/goose/tests/agent.rs`               |
| Compaction    | Unit + integration               | `crates/goose/tests/compaction.rs`          |
| MCP           | Analyze, diff, language tests    | `crates/goose-mcp/`                         |
| ACP           | Server integration with fixtures | `crates/goose-acp/tests/server_test.rs`     |
| Providers     | Scenario tests                   | `crates/goose-cli/scenario_tests/`          |
| Routing       | Unit (keyword + LLM + composite) | `crates/goose/src/agents/intent_router.rs`  |
| Desktop       | Vitest unit + Playwright E2E     | `ui/desktop/`                               |
| MCP Recording | Replay-based testing             | `just record-mcp-tests`                     |
| Self-test     | Recipe-based validation          | `goose-self-test.yaml`                      |

**Test counts:** ~114 total (100 goose unit + 11 integration + 3 server).

---

## 18. Risks & Open Items

| Risk                                        | Severity | Status   | Notes                                  |
|---------------------------------------------|----------|----------|----------------------------------------|
| Compound execution not wired                | Medium   | Open     | Only primary task executed             |
| Router ↔ SlotRegistry sync                  | Medium   | Open     | Two sources of truth for enabled state |
| Hardcoded agent names                       | Low      | Open     | `["Goose Agent", "Coding Agent"]`      |
| ACP modes lose tool_groups                  | Low      | Open     | AgentMode → ModeInfo drops tool groups |
| IntentRouter creates new agents per route() | Low      | Open     | Could cache                            |
| ModelChange events in CLI                   | Low      | Fixed    | Now shows attribution                  |
| Tool group string-based matching            | Low      | Accepted | No compile-time validation             |

---

## 19. References & Sources

### Protocol Specifications

- ACP Specification: <https://agentcommunicationprotocol.dev>
- ACP OpenAPI Spec (YAML): <https://raw.githubusercontent.com/i-am-bee/acp/refs/heads/main/docs/spec/openapi.yaml>
- ACP Agent Manifest: <https://agentcommunicationprotocol.dev/core-concepts/agent-manifest>
- ACP Architecture: <https://agentcommunicationprotocol.dev/core-concepts/architecture>
- ACP Compose Agents: <https://agentcommunicationprotocol.dev/how-to/compose-agents>
- ACP Wrap Existing Agent: <https://agentcommunicationprotocol.dev/how-to/wrap-existing-agent>
- A2A Protocol: <https://agent2agent.info>
- A2A AgentCard: <https://agent2agent.info/docs/concepts/agentcard/>
- A2A Task: <https://agent2agent.info/docs/concepts/task/>
- A2A Artifact: <https://agent2agent.info/docs/concepts/artifact/>

### Rust Crates

- `agent-client-protocol`: <https://docs.rs/agent-client-protocol/latest/agent_client_protocol/>
- `agent-client-protocol` SessionMode: <https://docs.rs/agent-client-protocol/latest/agent_client_protocol/struct.SessionMode.html>

### Articles & Analysis

- Linux Foundation A2A Announcement: <https://www.linuxfoundation.org/press/linux-foundation-launches-the-agent2agent-protocol-project-to-enable-secure-intelligent-communication-between-ai-agents>
- GoCodeo ACP Analysis: <https://www.gocodeo.com/post/acp-the-protocol-standard-for-ai-agent-interoperability>
- Towards Data Science — ACP in Practice: <https://towardsdatascience.com/the-future-of-ai-agent-communication-with-acp/>
- TrueFoundry Agent Registry: <https://www.truefoundry.com/blog/ai-agent-registry>
- A2A Server Implementations: <https://www.a2aprotocol.org/fr/resources/types/server>

### Internal Design Documents

- `docs/design/meta-orchestrator-architecture.md` — Full RFC: target architecture, phased plan
- `docs/design/multi-layer-orchestrator.md` — 3-layer architecture RFC
- `docs/design/extension-agent-separation.md` — ACP-aligned extension scoping
- `docs/design/agent-observability-ux.md` — UX proposal for transparency

---

*Last updated: 2026-02-14. Generated from source code analysis, protocol specifications,
design documents, and the goose_tmp vibe-coding session log.*
