**Goal:** Replace goosed's custom SSE-based streaming API with a standards-based ACP (Agent Communication Protocol) over HTTP interface, enabling better client portability, protocol standardization, and eventually a unified interface for all goose clients.

## Background

### Current Architecture

- **goose-cli** (Rust): Direct in-process agent communication
- **goosed** (goose-server): Custom REST + SSE streaming API consumed by the Electron desktop app

### Proposed Architecture

A new **ACP-over-HTTP** server that:
- Implements the standardized ACP (Agent Communication Protocol) specification
- Uses JSON-RPC 2.0 over HTTP with SSE streaming as the transport
- Enables any ACP-compatible client to interact with goose

## Prototype

See the prototype branch: https://github.com/block/goose/tree/alexhancock/goosed-acp-and-new-cli

---

## Phase 1: Stabilize ACP Server

**Goal:** Production-ready ACP server that can fully replace goosed

### Tasks

- [ ] Session persistence and resumption (`session/load`)
- [ ] Feature parity with goosed (recipes, extensions, modalities, etc)
- [ ] Integration tests

### Success Criteria
- All goosed features available via ACP-over-HTTP
- Integration tests passing

---

## Phase 2: TypeScript TUI Alpha

**Goal:** Feature-complete TUI which will be an evolution of the current CLI

### Tasks

- [ ] Equivalent of `goose configure`
- [ ] Equivalent of `goose session` (with list, resume, etc)
- [ ] Equivalent of `goose term` if we want to keep that going
- [ ] Make all MCP features work (sampling, elicitation, roots, etc)
- [ ] UX polish (syntax highlighting, markdown rendering, themes?)

### Success Criteria
- TUI usable as daily driver
- Build process that makes a single binary
- Docs

---

## Phase 3: Desktop Migration

**Goal:** Migrate Electron desktop app from goosed to ACP

### Tasks

- [ ] Integrate ACP client into desktop app
- [ ] Feature flag to toggle between goosed and ACP backends
- [ ] A/B testing and performance benchmarks

### Success Criteria
- Desktop app multi-chat works with ACP backend
- Feature flagged
- No user-facing regressions

---

## Phase 4: Consolidation

**Goal:** ACP becomes the single interface for all goose clients

### Tasks

- [ ] Deprecate and remove `goosed`
- [ ] Deprecate and remove `goose-cli`
- [ ] Update all documentation

### Success Criteria
- `goosed` removed from codebase
- `goose-cli` removed from codebase
- Single unified architecture for addressing goose

---

## Technical Details

### ACP Protocol Overview

The ACP protocol uses JSON-RPC 2.0 with the following methods:

**Client → Server Requests** (client sends, server responds):
```
initialize          - Handshake and capability negotiation
authenticate        - Authentication (currently no-op)
session/new         - Create a new conversation session (returns ACP session_id)
session/load        - Resume an existing session (replays history via notifications)
session/prompt      - Send user message, receive streaming response
```

**Client → Server Notifications** (client sends, no response):
```
session/cancel      - Cancel in-progress prompt
```

**Server → Client Requests** (server sends, client must respond):
```
request_permission  - Request user confirmation for tool execution
```

**Server → Client Notifications** (server sends via SSE, no response):
```
SessionNotification with SessionUpdate variants:
  - AgentMessageChunk    - Streaming text from agent
  - AgentThoughtChunk    - Streaming reasoning/thinking content
  - UserMessageChunk     - User message (used in session/load replay)
  - ToolCall             - Tool invocation started (status: pending)
  - ToolCallUpdate       - Tool status change (completed/failed) with result
```

### HTTP Transport

The HTTP transport wraps ACP's JSON-RPC protocol over HTTP + SSE:

```
POST /acp/session              - Create session, returns { session_id }
POST /acp/session/{id}/message - Send JSON-RPC request/response to server
GET  /acp/session/{id}/stream  - SSE stream for ALL server→client messages
GET  /health                   - Health check
```

### Message Flow

All server→client communication flows through the SSE stream, including:
- **Results**: Responses to client requests (initialize, session/prompt)
- **Notifications**: Streaming updates (agent_message_chunk, tool_call, tool_call_update)
- **Requests**: Server-initiated requests requiring client response (request_permission)

```
Client                              Server
  │                                   │
  │─── POST /acp/session ────────────>│  Create session (returns Goose session_id)
  │<────────── { session_id } ────────│
  │                                   │
  │─── GET /acp/session/{id}/stream ─>│  Open SSE connection
  │              ┌────────────────────│  Connection stays open for all responses
  │              │ (SSE stream open)  │
  │              ▼                    │
  │─── POST /message ────────────────>│  { method: "initialize", id: 1 }
  │<─────────── SSE event ───────────│  { id: 1, result: { capabilities } }
  │                                   │
  │─── POST /message ────────────────>│  { method: "session/prompt", id: 2,
  │                                   │    params: { session_id, prompt } }
  │<─────────── SSE event ───────────│  notification: AgentMessageChunk
  │<─────────── SSE event ───────────│  notification: AgentThoughtChunk (if reasoning)
  │<─────────── SSE event ───────────│  notification: ToolCall (status: pending)
  │<─────────── SSE event ───────────│  notification: ToolCallUpdate (status: completed)
  │<─────────── SSE event ───────────│  notification: AgentMessageChunk
  │<─────────── SSE event ───────────│  { id: 2, result: { stop_reason: "end_turn" } }
  │                                   │
  │                                   │
  │  ═══ Permission Flow (when tool requires confirmation) ═══
  │                                   │
  │─── POST /message ────────────────>│  { method: "session/prompt", id: 3, ... }
  │<─────────── SSE event ───────────│  notification: ToolCall (status: pending)
  │<─────────── SSE event ───────────│  { method: "request_permission", id: 99, params: {...} }
  │─── POST /message ────────────────>│  { id: 99, result: { outcome: "allow_once" } }
  │<─────────── SSE event ───────────│  notification: ToolCallUpdate (status: completed)
  │<─────────── SSE event ───────────│  { id: 3, result: { stop_reason: "end_turn" } }
  │                                   │
  │                                   │
  │  ═══ Cancel Flow ═══
  │                                   │
  │─── POST /message ────────────────>│  { method: "session/prompt", id: 4, ... }
  │<─────────── SSE event ───────────│  notification: AgentMessageChunk
  │─── POST /message ────────────────>│  { method: "session/cancel" }  ← notification (no id)
  │<─────────── SSE event ───────────│  { id: 4, result: { stop_reason: "cancelled" } }
  │                                   │
  │                                   │
  │  ═══ Resume Session Flow ═══
  │                                   │
  │─── POST /acp/session ────────────>│  Create new HTTP session
  │<────────── { session_id } ────────│  (new session_id for transport)
  │                                   │
  │─── GET /acp/session/{id}/stream ─>│  Open SSE connection
  │                                   │
  │─── POST /message ────────────────>│  { method: "initialize", id: 1 }
  │<─────────── SSE event ───────────│  { id: 1, result: { capabilities } }
  │                                   │
  │─── POST /message ────────────────>│  { method: "session/load", id: 2,
  │                                   │    params: { session_id: <existing>, cwd } }
  │<─────────── SSE event ───────────│  notification: UserMessageChunk (history replay)
  │<─────────── SSE event ───────────│  notification: AgentMessageChunk (history replay)
  │<─────────── SSE event ───────────│  notification: ToolCall (history replay)
  │<─────────── SSE event ───────────│  notification: ToolCallUpdate (history replay)
  │<─────────── SSE event ───────────│  { id: 2, result: {} }
```
