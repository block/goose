# 10 `on_load_session` Rewrite

## Motivation

`crates/goose/src/acp/server.rs::on_load_session` + `spawn_agent_setup` is
~440 lines split across an enum (`AgentHandle::Loading`), a watch channel
(`AgentSetupSignal`), a two-phase progress signal
(`AgentSetupProgress::ProviderReady` / `FullyReady`), and three wait helpers
(`get_session_agent`, `get_session_agent_provider_ready`,
`get_agent_or_receiver`). REST `resume_agent` accomplishes the same
user-visible outcome in ~80 lines.

The complexity exists to support **deferred setup**: `on_load_session`
returns as soon as conversation replay completes, while provider init and
extension loading continue in a background `tokio::spawn`. Operations that
need the agent wait on a watch channel server-side, invisible to the client.

Audit of that optimization (see analysis in conversation log; recapped here):

- **It does not speed up conversation paint.** Replay notifications stream
  during the call regardless of whether setup is deferred or inline.
- **It hides latency rather than removing it.** A prompt sent immediately
  after `loadSession` resolves blocks server-side on the watch channel.
  From the user's perspective, "spinner clears, but Send is mysteriously
  slow" is worse UX than "spinner stays a beat longer, then everything
  works."
- **It dropped the extension-failure toast.** REST returned
  `extension_results` synchronously; the deferred-setup path has no
  client-visible "setup complete" signal, so per-extension success/failure
  cannot be surfaced.
- **It cost the recipe system-prompt application.** REST's desktop client
  calls `update_from_session` after every `resumeAgent` to apply the
  rendered recipe. The ACP path has no equivalent — recipe-bearing sessions
  silently behave as plain chats.

The "inline setup" rewrite collapses those problems. This doc captures the
design before implementation.

## Scope

**Strictly: only `on_load_session`. Nothing else touched.**

**In scope (this PR):**

- Move current `on_load_session` body to a new file
  `acp/server/load_session.rs`, renamed to `on_load_session_legacy`.
  **Bit-for-bit identical behavior.**
- Add `on_load_session_inline` (new implementation) in the same file.
  Inline mirrors REST `resume_agent`'s shape (provider + extensions),
  with these new policies:
  - Reject non-empty `mcpServers` with `invalid_params`.
  - Ignore `args.cwd` entirely (session is source of truth).
  - Surface `recipe`, `userRecipeValues`, `extensionResults`, `workingDir`
    on `LoadSessionResponse._meta`.
- Add dispatcher `on_load_session` that routes based on
  `GOOSE_ACP_LEGACY_LOAD` env var. Default → inline.

**Recipe application is NOT in scope for this PR.** Mirroring REST's split
(`resume_agent` does provider + extensions; `update_from_session` applies
the rendered recipe), recipe application lives in a separate custom RPC
in a follow-up PR. The raw `recipe` and `userRecipeValues` still go out
on `_meta` so the desktop can render the parameter modal — that part is
unchanged.

**Explicitly NOT in scope (this PR):**

- `on_load_session_legacy` — untouched except for the rename. No policy
  changes, no bug fixes, no `mcp_servers` rejection, no cwd guard, no
  recipe application. The legacy path remains a true bit-for-bit rollback
  target.
- `on_new_session` — not touched at all. Still uses `spawn_agent_setup` +
  `AgentHandle::Loading`. `mcp_servers` rejection on new_session deferred
  to the follow-up.
- The fork/duplicate site near server.rs:3432 — not touched.
- `AgentHandle::Loading`, `AgentSetupSignal`, `AgentSetupProgress`,
  `spawn_agent_setup`, wait helpers, `add_mcp_extensions` — all remain in
  use by `on_new_session` and the fork site.

**Follow-up PRs (sequenced):**

- **Recipe RPC** (`_goose/recipe/apply`): mirrors REST `update_from_session`
  semantics. Client calls it after `loadSession` (and after the user fills
  any required parameters) to render the recipe and apply it as a system
  prompt extension on the agent. Required for recipe-bearing sessions to
  behave correctly under ACP — desktop migration depends on this landing
  alongside or before the ACP loadSession cutover.
- Rewrite `on_new_session` to the same inline pattern. Add `mcp_servers`
  rejection there.
- Rewrite the fork/duplicate site.
- Delete `on_load_session_legacy`, the dispatcher, the env var, and the
  whole async-setup machinery (`AgentHandle::Loading`, `spawn_agent_setup`,
  wait helpers).

## Non-goals

- No protocol changes. No new `SessionUpdate` variants. All new fields go
  on `_meta`.
- No new custom notifications introduced by this PR. (`_goose/session/update`
  for accumulated tokens/cost already exists and remains unchanged.)
- No client breakage. ui/desktop and goose-internal continue to work with
  zero changes; new `_meta` fields are opt-in reads.

## Design

### File layout

New file: `crates/goose/src/acp/server/load_session.rs`

Contains everything related to `on_load_session`:

```
acp/server/load_session.rs (new, ~470 lines)
├── pub(super) async fn on_load_session_legacy      (~250 lines, moved verbatim from server.rs)
├── async fn on_load_session_inline                  (~80 lines, new)
├── fn replay_conversation_to_client                 (~100 lines, new copy of legacy replay loop)
├── async fn build_agent_for_session                 (~80 lines, new copy of spawn_agent_setup phase 1+2)
└── pub(super) fn legacy_acp_load_enabled            (~10 lines, env helper)

acp/server.rs
└── on_load_session                                  (~10 line dispatcher, stays here; routes to legacy or inline based on env)
```

Existing `acp/server.rs` shrinks by ~250 lines (current `on_load_session`
body moves out). All other functions in `server.rs` remain unchanged.

The submodule declaration is added to wherever `acp/server/` is wired in
(likely `acp/server.rs` itself or `acp/mod.rs`).

**Why a new file rather than appending to existing `acp/server/sessions.rs`:**

- Load is large (~500 lines total) — enough to warrant its own file.
- Side-by-side legacy + inline in one file makes the comparison readable
  during the transition period.
- Cleanup PR is a single-file delete (`git rm load_session.rs`) plus folding
  the dispatcher away. Minimal blast radius.
- The existing `acp/server/` convention already includes operation-themed
  files (`dispatch.rs`, `custom_dispatch.rs`, `onboarding.rs`), so a new
  operation-themed file fits the precedent.

### Function shape

```rust
async fn on_load_session_inline(
    &self,
    cx: &ConnectionTo<Client>,
    args: LoadSessionRequest,
) -> Result<LoadSessionResponse, agent_client_protocol::Error> {
    // 1. Validate mcp_servers empty (policy)
    if !args.mcp_servers.is_empty() {
        return Err(invalid_params(
            "goose manages MCP servers server-side; use \
             _goose/extensions/add to add extensions to a session"
        ));
    }

    // 2. Fetch session
    let goose_session = self.session_manager.get_session(&id, true).await?;

    // 3. Stream replay first — conversation paints before slow setup
    replay_conversation_to_client(cx, &args.session_id, &goose_session)?;

    // 4. cwd: ignore request.cwd entirely. Session is source of truth.
    //    Verified zero sessions have empty working_dir in production data;
    //    no recovery path needed. Explicit changes use _goose/working_dir/update.

    // 5. Build agent inline (provider + extensions)
    //    Note: recipe is NOT applied here. Mirroring REST's split
    //    (resume_agent + update_from_session), recipe application is a
    //    separate custom RPC in a follow-up PR.
    let (agent, extension_results) =
        build_agent_for_session(&goose_session, &deps).await?;

    // 6. Register session as Ready (never Loading)
    self.sessions.lock().await.insert(id.clone(), GooseAcpSession {
        agent: AgentHandle::Ready(agent.clone()),
        tool_requests: HashMap::new(),
        chain_membership: HashMap::new(),
        responded_tool_ids: HashSet::new(),
        summarized_chains: HashSet::new(),
        cancel_token: None,
        pending_working_dir: None,
    });

    // 7. Post-load notifications
    send_initial_usage_updates(cx, &args.session_id, &goose_session, &agent)?;
    self.send_available_commands_update(cx, &args.session_id).await?;

    // 8. Response
    Ok(LoadSessionResponse::new()
        .modes(mode_state)
        .models(model_state)
        .config_options(config_options)
        .meta(json!({
            "recipe": goose_session.recipe,
            "userRecipeValues": goose_session.user_recipe_values,
            "extensionResults": extension_results,
            "workingDir": goose_session.working_dir,
        })))
}
```

Body ~80 lines. Helpers below.

### Helpers (new, this PR — pure additions, no extraction)

All helpers are **new code in `load_session.rs`, used only by
`on_load_session_inline`**. None of them are extracted from existing code.
Legacy and `spawn_agent_setup` keep their own copies of the equivalent
logic, untouched.

| Helper | Purpose | Approx LOC |
|---|---|---|
| `replay_conversation_to_client(cx, session_id, session)` | Walks `session.conversation.messages()`, emits one `SessionUpdate` notification per content item. **Logic copied from today's `on_load_session` replay loop; legacy keeps its inline copy unchanged.** | ~100 |
| `build_agent_for_session(session, deps) -> (Arc<Agent>, Vec<ExtensionLoadResult>)` | Phase 1 (provider init) + Phase 2 (extension load) executed inline and returning the per-extension results. **Logic copied from `spawn_agent_setup`; that function stays unchanged.** Recipe is **not** applied here — that lives in a separate custom RPC. | ~80 |

Recipe application is intentionally out of scope. See "Follow-up PRs" for
the `_goose/recipe/apply` custom RPC that mirrors REST `update_from_session`.

### No extraction, only duplication — by design

`build_agent_for_session` and `replay_conversation_to_client` duplicate
logic from `spawn_agent_setup` and the legacy `on_load_session` body
respectively, instead of extracting shared code.

**Rationale (ACP-side):** the legacy path and `spawn_agent_setup` are
the kill-switch fallback (and `on_new_session`'s implementation). If we
refactor either to call new shared helpers, their behavior is no longer
bit-for-bit identical to today, weakening the "flip env var to revert"
guarantee and risking unintended changes to `on_new_session`.

**Rationale (vs goose-core helpers):** goose-core already has
`agent.restore_provider_from_session` and
`agent.load_extensions_from_session` (used by REST, AgentManager,
Gateway, tests). `build_agent_for_session` re-implements equivalent
logic instead of calling those helpers. This is also a deliberate
choice for this PR:

- `restore_provider_from_session` reads `Config::global()` rather than
  the per-instance `self.config_dir` that ACP uses. Behaviorally the
  same in production, but architecturally couples ACP to the global
  singleton.
- `restore_provider_from_session` persists the fallback provider back
  to the session DB on case-1 (registry miss). Violates the read-only
  principle we adopted for `loadSession`.
- `load_extensions_from_session` doesn't know about the AcpTools
  `developer` wrap and would either load developer twice or need a
  new "exclusion" parameter.

Adopting the goose-core helpers also requires deciding on a per-instance
`AgentManager` to get LRU caching and auto-restore for free. That set of
design choices (helpers + AgentManager + scheduler ownership) is
substantial enough to deserve its own focused PR — see Path B in the
follow-up list. **Bundling it into this PR would expand scope and weaken
the kill-switch's bit-for-bit revert guarantee.**

**Cost:** ~200 lines of duplicated logic, temporarily. The duplication
disappears naturally in the follow-up PRs:
- Legacy's replay copy → deleted with `on_load_session_legacy`.
- `spawn_agent_setup`'s phase-1/2 copy → deleted when `on_new_session`
  and the fork site are rewritten to use the inline pattern.
- `build_agent_for_session`'s phase-1/2 copy → replaced by calls to
  goose-core helpers in the Path B follow-up.

**This PR is pure addition for legacy, `spawn_agent_setup`, and the
goose-core helpers. Zero edits to any of them.**

### What stays in place

- `AgentHandle::Loading`, `AgentSetupSignal`, `AgentSetupProgress` — still
  used by `on_new_session` and the third site.
- `spawn_agent_setup` — same.
- `get_session_agent`, `get_session_agent_provider_ready`,
  `get_agent_or_receiver` — same.
- `add_mcp_extensions` — still used by `on_new_session`.

### Dispatcher

```rust
async fn on_load_session(&self, cx, args) -> ... {
    if legacy_acp_load_enabled() {
        self.on_load_session_legacy(cx, args).await
    } else {
        self.on_load_session_inline(cx, args).await
    }
}

fn legacy_acp_load_enabled() -> bool {
    std::env::var("GOOSE_ACP_LEGACY_LOAD")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
```

## Policy decisions

### `mcpServers` rejection (inline only)

`on_load_session_inline` rejects non-empty `LoadSessionRequest.mcp_servers`:

```
Err: invalid_params (-32602)
data: "goose manages MCP servers server-side; use _goose/extensions/add
       to add extensions to a session"
```

**Scope:** inline path only. Legacy load and `on_new_session` are NOT
touched in this PR. `on_new_session` rejection deferred to the follow-up
PR that rewrites it.

**Why:** The current `add_extensions_bulk` call inside `spawn_agent_setup`
silently persists request-supplied MCP servers into `session.extension_data`
([agent.rs:1213-1214](../../crates/goose/src/agents/agent.rs#L1213)). The
client thinks it's saying "expose these for this session"; it's really
saying "permanently add these to this session forever." No removal path
exists.

**Known clients:** both pass `[]` today.
- ui/desktop: `acp/sessions.ts:17`, `:38`
- goose-internal: `shared/api/acpApi.ts:216`, `:245`

So the rejection is safe to enable for inline immediately — neither known
client triggers it.

**Future Zed-style use case:** explicit `_goose/extensions/add` post-load
(already in the custom RPC set), with explicit lifecycle semantics.

### `cwd` policy (inline only)

Today's legacy behavior:

```rust
self.session_manager.update(&session_id)
    .working_dir(args.cwd.clone())
    .apply().await?;
```

Unconditional overwrite. Combined with goose-internal's `"~"` fallback
(`shared/api/acp.ts:212`), this silently corrupts saved working_dir on every
`acpLoadSession` call where the caller didn't pass `workingDir`.

**New behavior (inline only): ignore `args.cwd` entirely on load.**
`loadSession` becomes a pure read operation — no side effects on
`session.working_dir`.

**Legacy is NOT touched.** It keeps the unconditional overwrite. If the
kill switch is flipped to legacy for any reason, the cwd corruption bug
returns as part of the rollback (which is correct — kill switch is
bit-for-bit revert).

Decision rationale:

- Verified against the local sessions DB: zero sessions have empty
  `working_dir` (0 out of 86 in sample). A "set if empty" recovery path
  would be dead code.
- The existing UI design treats working_dir changes as explicit, validated,
  user-initiated operations:
  - REST: `POST /agent/update_working_dir`
    ([routes/agent.rs:903](../../crates/goose-server/src/routes/agent.rs#L903))
    — validates path exists, persists, restarts agent.
  - ACP: `_goose/working_dir/update` custom RPC
    ([acp/server/sessions.rs:4](../../crates/goose/src/acp/server/sessions.rs#L4))
    — same semantics.
- Folding cwd changes into `loadSession` as an unavoidable side effect
  violates the read/write split that the rest of the codebase already
  follows.

Sessions own their `working_dir`. Once set (by `newSession`), it's
immutable via `loadSession`. Clients that need to change working dir use
the explicit `_goose/working_dir/update` RPC.

### Recipe application — NOT in this PR

Recipe application is intentionally split out of `on_load_session_inline`,
mirroring REST's split between `resume_agent` (no recipe) and
`update_from_session` (applies recipe).

**Why split:**

- Each function has one responsibility. Load sets up session machinery
  (provider + extensions). Recipe RPC handles recipe-specific prompt
  extension. Easier to read, test, and modify in isolation.
- Matches the existing REST architecture, so anyone familiar with REST
  immediately understands ACP.
- Cleanly handles the "fresh session, params not yet filled" case: load
  returns, UI shows param modal, user fills, then recipe RPC fires.
- Smaller PR scope. No recipe primitives or `apply_recipe_if_present`
  helper in `load_session.rs`.

**What this PR still does for recipes:** the raw `recipe` and
`userRecipeValues` are surfaced on `LoadSessionResponse._meta` so the
desktop can render the parameter modal (when params are missing) or
trigger the recipe RPC (when params are present). Same data the desktop
sees today from REST `getSession`/`resumeAgent` — just delivered via ACP.

**Recipe application itself** lands in a follow-up PR that introduces a
custom RPC `_goose/recipe/apply` mirroring REST `update_from_session`:
takes a session_id (and optionally values), renders the recipe with
`build_recipe_with_parameter_values`, applies via
`apply_recipe_to_agent` + `agent.extend_system_prompt("recipe", …)`.

**Trade-off accepted:** until that follow-up lands, recipe-bearing
sessions loaded via ACP `loadSession` will not have their recipes
applied. Desktop and goose-internal migrations to ACP need to wait for
both PRs (or call REST `update_from_session` as a transition fallback).
Same risk class as the bug that motivated this whole rewrite (legacy ACP
dropped recipe application silently) — mitigated by surfacing the recipe
data on `_meta` so the gap is visible to anyone implementing a client.

### `extension_results` surface

`LoadSessionResponse._meta.extensionResults` as an array of
`{ name, success, error? }`. Vanilla ACP clients ignore `_meta` and load
the session normally. Goose-aware clients (ui/desktop, goose-internal) read
the field and fire toasts on `success=false` entries.

**Why `_meta` and not a top-level field:** `LoadSessionResponse` is a
protocol-defined schema. New fields go on `_meta` per ACP's extensibility
convention.

**Why not a new `SessionUpdate` variant:** ACP does not sanction custom
variants of standard enums. Clients pattern-match on known variants;
adding `ExtensionLoadResult` to `SessionUpdate` breaks vanilla clients.

**Why not a custom notification:** `_goose/extensions/load_results` would
work, but it's a one-shot value already known at the moment the response
resolves. Folding it into the response is simpler — no separate subscription
or ordering concern.

### Recipe surface (raw)

`LoadSessionResponse._meta.recipe` (full recipe JSON) and
`LoadSessionResponse._meta.userRecipeValues` (persisted param values).

Both surface unconditionally — the client uses them to render the param
input modal (for fresh sessions) and to call the future `_goose/recipe/apply`
RPC (once params are filled).

**Sanitization:** verified against REST `GET /sessions/{id}`
([routes/session.rs:134-145](../../crates/goose-server/src/routes/session.rs#L134)).
REST returns `Json(session)` with the `recipe` field passed through
unchanged — no sanitization. Match REST behavior: pass `goose_session.recipe`
through as-is.

## Performance analysis

### Latency comparison

| Operation | REST today | ACP legacy | ACP inline |
|---|---|---|---|
| `loadSession` request resolution | n/a | replay_time | replay_time + provider_init + extension_load |
| `resumeAgent` request resolution | replay_time + provider_init + extension_load + recipe_apply | n/a | n/a |
| Conversation paint | response | during stream | during stream |
| Time to first successful prompt | response | replay_time + extension_load_wait_in_prompt | response |

### What gets slower

`loadSession` resolution gains the extension load time (~1-3s typical).
Spinner stays a beat longer.

### What does not get slower

- Conversation paint — replay notifications stream during the call
  regardless.
- Time to first successful prompt — today's "fast load, slow prompt" path
  blocks on the watch channel anyway; inline blocks the spinner instead.
  Same wall-clock wait, made visible.

### Mitigations (out of scope this PR)

If measurement shows the longer spinner is a real problem:

1. **Prewarm globally-enabled extensions at ACP `initialize` time.** Once
   per connection, not once per session. Per-session load only handles
   session-specific configuration.
2. **Share extension_manager across sessions in a connection.** Each
   `Agent` today has its own. Most extensions are stateless and global —
   one manager per connection is a much bigger refactor but a real win.

Both are post-rewrite optimizations. Do not preempt.

## ACP protocol fit

- `SessionUpdate` notifications used during replay: `UserMessageChunk`,
  `AgentMessageChunk`, `ToolCall`, `ToolCallUpdate`, `AgentThoughtChunk`.
  All standard. No custom variants.
- New fields on `LoadSessionResponse._meta`: `recipe`, `userRecipeValues`,
  `extensionResults`, `workingDir`. All optional, all camelCase, matching
  existing `session_meta` convention in
  `crates/goose/src/acp/server.rs:270-289`.
- Replay-complete boundary remains `loadSession` resolution per ACP spec.
- `_goose/session/update` (custom notification for accumulated tokens/cost)
  remains unchanged and continues to be emitted after replay.
- Legacy `UsageUpdate` notification continues to be emitted alongside
  `_goose/session/update` for backwards compatibility. Same as today.

No protocol violations. No new namespacing.

## WSS / streaming fit

The inline shape is cleaner than deferred for WSS:

- **One boundary instead of two.** Today's client distinguishes "request
  resolved" (response back) from "agent ready" (no signal exists, just
  hope). Inline: `loadSession` resolution = agent ready. Simpler state
  machine.
- **No dangling background tasks.** If a client disconnects right after
  `loadSession` resolves, nothing's left running server-side awaiting their
  attention. Today, `spawn_agent_setup` keeps running even after the
  client is gone.
- **Backpressure-friendly.** Replay notifications still stream during the
  call; the server flushes through the WSS pipe as it always does. No
  change to streaming behavior.

## Client impact

### ui/desktop (in-flight migration to ACP)

- Replace `resumeAgent` + `update_from_session` with one `acpLoadSession`
  call.
- Read recipe / extension_results / workingDir from
  `LoadSessionResponse._meta`.
- Use `showExtensionLoadResults(response._meta.extensionResults)` to fire
  toasts on load resolution (reuse existing helper at
  `ui/desktop/src/utils/extensionErrorUtils.ts`).
- Subscription pattern: install notification router and subscribe before
  calling `acpLoadSession`, per existing migration plan in
  `05-conversation-load.md` (no change to that plan).

### goose-internal

- No required changes. Continues to throw away `LoadSessionResponse` and
  works.
- Recommended opt-in: start reading `response._meta.extensionResults` to
  add extension-failure toasts (currently absent).
- `prepared` session cache (`acpSessionRegistry.ts:60-62`) remains valid;
  cache hit semantics unchanged.
- `setProvider` followup call (`acpSessionRegistry.ts:48-49`) can stay or
  be replaced by reading `LoadSessionResponse.models.current_model_id`.
  Independent decision.
- The `"~"` cwd fallback (`shared/api/acp.ts:212`) stops being dangerous —
  saved working_dir is no longer overwriteable via `loadSession`.

### Other ACP clients (future)

- Zed-style clients that want to add transient MCP servers per session:
  reject on load, instead use `_goose/extensions/add` post-load.
- Vanilla ACP clients that ignore `_meta`: continue to load and replay
  conversation normally. No behavior change.

## Maintainability

### File shape during transition (after this PR)

```
on_load_session (~10 line dispatcher)
├── on_load_session_legacy (renamed + moved verbatim, ~250 lines,
│   ZERO logic changes — true bit-for-bit fallback)
└── on_load_session_inline (~80 line body — new code with all new policies)
    ├── replay_conversation_to_client (~100 lines, new copy)
    ├── build_agent_for_session (~80 lines, new copy of spawn_agent_setup logic)
    └── apply_recipe_if_present (~30 lines, new)

(unchanged, still used by on_new_session and fork/duplicate site:)
├── AgentHandle::Loading
├── AgentSetupSignal / AgentSetupProgress
├── spawn_agent_setup
├── get_session_agent / get_session_agent_provider_ready / get_agent_or_receiver
└── add_mcp_extensions
```

Net change this PR: ~+250 lines (new inline path + helpers, no deletions
yet). File grows during transition; shrinks dramatically when follow-up
PRs delete the deferred-setup machinery.

### File shape after follow-up cleanup PRs

```
on_load_session (~80 line body) — no dispatcher, no legacy
├── replay_conversation_to_client (~100 lines)
├── build_agent_for_session (~80 lines)
└── apply_recipe_if_present (~30 lines)
on_new_session (~similar inline shape)
```

Net change after all PRs land: ~-300 lines vs today.

## Testing

All tests target the inline path. Legacy is exercised only by setting
`GOOSE_ACP_LEGACY_LOAD=1` and confirming the existing test suite still
passes unchanged.

- Unit test `legacy_acp_load_enabled` env routing.
- Unit test `mcp_servers` rejection on `on_load_session_inline` (legacy
  and `on_new_session` are not in scope for this PR).
- Integration test the inline path: load a session with conversation and
  extensions; assert messages replayed, agent Ready,
  `_meta.extensionResults` populated, `_meta.recipe` and
  `_meta.userRecipeValues` surfaced when present, `working_dir`
  preserved.
- Integration test cwd is ignored on inline: load with non-empty saved
  `working_dir` and a different request `cwd`; assert saved value
  preserved (no overwrite happened).
- Integration test that recipe is **not** applied to the agent's system
  prompt on load (recipe-bearing session: assert no `recipe` system prompt
  exists on the agent after load). Recipe application is tested
  separately by the `_goose/recipe/apply` PR.
- Re-run the existing ACP load test suite with `GOOSE_ACP_LEGACY_LOAD=1`
  to confirm the legacy path is bit-for-bit unchanged.

## Decisions

| # | Question | Decision |
|---|---|---|
| 1 | cwd policy | **Always ignore `args.cwd` on load** (inline only). Verified zero sessions have empty `working_dir` in DB. Legacy untouched. |
| 2 | Recipe application | **Deferred to a separate custom RPC** (`_goose/recipe/apply`, follow-up PR). Mirrors REST split (`resume_agent` doesn't apply recipe; `update_from_session` does). `_meta.recipe` and `_meta.userRecipeValues` still surface so UI can render param modal. |
| 3 | `extensionResults` shape | **Full list** (success + failures, match REST). Desktop's `showExtensionLoadResults` branches on shape and expects the full list. |
| 4 | Recipe sanitization | **No sanitization needed.** Verified REST `GET /sessions/{id}` returns `recipe` field unchanged. Match REST behavior — pass through as-is. |
| 5 | Legacy `UsageUpdate` notification | **Keep emitting** alongside `_goose/session/update` for backwards compat. Delete in the follow-up PR that retires legacy load. |
| 6 | Env var name | `GOOSE_ACP_LEGACY_LOAD=1` opts back into legacy. Default = inline (after the flip commit). |
| 7 | Per-extension failures | **Match REST**: warn-log per-extension failures, include in `Vec<ExtensionLoadResult>` with `success: false`. Do not fail the entire load. |
| 8 | `on_new_session` policies | **Deferred to follow-up.** This PR does not touch `on_new_session` at all. `mcp_servers` rejection and other policy changes apply there only when it's rewritten. |
| 9 | File layout | **Option B1**: new file `acp/server/load_session.rs` holds legacy + inline + helpers + env. Dispatcher stays in `server.rs`. Legacy moves verbatim from `server.rs`. |

## PR commit plan

Three commits, each independently revertable:

1. **Move + rename, zero behavior change.**
   - Create `acp/server/load_session.rs`.
   - Move current `on_load_session` body verbatim from `server.rs` into
     the new file as `on_load_session_legacy`.
   - Add `legacy_acp_load_enabled()` env helper.
   - Add `on_load_session` dispatcher that always routes to legacy.
   - Add `mod load_session;` declaration where appropriate.
   - At end of commit: file moved, dispatcher in place, behavior 100%
     identical to today.

2. **Add `on_load_session_inline` with all new behavior.**
   - Implement inline (sync setup, `mcp_servers` rejection, cwd ignore,
     recipe apply, `_meta.extensionResults` on response).
   - Add helpers: `replay_conversation_to_client`,
     `build_agent_for_session`, `apply_recipe_if_present`.
   - Dispatcher still routes to legacy by default. Inline is dead code at
     end of this commit, exercised only by tests.

3. **Flip dispatcher default to inline.**
   - Single-line change: `legacy_acp_load_enabled()` defaults to `false`.
   - Inline becomes the production path. Legacy survives behind
     `GOOSE_ACP_LEGACY_LOAD=1`.

If commit 3 needs to be reverted in production: set
`GOOSE_ACP_LEGACY_LOAD=1` server-side, no redeploy required. If commit 2
needs to be reverted: `git revert` it, dispatcher still works because
default is legacy.

## Rollback

- Default: inline. Revert by setting `GOOSE_ACP_LEGACY_LOAD=1` on the
  server. No redeploy required.
- Hard revert: revert the dispatcher-flip commit. Legacy stays in place
  until a future cleanup PR.
- Both clients (ui/desktop, goose-internal) work against either path —
  rollback is server-only.

## Follow-up PRs (sequenced)

1. **Recipe RPC** (`_goose/recipe/apply`): mirrors REST `update_from_session`.
   See "Recipe application — NOT in this PR" above. Required for
   recipe-bearing sessions to behave correctly under ACP.

2. **Path B: adopt goose-core helpers + per-instance `AgentManager`.**
   - Replace `build_agent_for_session`'s manual phase-1/2 with calls to
     `agent.restore_provider_from_session` and
     `agent.load_extensions_from_session` from goose core.
   - Introduce an ACP-instantiated `AgentManager` on `GooseAcpAgent` for
     LRU caching + auto-restore on miss (not the REST singleton — a
     separate instance per ACP connection).
   - Parameterize `AgentManager` for `GoosePlatform` and
     `session_name_update_tx` (currently hardcoded / unsupported).
   - Resolve scheduler ownership (currently constructed inside
     `AgentManager::new`; must be shared process-wide).
   - Handle AcpTools `developer` wrap as a post-load overwrite, or
     enhance `load_extensions_from_session` to accept an exclusion list.
   - Decide on `Config::global()` vs per-instance `config_dir` policy.
   - Decide on persist-on-fallback policy
     (`restore_provider_from_session` writes to DB on case-1 miss; we
     may want a read-only variant).

3. Rewrite `on_new_session` to inline shape, alongside or after Path B
   so it uses the same shared helpers.

4. Rewrite the fork/duplicate site near server.rs:3432.

5. Delete `on_load_session_legacy`, `spawn_agent_setup`,
   `AgentHandle::Loading`, `AgentSetupSignal`, `AgentSetupProgress`,
   `get_session_agent_provider_ready`, `get_agent_or_receiver`,
   `add_mcp_extensions`, the env var, and the dispatcher.

6. (Optional) Prewarm globally-enabled extensions at ACP `initialize`
   time, if measurement shows the longer spinner is a problem.

7. (Optional) Share `extension_manager` across sessions in a connection.
   Bigger refactor.
