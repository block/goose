# MCP Servers Ownership on `session/new` and `session/load`

## Problem

ACP's `session/new` and `session/load` both accept an `mcpServers` parameter.
The spec's implied model is:

- **Client owns MCP configuration.** The client persists the per-session
  MCP server list in its own storage.
- **Agent owns conversation.** The agent persists history, tool calls,
  recipe state, etc.
- On every `session/new` or `session/load`, the client re-supplies
  `mcpServers`. The agent only caches MCP connections in-memory for the
  lifetime of the active session — it never persists them across loads.

This split is coherent: the agent doesn't need durable MCP state because
the client always re-declares it.

Goose violates this split. Per-session extension config lives in the goose
server DB. The desktop/CLI clients read and mutate that config via REST
today, but the **agent itself** is the source of truth — not the client.

So when an ACP `session/load` arrives with `mcpServers`, goose has two
sources for the same data: the request payload and the server-side DB.
Silently ignoring the request causes hard-to-debug drift for any
non-goose ACP client. Honoring it means overwriting persisted state
(same destructive shape as `cwd` overwriting `working_dir` on load).

## Decision

Keep the current goose architecture (server owns MCP config). Do not
migrate to client-owned config — that's a multi-week refactor touching
CLI, desktop, server, DB schema, and recipe handling, and the multi-client
/ spec-purity payoff is small today.

Communicate the divergence loudly and discoverably instead.

## Proposal: Advertise + Error

### 1. Advertise ownership at handshake

Add to `InitializeResponse._meta`:

```json
{
  "goose": {
    "mcpServersOwnership": "agent",
    "mcpServersOnNewBehavior": "reject-if-non-empty",
    "mcpServersOnLoadBehavior": "reject-if-non-empty"
  }
}
```

Goose-aware clients (desktop, CLI) read this at init and send
`mcpServers: []` on subsequent calls. Third-party clients can inspect the
init response to discover the contract.

### 2. Reject non-empty `mcpServers` on `session/new` and `session/load`

- Empty array or omitted: accepted, proceed normally.
- Non-empty: reject with a goose-specific error code and a message that:
  - Names the offending field.
  - States "goose manages MCP servers agent-side."
  - Points at the `_meta` advertisement from step 1.
  - Suggests sending `mcpServers: []`.

### 3. Document

- Note the divergence in the goose ACP integration doc.
- Add a comment at the rejection site in
  `crates/goose/src/acp/server.rs` so future readers find the rationale.

## Why this shape

- **No silent drift.** Any client sending MCPs gets a loud, actionable
  error on first contact — not weeks later when a tool mysteriously
  doesn't show up.
- **Goose-aware clients bypass the error entirely** by reading `_meta`
  during init.
- **Existing server-owned extension architecture stays intact.** No DB
  migration, no client-side config store, no recipe-bundled-extension
  rework.
- **Minimal implementation.** One `_meta` field at init, one validation
  check on each session entry point.

## Trade-offs considered

| Option | Verdict |
|---|---|
| Silently ignore `mcpServers` | Rejected — invisible drift, debugging nightmare for third-party integrators. |
| Warn-and-ignore (emit notification) | Rejected — relies on clients surfacing notifications; still ambiguous. |
| Honor as additive overlay (server config + client-supplied) | Rejected for now — more code, blurs ownership, no concrete demand. |
| Migrate goose to client-owned MCP config (spec-pure) | Deferred — real refactor (2–4 weeks), no current driver. |
| **Advertise + error (this proposal)** | Chosen. |

## Spec compatibility note

The ACP spec lists `mcpServers` as required on `session/new`. Rejecting
when the field is **present-but-non-empty** is a goose-specific behavior;
clients that pass `mcpServers: []` remain spec-conformant on the wire.
The `_meta` advertisement is the discovery mechanism that lets careful
clients comply without trial-and-error.

## Out of scope

- Migrating per-session extension config out of the goose server DB into
  client storage.
- Cross-client MCP config sharing (CLI ↔ desktop).
- Third-party ACP client integration paths beyond the error message and
  init-time advertisement.

Revisit when a real third-party ACP client wants to contribute tools to a
goose session — at that point, option 4 (additive overlay) or full
client-owned migration becomes worth the cost.
