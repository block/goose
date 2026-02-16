# Deep dive: orchestration & multi-agent systems with A2A + MCP (and how Goose should use them)

This doc is intended as a working note for Goose design (local draft unless committed).

## What ‘orchestration’ means in a multi-agent system

In practice, orchestration is the control-plane that decides: which agent to call, with what inputs, how to track progress, how to stream partial results, and how to collect outputs/artifacts. It is distinct from tool execution (files, web, DBs, etc.).

## A2A (Agent2Agent) — orchestration across agents

A2A is the *agent↔agent interoperability* protocol used for delegating work to other agentic services and coordinating multi-agent workflows.

### Canonical spec inputs in the repo
- Repo: https://github.com/a2aproject/A2A
- Protobuf schema: `specification/a2a.proto` (package observed below)
- Buf configs: `specification/buf.yaml`, `specification/buf.gen.yaml`

### Evidence snippets from the local clone

README snippets:


Proto header (first lines):

```proto

```

### Orchestration primitives (what Goose will implement)

From A2A’s model (README + protobuf schema), Goose should expect these primitives:

- **Agent Card**: capability discovery (how you choose the right remote agent).
- **Task**: the unit of delegated work (create, track state, complete/fail/cancel).
- **Artifact**: output payloads/files/structured results produced by a task.
- **Streaming**: incremental progress/events (SSE mentioned in README).

### Proposed Goose A2A architecture (Rust)

Because Goose is Rust-first and A2A does not yet have a mature Rust SDK in-repo, the practical path is:

1. **Generate types** from `specification/a2a.proto` using `prost`/`tonic` (types and service definitions).
2. Implement the **A2A transport semantics** described by the spec/docs (JSON-RPC over HTTP(S), streaming via SSE, async/push mechanisms).
3. Map A2A tasks/artifacts to Goose internal abstractions (runs, messages, files, UI artifacts).
4. Keep orchestration separate from tool execution: A2A delegates to agents; those agents may themselves use MCP tools.

## MCP — tool orchestration (agent↔tool)

MCP is complementary, not competing with A2A. The simplest mental model:

- **A2A** decides *which agent* does *which work* (task delegation).
- **MCP** decides *which tool/server* is invoked to do an action (tool calls, resources).

In Goose: use MCP for tools (including memory, git, filesystem, web). Use A2A for multi-agent delegation.

## agent-client-protocol (Rust crate) — client↔agent

Source: https://docs.rs/agent-client-protocol/0.9.3/agent_client_protocol/

> rue"></span></span> <span class="title">Docs.rs</span> </a><ul class="pure-menu-list"> <script id="crate-metadata" type="application/json"> { "name": "agent-client-protocol", "version": "0.9.3" } </script><li class="pure-menu-item pure-menu-has-children"> <a href="#" class="pure-menu-link crate-name" title="A protocol for standardizing communication betw

Source: https://docs.rs/agent-client-protocol/0.9.3/agent_client_protocol/

> " href="struct.SessionId.html" title="struct agent_client_protocol::SessionId">Session<wbr>Id</a></dt><dd>A unique identifier for a conversation session between a client and agent.</dd><dt><a class="struct" href="struct.SessionMode.html" title="struct agent_client_protocol::SessionMode">Session<wbr>Mode</a></dt><dd>A mode the agent can operate in.</dd><dt><a class="struct" href="struct.SessionModeId.html" title="struct agent_client_protocol::Sessi

Source: https://docs.rs/agent-client-protocol/0.9.3/agent_client_protocol/

> onnection">Client<wbr>Side<wbr>Connection</a></dt><dd>A client-side connection to an agent.</dd><dt><a class="struct" href="struct.Content.html" title="struct agent_client_protocol::Content">Content</a></dt><dd>Standard content block (text, images, resources).</dd><dt><a class="struct" href="struct.ContentChunk.html" title="struct agent_client_protocol::ContentChunk">Content<wbr>Chunk</a></dt><dd>A streamed item of content</dd><dt><a class="struct"

Source: https://docs.rs/agent-client-protocol/0.9.3/agent_client_protocol/

## Coverage matrix (when to use what)

| Dimension | A2A | MCP | agent-client-protocol |
|---|---|---|---|
| What it connects | agent ↔ agent | agent ↔ tool/resource server | client/editor ↔ agent |
| Best for | multi-agent workflows, delegation, interoperability | tool calls, resources, memory servers | IDE/CLI integration, client messaging |
| Core objects | Agent Card, Task, Artifact | tools, resources, prompts (server-defined) | sessions, requests/responses, content blocks |
| Transport | JSON-RPC over HTTP(S) + SSE streaming (per A2A repo) | MCP protocol (varies by transport; JSON-RPC-style) | Rust types for the protocol + schema crate |
| Goose implementation | implement A2A client/server in Rust from spec | Goose already uses MCP ecosystem; extend where needed | optionally adopt crate for Goose client integrations |

## Implementation sketch for Goose orchestration

### Multi-agent workflow (A2A + MCP)
1. Planner decides to delegate a subtask.
2. Discover candidate agents via Agent Cards (registry or configured list).
3. Create A2A Task on selected agent.
4. Stream task progress/events (SSE) into Goose UI/CLI.
5. Collect Artifacts; store/emit them.
6. If task needs tools, the executing agent uses MCP servers; Goose orchestration layer treats that as opaque.

### Where this fits in Goose repo

- Core protocol implementations should live in `crates/goose` (shared by CLI + server).
- CLI calls into goose crate for A2A operations (similar to other commands).
- Desktop: add goosed routes under `crates/goose-server/src/routes` and regenerate OpenAPI.
