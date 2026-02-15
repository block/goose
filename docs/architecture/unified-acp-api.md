# Unified API Architecture — ACP-Superset Server

## Two ACPs, One Server

There are **two different protocols both called "ACP"** in the agent ecosystem:

| | ACP-REST (Agent Communication Protocol) | ACP-IDE (Agent Client Protocol) |
|---|---|---|
| **Full name** | Agent Communication Protocol | Agent Client Protocol |
| **Purpose** | Agent ↔ Agent / App ↔ Agent interop | IDE/Editor ↔ Coding Agent |
| **Transport** | REST/HTTP + SSE streaming | JSON-RPC 2.0 (stdio/HTTP/WebSocket) |
| **Spec** | agentcommunicationprotocol.dev | agentclientprotocol.com |
| **Key concepts** | Runs, Sessions, AgentManifest, Events | Sessions, Modes, Capabilities |
| **Relation** | Now part of A2A (Linux Foundation) | Separate project |

**goosed implements BOTH as a superset** — a single Axum server with three entrypoints.

## Architecture: Single Server, Three Entrypoints

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              INTERFACES                                     │
│                                                                             │
│  ┌──────────┐    ┌──────────────┐    ┌──────────┐    ┌───────────────────┐ │
│  │   CLI    │    │   Desktop    │    │   IDE    │    │  External ACP    │ │
│  │          │    │  (Electron)  │    │ (VSCode) │    │  Agents          │ │
│  └────┬─────┘    └──────┬───────┘    └────┬─────┘    └────────┬─────────┘ │
│       │                 │                 │                    │           │
│       │ HTTP/SSE        │ HTTP/SSE        │ JSON-RPC           │ HTTP/SSE  │
│       │ (GoosedClient)  │ (fetch)         │ (WebSocket/HTTP)   │ (REST)    │
└───────┴─────────────────┴─────────┬───────┴────────────────────┘           │
                                    │                                         │
════════════════════════════════════╪═════════════════════════════════════════╡
                                    │                                         │
┌───────────────────────────────────▼─────────────────────────────────────────┐
│                    goosed (single Axum server)                               │
│                                                                             │
│  ┌─── Entrypoint 1: Goose-native REST ──┐                                  │
│  │  POST /reply         (SSE stream)     │  ← CLI, Desktop, Web            │
│  │  GET  /sessions/*    (CRUD)           │                                  │
│  │  POST /agent/*       (tools, prompts) │                                  │
│  └───────────────────────────────────────┘                                  │
│                                                                             │
│  ┌─── Entrypoint 2: ACP-REST ───────────┐                                  │
│  │  GET  /ping                           │  ← External agents, ACP clients │
│  │  GET  /agents                         │                                  │
│  │  POST /runs          (SSE stream)     │                                  │
│  │  POST /runs/{id}/cancel               │                                  │
│  │  GET  /runs/{id}/events               │                                  │
│  │  GET  /session/{id}                   │                                  │
│  └───────────────────────────────────────┘                                  │
│                                                                             │
│  ┌─── Entrypoint 3: ACP-IDE ────────────┐                                  │
│  │  POST /acp    (JSON-RPC over HTTP)    │  ← VS Code, IDE extensions      │
│  │  GET  /acp    (WebSocket → JSON-RPC)  │                                  │
│  │  DELETE /acp  (session cleanup)       │                                  │
│  │                                       │                                  │
│  │  Methods:                             │                                  │
│  │    initialize, new_session,           │                                  │
│  │    load_session, prompt, cancel,      │                                  │
│  │    set_session_mode, set_session_model│                                  │
│  └───────────────────────────────────────┘                                  │
│                                                                             │
│  ┌─── Shared Core ──────────────────────────────────────────────────────┐  │
│  │                                                                       │  │
│  │  AppState                                                             │  │
│  │    ├─ AgentManager → Agent.reply()    (one agent per session)        │  │
│  │    ├─ SessionManager                  (shared state store)           │  │
│  │    ├─ RunStore                         (ACP-REST run tracking)       │  │
│  │    ├─ AcpIdeSessions                   (ACP-IDE session tracking)    │  │
│  │    ├─ AgentSlotRegistry               (modes/extensions)             │  │
│  │    └─ ActionRequiredManager           (elicitation/permissions)      │  │
│  │                                                                       │  │
│  │  goose::acp_compat (shared adapter layer)                             │  │
│  │    ├─ message.rs   Message ⟷ AcpMessage converters                   │  │
│  │    ├─ events.rs    AgentEvent → ACP SSE events                       │  │
│  │    ├─ manifest.rs  AgentManifest from IntentRouter                   │  │
│  │    └─ types.rs     AcpRun, RunMode, AwaitRequest, etc.              │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Why Single Server

1. **Shared state** — All three entrypoints use the same Agent, SessionManager, RunStore.
   No need to synchronize across processes.

2. **Single port** — IDEs, CLIs, and external agents all connect to localhost:PORT.

3. **Shared session** — An IDE can start a coding session via ACP-IDE (JSON-RPC), and the
   CLI can inspect/resume it via ACP-REST or /sessions. Same conversation, same agent.

4. **Mode is just dispatch** — Whether the request comes as:
   - `POST /runs { agent_name: "backend" }` (ACP-REST)
   - `set_session_mode("backend")` (ACP-IDE)
   - Orchestrator auto-routing (goose-native)
   
   It all resolves to the same internal mode application.

## ACP-REST Endpoint Mapping (Agent Communication Protocol)

| ACP-REST Endpoint | goosed Route | Implementation |
|---|---|---|
| `GET /ping` | `/ping` | `acp_discovery.rs` |
| `GET /agents` | `/agents` | `acp_discovery.rs` |
| `GET /agents/{name}` | `/agents/{name}` | `acp_discovery.rs` |
| `POST /runs` (stream/sync/async) | `/runs` | `runs.rs` → Agent.reply() |
| `GET /runs/{run_id}` | `/runs/{run_id}` | `runs.rs` |
| `POST /runs/{run_id}` (resume) | `/runs/{run_id}` | `runs.rs` → ActionRequiredManager |
| `POST /runs/{run_id}/cancel` | `/runs/{run_id}/cancel` | `runs.rs` → CancellationToken |
| `GET /runs/{run_id}/events` | `/runs/{run_id}/events` | `runs.rs` (persisted in RunStore) |
| `GET /session/{session_id}` | `/session/{session_id}` | `acp_discovery.rs` (ACP schema) |

## ACP-IDE Method Mapping (Agent Client Protocol)

| ACP-IDE Method | Transport | Implementation |
|---|---|---|
| `initialize` | POST/WS | Creates session, returns capabilities + modes |
| `new_session` | POST/WS | Creates fresh session |
| `load_session` | POST/WS | Loads existing session from SessionManager |
| `prompt` | POST/WS | Delegates to Agent.reply(), streams notifications |
| `cancel` | Notification | Triggers CancellationToken |
| `set_session_mode` | POST/WS | Resolves mode, applies tool_groups + instructions |
| `set_session_model` | POST/WS | (stub — model switching planned) |

## Message Format Conversion

| Internal (goose) | ACP-REST (MessagePart) | ACP-IDE (JSON-RPC notification) |
|---|---|---|
| `MessageContent::Text` | `{ content_type: "text/plain", content: "..." }` | `session/update { type: "text", text: "..." }` |
| `MessageContent::Image` | `{ content_type: "image/png", content_encoding: "base64" }` | (not yet mapped) |
| `MessageContent::ToolRequest` | `{ content_type: "application/json", metadata: trajectory }` | `session/update { type: "tool_call", ... }` |
| `MessageContent::ToolResponse` | `{ content_type: "application/json", metadata: trajectory }` | `session/update { type: "tool_result", ... }` |
| `MessageContent::Thinking` | `{ content_type: "text/plain", metadata: { thinking: true } }` | `session/update { type: "thinking", ... }` |

## SSE Event Mapping (ACP-REST only)

| goosed Internal Event | ACP-REST SSE Event(s) |
|---|---|
| `Message { message }` | `message.created` + N × `message.part` + `message.completed` |
| `Error { error }` | `error` |
| `Finish { reason }` | `run.completed` or `run.failed` |
| `ModelChange` | `generic` (goose-specific) |
| `RoutingDecision` | `generic` (goose-specific) |
| `PlanProposal` | `generic` (goose-specific) |
| *(stream start)* | `run.created` + `run.in-progress` |
| *(ActionRequired)* | `run.awaiting` |
| *(cancel_token)* | `run.cancelled` |

## Run Lifecycle State Machine (ACP-REST)

```
                    POST /runs
                        │
                        ▼
                  ┌──────────┐
                  │ created  │
                  └────┬─────┘
                       │
                       ▼
                ┌─────────────┐
            ┌───│ in_progress │───┐
            │   └──────┬──────┘   │
            │          │          │
     ActionRequired    │     stream error
            │          │          │
            ▼          │          ▼
      ┌──────────┐     │    ┌─────────┐
      │ awaiting │     │    │ failed  │
      └────┬─────┘     │    └─────────┘
           │           │
    POST /runs/{id}    │
      (resume)         │
           │           │
           ▼           ▼
      ┌──────────┐  ┌───────────┐
      │ resumed  │  │ completed │
      │→in_prog  │  └───────────┘
      └──────────┘
                    POST /runs/{id}/cancel
                           │
                           ▼
                    ┌───────────┐
                    │ cancelled │
                    └───────────┘
```

## Goose-Specific Extensions (superset)

These endpoints are NOT part of either ACP spec but provide richer functionality:

| Endpoint | Purpose |
|---|---|
| `POST /reply` | Original chat endpoint (SSE streaming with plan mode) |
| `GET /sessions/{id}` | Rich session info (full conversation, tokens) |
| `POST /sessions/{id}/clear` | Clear conversation + reset tokens |
| `POST /sessions/{id}/messages` | Persist a message |
| `POST /sessions/{id}/recipe` | Create recipe from conversation |
| `POST /agent/extensions` | Manage MCP extensions |
| `GET /agent/tools` | List available tools |
| `GET/POST /agent/prompts` | MCP prompt templates |
| `POST /reply mode="plan"` | Plan mode (orchestrator planning) |
| `/.well-known/agent-card.json` | A2A agent card |

## Agent / Mode Architecture

Each agent slot exposes multiple modes. ACP-REST sees them as flat agents
via `GET /agents`. ACP-IDE discovers them via `initialize` response.
Internally, the IntentRouter + OrchestratorAgent handle mode selection.

### GooseAgent (7 modes)
| Slug | Name | Category | Description |
|---|---|---|---|
| assistant | Assistant | Session | General-purpose assistant (default) |
| specialist | Specialist | Session | Focused task execution |
| recipe_maker | Recipe Maker | PromptOnly | Generate recipe files |
| app_maker | App Maker | LlmOnly | Create Goose apps |
| app_iterator | App Iterator | LlmOnly | Update Goose apps |
| judge | Judge | LlmOnly | Analyze tool operations |
| planner | Planner | PromptOnly | Create execution plans |

### CodingAgent (8 modes)
| Slug | Name | Description |
|---|---|---|
| pm | Product Manager | Requirements, stories, roadmap |
| architect | Architect | System design, technical decisions |
| backend | Backend Engineer | APIs, data models, logic (default) |
| frontend | Frontend Engineer | UI, components, UX |
| qa | Quality Assurance | Testing, coverage, quality |
| security | Security Champion | Vulnerability analysis |
| sre | SRE | Reliability, monitoring, infrastructure |
| devsecops | DevSecOps | CI/CD, deployment, security ops |

## File Layout

```
crates/goose/src/acp_compat/         ← shared adapter layer
├── mod.rs                            re-exports
├── types.rs                          AcpRun, RunCreateRequest, RunMode
├── message.rs                        goose Message ⟷ ACP Message converters
├── events.rs                         AgentEvent → ACP SSE events
└── manifest.rs                       AgentManifest, AgentStatus

crates/goose-server/src/routes/
├── acp_discovery.rs                  GET /ping, /agents, /session (ACP-REST)
├── acp_ide.rs                        POST/GET/DELETE /acp (ACP-IDE JSON-RPC)
├── runs.rs                           POST /runs lifecycle (ACP-REST)
├── reply.rs                          POST /reply (goose-native)
├── session.rs                        /sessions/* (goose-native)
└── ...                               other goose-specific routes
```

## Decision: goose-acp Crate Removed

The `goose-acp` crate (2,568 lines) was removed because:
1. It duplicated agent management logic already in goose-server
2. ACP-IDE functionality is better served as routes in the unified server
3. One server = shared state, no synchronization overhead
4. The JSON-RPC transport layer (~850 lines in `acp_ide.rs`) is simpler than
   the full crate because it delegates to AppState instead of maintaining its
   own GooseAcpAgent with separate session/mode/extension management

The `agent-client-protocol` external crate dependency was also removed — we define
our own JSON-RPC types (simpler, no rmcp/SDK dependency chain).
