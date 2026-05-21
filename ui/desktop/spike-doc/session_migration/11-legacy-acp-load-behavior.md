# 11 Legacy ACP Load Behavior

What the current `on_load_session` + `spawn_agent_setup` design does
under the hood, and what users of goose-internal actually experience.

This is verification-grounded documentation: every claim here was checked
against `crates/goose/src/acp/server.rs` and the goose-internal client code.
Companion doc to [10-on-load-session-rewrite.md](10-on-load-session-rewrite.md);
this one captures the "what is" so the rewrite proposal's "what should be"
has a baseline.

## The two-phase setup

Legacy `on_load_session` returns to the client after conversation replay
streams, then continues setup in a `tokio::spawn` background task
(`spawn_agent_setup` at `crates/goose/src/acp/server.rs:1314`). That
background task runs in two phases:

### Phase 1 — provider initialization (~50ms)

- Construct the `Agent` struct in memory.
- Build the provider object (e.g. OpenAI / Anthropic) with API key.
- Call `agent.update_provider(provider)`.
- Set `goose_mode` on the agent.
- Signal `ProviderReady` on the watch channel.

Fast because nothing here spawns processes or makes network calls — pure
in-memory object construction.

### Phase 2 — extension loading (1–3 seconds typical, variable)

For each enabled extension, call `extension_manager.add_extension(config,
...)`. Cost per extension:

- **Stdio extensions** (npm/node-based MCP servers): spawn child process,
  set up stdio pipes, perform MCP `initialize` handshake, call
  `tools/list`. Cold node start can be 1-2s per extension.
- **StreamableHttp extensions**: open HTTP connection, auth, MCP
  `initialize`, `tools/list`. Bounded by network RTT + remote cold start.
- **Builtin / Platform**: register in-process tools. Microseconds.

All extensions load in parallel via `futures::future::join_all`, so total
Phase 2 time = slowest single extension, not sum.

After Phase 2:

- Promote `AgentHandle::Loading` → `AgentHandle::Ready(agent)`.
- Signal `FullyReady` on the watch channel.

### Why two phases

The split exists so operations that only need the provider (model switch,
config update) can proceed after Phase 1 without waiting for the slower
Phase 2. Three different waiter helpers exist for this:

- `get_session_agent` — waits for `FullyReady` (most callers, including
  `on_prompt`, `on_get_tools`).
- `get_session_agent_provider_ready` — waits for `ProviderReady` only.
  Used by `update_provider`, `set_model`, `build_config_update`.
- `get_agent_or_receiver` — low-level, returns either the agent or the
  watch receiver to wait on.

## What goose-internal does with this

goose-internal's flow
(`~/Development/goose-internal/src/shared/api/acpSessionRegistry.ts`):

```
await acpApi.loadSession(sessionId, workingDir);
await acpApi.setProvider(sessionId, providerId);
```

- `loadSession`: passes `mcpServers: []`, throws away the response.
- `setProvider`: hits `get_session_agent_provider_ready` server-side.
  Blocks ~50ms in legacy (waits for Phase 1 completion).
- The `prepared` cache skips this whole sequence on second open of the same
  session within one connection.

Net legacy timing for first open of an existing session:

- `loadSession` resolves: ~10ms (replay only).
- `setProvider` resolves: ~50ms (waits for Phase 1).
- Total client-perceived load: ~60ms.
- Phase 2 still loading in background, invisible to client.

## What happens when the user prompts before Phase 2 completes

Step by step:

1. User hits Send → `client.prompt(...)` RPC fires
   (`acpApi.ts:259`).
2. Server's `on_prompt` (`server.rs:2963`) calls `get_session_agent`.
3. `get_session_agent` (`server.rs:2617`) calls
   `rx.wait_for(FullyReady|Err)`. **This await is not cancellable.**
4. **No notification fires to the client during this wait.** RPC just hangs.
5. Once Phase 2 completes, prompt processes normally and streams response
   chunks back as `SessionUpdate` notifications.

### User-visible behavior

- Click Send → "AI is typing…" indicator appears.
- **0–3 seconds of nothing happens visibly.**
- Then response starts streaming.
- Looks indistinguishable from a slow LLM first-token. No way to know it's
  actually waiting on Phase 2.

### Cancellation is broken during this wait

Verified against `server.rs:2973-2976` and `server.rs:3161-3180`:

1. `on_prompt` creates `cancel_token` at line 2973.
2. Passes it to `get_session_agent`. Inside `get_agent_or_receiver`
   (`server.rs:2606-2608`), `session.cancel_token = Some(token)` is set
   **before** the watch wait begins.
3. `get_session_agent` then awaits `rx.wait_for(FullyReady|Err)` at line
   2627. **This await is not cancellable** — there's no `tokio::select!`
   between it and the cancel token.

What happens if the user clicks Cancel during the invisible wait:

- `on_cancel` (`server.rs:3171`) calls `token.cancel()`.
- The token is now cancelled, but `get_session_agent` is still blocked on
  the watch channel. The cancel signal is not observed.
- Wait continues until Phase 2 completes (or Phase 1 fails).
- Once unblocked, `on_prompt` proceeds to `agent.reply()` at line 3010,
  passing the already-cancelled token.
- The stream loop checks `cancel_token.is_cancelled()` at line 3031 on the
  first event and breaks.

Net result: the cancel is recorded but ineffective until the wait
completes. Phase 2 itself is never interrupted — it runs to completion in
the background regardless of any cancellation. The prompt request returns
cancelled only after Phase 2 finishes. The user's wall-clock wait is the
same as if they hadn't cancelled.

The next prompt they type will wait the same way (Phase 2 still loading
from the original spawn).

## What happens if Phase 2 fails

Verified against `server.rs:1521-1539`:

- Per-extension failures: `warn!`-logged only, error swallowed
  (`server.rs:1498`).
- Top-level Phase 2 error: `error!`-logged, but the agent is promoted to
  `Ready` anyway (`server.rs:1538`). The comment explicitly says
  "Extension failures are non-fatal."

User-visible behavior:

- Agent is marked Ready. Subsequent prompts succeed.
- If the LLM tries to call a tool from a failed extension, it gets
  "tool not found" or similar mid-conversation.
- No breadcrumb back to the original load failure. No client notification
  was ever sent about it.
- User sees the agent fail mysteriously in conversation, ~30+ seconds
  after the actual failure happened.

## What happens if Phase 1 fails (provider init)

Verified against `server.rs:1423-1427` and `server.rs:2639-2641`:

- Phase 1 error: `error!`-logged AND `agent_tx.send(Some(Err(e)))` is
  called (`server.rs:1425`).
- The next caller of `get_session_agent` receives the error via the watch
  channel.
- `get_session_agent` translates it to
  `Error::internal_error().data(e.clone())`.
- The prompt RPC fails with that internal error.
- Client sees an error message.

This is the one failure path that's actually visible. But: it only
surfaces when the next request arrives. If no request comes, the failure
is just a server log line.

## Net behavior table for goose-internal users

| Scenario | What user sees |
|---|---|
| Prompts while Phase 2 still loading | Slow first-token, indistinguishable from slow LLM |
| Prompts after Phase 2 finished | Normal fast response |
| Phase 2 silent failure, uses working tools | Normal response |
| Phase 2 silent failure, uses broken tool | Confusing "tool failed" with no context |
| Phase 1 failure | Visible error on next request |
| Cancels during invisible wait | Effectively ignored until Phase 2 finishes |

## Other lifecycle problems with the deferred-setup pattern

Beyond the prompt-during-load case, the deferred-setup design has several
lifecycle issues. Each verified against the actual code.

### 1. Client disconnect leaks work

The background task (`server.rs:1350 tokio::spawn`) holds `Arc` clones of
`sessions`, `session_manager`, `permission_manager`, etc. It does not check
connection liveness anywhere. Phase 1 and Phase 2 run to completion
regardless of whether the client is still around. MCP child processes
still spawn. Extensions still complete their MCP handshake. The session
entry ends up `Ready` in the HashMap, owned by no one.

The `cx.clone()` at line 1333 is used only for the AcpTools wrapper (line
1445) and the session-name notifier (line 1365) — neither gates the core
setup work.

### 2. Ambiguous lifecycle — four meanings of "loaded"

Verified by reading the wait helpers and call sites:

- `loadSession` request resolved → spinner cleared (client thinks done).
- Session is in `sessions` HashMap → present, but `AgentHandle::Loading`.
- `ProviderReady` signal fired → can call `setModel`.
- `FullyReady` signal fired → can actually prompt.

Different code paths care about different meanings. Reading the file,
you can't tell which a given operation needs without checking the wait
helper.

### 3. Asymmetric error visibility between phases

Phase 1 errors propagate to the next waiter (visible). Phase 2 errors are
swallowed and the agent goes Ready anyway (invisible). Two failure paths
with totally different observable consequences, no protocol surface to
distinguish them.

### 4. Panics produce indistinguishable errors

`tokio::spawn` discards the JoinHandle at line 1350. If the background
task panics, the `agent_tx` watch sender is dropped during unwinding,
which closes the channel. Existing waiters get a closed-channel error,
translated to `"Agent setup task was dropped"`
(`server.rs:2637`). Same generic error as a clean exit without signal.

Panics aren't fully invisible — waiting requests find out something went
wrong — but they're indistinguishable from other failure modes.

### 5. No load-time cancellation

The session is registered with `cancel_token: None`
(`server.rs:2895`). `spawn_agent_setup`'s body contains no
`CancellationToken`, no `tokio::select!`, nothing checking a cancel
signal. The session's `cancel_token` is only set later, by
`get_agent_or_receiver` when called from `on_prompt`. So there is no
mechanism to cancel an in-flight load. Once `loadSession` starts, Phase 2
runs to completion regardless.

### 6. cwd is unconditionally overwritten on every load

`server.rs:2879-2884`:

```rust
self.session_manager.update(&session_id)
    .working_dir(args.cwd.clone())
    .apply().await?;
```

Combined with goose-internal's `"~"` fallback when `workingDir` is
undefined (`shared/api/acp.ts:212`), every `acpLoadSession` call where
the caller didn't pass a workingDir silently corrupts the session's saved
`working_dir` to `"~"`. This is a latent bug, not a hypothetical.

## Architectural regression vs REST: agent cache scope

Separate from the deferred-setup issues above, the ACP architecture
introduces a class of problems REST did not have: **agent caches are
per-connection, not process-wide.** Each `GooseAcpAgent` (one per ACP
connection) has its own `sessions` HashMap holding its own `Arc<Agent>`
instances. REST's `AgentManager` is a process-wide singleton with an LRU
cache, shared across every HTTP request.

This split has three distinct consequences, all of which are regressions
relative to REST behavior.

### 1. No cross-client cache sharing

If client A and client B both load the same session_id, each builds its
own `Arc<Agent>` and spawns its own set of MCP child processes for that
session. REST would have served both from a single cached agent.

Practical impact:
- One goose-internal app: no impact (single client).
- goose-internal + `goose acp` CLI on the same machine, both touching the
  same session: duplicated MCP processes.
- Multiple goose-internal app instances: duplicated MCP processes.

Rare in practice today, but a real resource cost when it happens.

### 2. No LRU eviction — intra-connection accumulation

REST's `AgentManager.sessions` is `LruCache<String, Arc<Agent>>` with a
capacity of 100 (`GOOSE_MAX_ACTIVE_AGENTS`). Old agents get evicted as
new ones come in. MCP processes for evicted agents terminate.

ACP's `GooseAcpAgent.sessions` is an unbounded `HashMap`. Entries leave
only when:
- The client explicitly calls `closeSession` (`server.rs:3497`), or
- The connection ends (whole `GooseAcpAgent` drops).

**goose-internal never calls `closeSession`** — verified by grepping
`~/Development/goose-internal/src` for `closeSession` and `close_session`
(zero matches). So sessions accumulate for the entire connection lifetime,
which for goose-internal is the app process lifetime.

Practical impact: a user who opens 50 chats in one workday holds 50 live
agents in memory with 50 sets of MCP child processes, until they quit the
app.

### 3. Multi-client data races on shared session state

When two clients each have their own `Arc<Agent>` for the same persistent
session, writes to the session DB race because the application layer has
no read-modify-write protection.

**Verified DB primitives:**
- `SessionManager` uses `sqlx::Pool<Sqlite>` (`session_manager.rs:472`).
- Multi-statement writes use `BEGIN IMMEDIATE` transactions
  (`session_manager.rs:777, 839, 1132, 1327`).
- `apply_update` is a single atomic `UPDATE` statement
  (`session_manager.rs:1206`).
- No row-version check, no compare-and-set. Classic last-write-wins per
  field.

So SQLite serializes writes at the file level (no corruption) but the
application has no protection against lost updates. Concrete races:

**Lost extension state.** `persist_extension_state`
(`agent.rs:1033`) is read-modify-write with no transaction:
```rust
let session = session_manager.get_session(session_id, false).await?;
let mut extension_data = session.extension_data.clone();
extensions_state.to_extension_data(&mut extension_data)?;
session_manager.update(session_id).extension_data(extension_data).apply().await?;
```
If Client A and Client B both enable an extension at similar times, one
write clobbers the other. The extension Client A enabled is silently
gone.

**Lost token accumulation.** Each agent writes absolute token totals based
on its own prompts. Concurrent prompting → last write wins → one client's
tokens vanish from the persisted total.

**In-memory state divergence.** Each `Arc<Agent>` has its own in-memory
view of extensions, conversation buffer, etc. Client A's writes don't
update Client B's in-memory state. Client B will operate on a stale view
until it reloads.

**Session metadata (name, recipe, model_config).** All `apply_update`-based,
all last-write-wins.

**Message append.** Less broken: `add_message` uses transactions, both
messages get persisted. But order is whichever transaction commits first,
and each agent's in-memory conversation cache only contains its own
appended message until reload.

### Why REST naturally avoids these races

REST's process-wide `AgentManager` cache means two HTTP requests for the
same session_id get **the same `Arc<Agent>`**. Internal Agent state
(extension_manager, conversation buffer, token counters) is one in-memory
object for both requests. The Agent's internal locking serializes its own
writes. One writer per session at any time, by construction.

ACP's per-connection design loses this property. Multiple in-memory Agents
for the same persistent session → DB races become possible.

### Severity

- SQL-layer guarantees prevent corruption — no torn writes, no schema
  violations.
- Application-layer bugs manifest as **silent inconsistencies**: vanished
  extensions, wrong token counts, stomped recipe values, stale tool
  availability.
- Hard to detect, hard to debug, no error messages.

### What fixing all three would require

All three regressions share the same underlying root cause: ACP's
per-connection `Arc<Agent>` design. Fixing any one of them well requires
the same architectural work:

- Decompose `Agent` into shared parts (provider HTTP client, MCP child
  processes, conversation state) and per-connection wrapping (`AcpTools`
  capability wrapping, in-flight tool requests, cancel tokens).
- Hold the shared parts in a process-wide cache (analogous to REST's
  `AgentManager`).
- Add LRU eviction to bound memory.
- Reference-count so MCP processes shut down only when no connection holds
  the session.
- Per-connection `GooseAcpAgent` builds thin facades over the shared parts.

This is a real architectural project, not part of the load rewrite. It
should be tracked as a known future workstream.

### Out of scope for the load rewrite

The on_load_session inline rewrite does **not** address any of these. It
fixes the deferred-setup issues (extension toasts, recipe application,
cancellation, lifecycle) but inherits the per-connection cache architecture
unchanged. Sessions still accumulate per-connection, MCP processes still
duplicate across clients, multi-client writes still race.

These are pre-existing properties of ACP's design that long predate the
load rewrite. Calling them out here so they're not forgotten — fixing them
is a separate, larger workstream.

## What the desktop REST flow shows is needed

ui/desktop's REST `resumeAgent` flow gives an existence proof that the
client wants extension load results:

- `useChatStream.ts:722-725`: reads `resumeData.extension_results`, calls
  `showExtensionLoadResults(extensionResults)`.
- `extensionErrorUtils.ts:40`: fully-built error-toast / grouped-status
  UI with per-failure recovery hints ("Ask goose to explain this
  error...").
- `BottomMenuExtensionSelection.tsx:96`: subscribes to
  `SESSION_EXTENSIONS_LOADED` event so the extension picker refreshes.

This is product behavior that the desktop team decided was needed. The
legacy ACP design quietly broke it by removing the channel through which
`extension_results` flowed.

## What this means for the rewrite

The inline + `_meta.extensionResults` design proposed in
[10-on-load-session-rewrite.md](10-on-load-session-rewrite.md) fixes all
of the above in one change:

- No invisible wait: `loadSession` resolves only when everything is
  actually ready. Cancel works because there's nothing hidden in the
  background to cancel.
- Extension failures surface as a toast at load time, not as confusing
  mid-conversation errors.
- No "agent ready but degraded" state.
- One meaning of "loaded": the request resolved.
- Phase 1 / Phase 2 distinction disappears entirely. No watch channel, no
  wait helpers, no AgentHandle::Loading.
- Lifecycle, cancellation, and panic handling collapse to normal RPC
  semantics.
- cwd guard prevents silent corruption.

The current behavior isn't just complex — it produces real user-visible
bugs that the inline rewrite eliminates.

## REST comparison (for context)

REST `resume_agent` is fully synchronous: blocks on provider + extension
loading + recipe application before returning. The desktop user already
experiences a 1-3s spinner on cold session open today. Migrating to inline
ACP preserves that exact behavior.

REST also has a background-prewarm optimization, but only at
**session-creation** time (`start_agent`, `routes/agent.rs:331`). For
"open an existing session from cold" — the most common load case —
REST does **not** prewarm. It loads extensions synchronously inline,
same as the inline ACP rewrite would.

So:

- For cold-open of existing sessions: REST and inline ACP behave the
  same. Legacy ACP is the outlier with its hidden background load.
- For new-session-then-prompt within one server lifetime: REST has
  a prewarm advantage. Out of scope for this load-rewrite PR.

The perf optimization legacy ACP provides over inline only matters for
the narrow case of "open old session and let it sit several seconds before
prompting." That's not a workflow REST users have ever had optimized
either, so there's no regression to defend against.
