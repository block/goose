# 12 Inline vs Legacy: Side-by-Side Comparison

Detailed comparison of `on_load_session_inline` against `on_load_session_legacy`
in `crates/goose/src/acp/server/load_session.rs`. Use this as the
correctness checklist before flipping the dispatcher default.

Companion to:
- [10-on-load-session-rewrite.md](10-on-load-session-rewrite.md) — design rationale
- [11-legacy-acp-load-behavior.md](11-legacy-acp-load-behavior.md) — what legacy does and its problems

This doc is the implementation-level checklist proving that everything
legacy does has either:
- An equivalent in inline (with `mechanism: same` or `mechanism: equivalent`)
- A deliberate, documented difference
- Or doesn't need to be done at all (and that's documented too)

## A. Deliberate behavior changes (visible to clients/operators)

These are the intentional differences. Each one is a design decision from
doc 10.

| # | Legacy | Inline | Why |
|---|---|---|---|
| A1 | Accepts `mcp_servers` and silently persists them into `session.extension_data` via `add_extensions_bulk` | Rejects non-empty `mcp_servers` with `invalid_params` | Server-owned extension model. Clients use `_goose/extensions/add` post-load for transient extensions. |
| A2 | Overwrites `session.working_dir` with `args.cwd` unconditionally | Ignores `args.cwd` entirely | `loadSession` is a read. Explicit changes use `_goose/working_dir/update`. Fixes the goose-internal `"~"` corruption bug. |
| A3 | Returns to client after replay; agent setup continues in background `tokio::spawn` | Returns to client AFTER agent setup completes inline | One meaning of "loaded". No "spinner cleared but Send blocks" anti-pattern. |
| A4 | Session registered as `AgentHandle::Loading(rx)`, promoted to `Ready` by background task | Session registered as `AgentHandle::Ready(agent)` directly | No watch channel, no waiters, no Loading window. |
| A5 | Extension load results discarded (only `warn!`-logged) | Collected into `Vec<ExtensionLoadResult>`, surfaced on `_meta.extensionResults` | Closes the desktop's lost-extension-toast regression. |
| A6 | No `_meta` on response | `_meta` with `recipe`, `userRecipeValues`, `extensionResults`, `workingDir` | Desktop needs these for the parameter modal, the failure toasts, and to know the saved working_dir. |
| A7 | Provider/mode update errors are invisible (signaled via watch channel; legacy `loadSession` still returns `Ok`) | Provider/mode errors fail `loadSession` with the error | Fail fast, fail visibly. Phase 1 errors no longer hide until next prompt. |

## B. Same observable behavior, different mechanism

These are implementation-level differences that produce identical observable
behavior. Listed so reviewers can verify the equivalence rather than have
to derive it from scratch.

| # | Legacy | Inline | Equivalence proof |
|---|---|---|---|
| B1 | Replay loop inlined in `on_load_session_legacy` body | Replay loop in `replay_conversation_to_client` free function | Verbatim logic copy. Same match arms, same notification calls, same tool_request stashing. |
| B2 | Provider built via `provider_factory(provider_name.to_string(), ...)` closure called directly inside `spawn_agent_setup` | Provider built via `self.create_provider(&provider_name, ...)` | `create_provider` is a thin wrapper: `(self.provider_factory)(provider_name.to_string(), ...)`. Same closure invocation. |
| B3 | Phase 1 (provider init) + Phase 2 (extension load) run in spawned background task | Phase 1 + Phase 2 run inline (awaited, not spawned) | Same operations in same order. Difference is only `tokio::spawn` vs await. |
| B4 | `session_name_update_tx` created inside spawned task | Created inside `build_agent_for_session` | Both end up stored on `AgentConfig.session_name_update_tx`. Same end state. |
| B5 | Provider re-resolved inside `spawn_agent_setup` (called with `resolved_provider: None`) | Pre-resolved value passed into `build_agent_for_session` | Both paths end at `resolve_provider_and_model_from_config(&config, goose_session)`. Same data. Inline avoids one config load. |
| B6 | Order: replay → DB cwd update → register Loading → response data → spawn → notifications → return | Order: replay → response data → build agent → register Ready → notifications → response → return | Relative order of "build response struct" vs "send notifications" is reversed, but the response is the RPC reply and notifications go through the SSE channel — they're independent and the client cannot observe relative ordering. |
| B7 | Extensions loaded in parallel via `futures::future::join_all` | Same `futures::future::join_all` | Identical. |
| B8 | AcpTools developer wrap conditional on `client_fs_capabilities.read_text_file \|\| .write_text_file \|\| client_terminal` AND `developer` extension is enabled | Same condition | Identical condition, identical wrap construction. |
| B9 | Initial usage notifications: custom `_goose/session/update` + legacy `UsageUpdate` | Same: custom + legacy `UsageUpdate` | Identical. |
| B10 | `send_available_commands_update` called near end | Same | Identical. |
| B11 | `Agent::with_config(...)` constructed identically (same `AgentConfig` fields, same `with_mcp_host_info`, same `with_session_name_update_tx`) | Same | Identical. |
| B12 | `EnabledExtensionsState::extensions_or_default(Some(&goose_session.extension_data), &config)` used for `ext_state` | Same | Identical. |
| B13 | Builtin extensions added via `get_enabled_extensions_with_config(&config)` + `builtins.iter().map(builtin_to_extension_config)` | Same | Identical. |
| B14 | `agent.update_provider(provider.clone(), &goose_session.id)` and `agent.update_goose_mode(goose_mode, &setup_session_id)` | Same calls in same order | Identical. |
| B15 | Provider/extension errors logged with `warn!` macro and message | Same `warn!` calls | Identical log surface. |

## C. Things both DON'T do (no regression vs legacy)

These are behaviors the inline implementation doesn't have, but neither does
legacy. Listed to confirm we haven't introduced a gap.

| Item | Status |
|---|---|
| Apply rendered recipe to agent's system prompt | Neither applies. Both leave it for a separate operation. **Same.** (REST splits the same way: `resume_agent` doesn't apply; `update_from_session` does.) |
| Notify client when extensions finish loading via a stream event | Neither does. **Same.** (Inline surfaces results on `_meta.extensionResults` in the response payload, which is new information vs legacy but not a new stream event.) |
| Cancel in-flight load if client disconnects | Neither supports. **Same.** Once load starts, it runs to completion. |
| Auto-restore on cache miss (a la REST AgentManager) | Neither does. **Same.** ACP requires explicit `loadSession`. |
| LRU eviction of in-memory sessions | Neither does. **Same.** Both have the unbounded HashMap growth issue documented in 11. |

## D. Subtle thing worth flagging — `pending_working_dir` field

`GooseAcpSession.pending_working_dir` is initialized to `None` in both legacy
and inline. The field is set by `on_update_working_dir` (`sessions.rs:32-34`)
when an update arrives while the agent is in `Loading` state, and applied
by `spawn_agent_setup` when promoting to `Ready`.

For inline-loaded sessions, `pending_working_dir` is unreachable because the
session is never in `Loading`. For `on_new_session`-loaded sessions (which
still use `spawn_agent_setup`), it remains live and functional.

**Conclusion**: inline doesn't break this field. It's dormant for inline's
sessions but the field stays in the struct because `on_new_session` still
needs it.

## E. Failure-mode delta worth explicit awareness

**Legacy**: if `cx.send_notification` fails AFTER the `Loading` insert, the
session is left in `self.sessions` as `Loading`. The background task
continues. Future requests will wait on the watch channel for FullyReady,
then succeed.

**Inline**: if `cx.send_notification` fails AFTER the `Ready` insert, the
session is left in `self.sessions` as `Ready` but the `loadSession` RPC
returns Err. From the client's perspective, the load failed. From the
server's perspective, the session is loaded. Mismatched state.

**In practice**: `cx.send_notification` failures mean the connection is
dying. Both sides die. Not a real issue. Documented for awareness.

If strictness were desired: insert AFTER all notifications succeed. But
then there's a window where the agent exists in memory (Arc<Agent>) but
isn't in `self.sessions` yet, and concurrent RPCs would get "session not
found". Probably not worth the change.

## F. Function signatures (locked-in API)

```rust
pub(super) fn legacy_acp_load_enabled() -> bool;

impl GooseAcpAgent {
    pub(super) async fn on_load_session_legacy(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error>;

    pub(super) async fn on_load_session_inline(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error>;

    async fn build_agent_for_session(
        &self,
        cx: &ConnectionTo<Client>,
        acp_session_id: &SessionId,
        goose_session: &Session,
        resolved_provider: Option<(String, crate::model::ModelConfig)>,
        prebuilt_provider: Option<Arc<dyn Provider>>,
    ) -> Result<
        (Arc<Agent>, Vec<crate::agents::ExtensionLoadResult>),
        agent_client_protocol::Error,
    >;
}

fn replay_conversation_to_client(
    cx: &ConnectionTo<Client>,
    acp_session_id: &SessionId,
    goose_session: &Session,
) -> Result<
    HashMap<String, crate::conversation::message::ToolRequest>,
    agent_client_protocol::Error,
>;
```

Visibility: both `on_load_session_*` and `legacy_acp_load_enabled` are
`pub(super)` so the dispatcher in `server.rs` can call them. The helpers
are private to `load_session.rs`.

## G. Net summary

The inline implementation is:
- **Legacy minus** the deferred-setup machinery (`Loading` handle, watch
  channel, background spawn).
- **Plus** three deliberate policy changes (A1 `mcp_servers` reject, A2 cwd
  ignore, A5 collect extension results).
- **Plus** one new payload addition (A6 `_meta`).
- **Plus** stricter failure surfacing (A7 — Phase 1 errors fail loadSession
  rather than hiding).

Every other "difference" is mechanism-level (background → inline,
watch channel → return value, inlined loop → helper function), producing
identical observable behavior. Verified item-by-item in section B.

Nothing important is silently dropped. Section C confirms the omissions
match legacy. Section D confirms `pending_working_dir` dormancy isn't
breakage. Section E flags the only hypothetical mismatch (failed
notification after insert), which is not reachable under normal connection
liveness.

## H. Checklist for the dispatcher flip

Before flipping the default to inline in the next commit:

- [ ] All A items match the design doc (10) — yes.
- [ ] B items verified equivalent (manually traced above) — yes.
- [ ] C confirms no regression — yes.
- [ ] D field-dormancy understood — yes.
- [ ] E failure-mode delta acceptable — yes (connection-dying scenario).
- [ ] cargo build passes
- [ ] cargo fmt + clippy clean
- [ ] Tested with `GOOSE_ACP_LEGACY_LOAD=0` end-to-end against goose-internal
- [ ] Confirmed `_meta.extensionResults` toast renders on at least one
      extension-failure scenario
- [ ] Confirmed cwd no longer overwrites after load
- [ ] Confirmed mcp_servers rejection returns the expected error to a
      mis-configured client
