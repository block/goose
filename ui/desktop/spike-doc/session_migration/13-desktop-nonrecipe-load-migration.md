# 13 Desktop Migration: Non-Recipe Sessions to ACP `loadSession`

## Goal

Swap the desktop's session-load path from REST `resumeAgent` to ACP
`acpLoadSession` for **non-recipe sessions only**. Recipe-bearing sessions
keep the REST path until the `_goose/recipe/apply` follow-up lands.

Companion to:
- [05-conversation-load.md](05-conversation-load.md) — original full
  migration plan
- [10-on-load-session-rewrite.md](10-on-load-session-rewrite.md) —
  server-side rewrite (this PR's prerequisite)
- [11-legacy-acp-load-behavior.md](11-legacy-acp-load-behavior.md)
- [12-inline-vs-legacy-comparison.md](12-inline-vs-legacy-comparison.md)

## Scope

**In scope:**

- Non-recipe sessions migrate to `acpLoadSession`.
- Desktop ACP adapter handles all replay notification types.
- Desktop wires `_goose/session/update` for cost/token tracking.
- Selective routing in `useChatStream` based on `SessionListItem.hasRecipe`.

**Out of scope (other workstreams):**

- Recipe-bearing session migration (depends on Recipe RPC follow-up).
- Live prompt, cancel, reattach migration (later slices per
  [05-conversation-load.md](05-conversation-load.md)).
- Name polling → push notifications (separate slice).
- `update_from_session` / recipe application via ACP.

## Why non-recipe first

Recipe sessions need an additional server-side primitive
(`_goose/recipe/apply` custom RPC) that hasn't landed yet. Splitting the
migration into "non-recipe now, recipe later" lets the bulk of session
traffic move to ACP without waiting for the recipe RPC. Recipe sessions
stay on REST `resumeAgent` until ready.

`SessionListItem.hasRecipe` is already available on the session-list
slice (`ui/desktop/src/acp/sessions.ts:60`), so the routing decision
is trivial at load time.

## Prerequisites — server side

This PR (server load rewrite) must be merged and inline default flipped
on. Specifically, `_meta.recipe`, `_meta.userRecipeValues`,
`_meta.extensionResults`, `_meta.workingDir` are on
`LoadSessionResponse._meta` and replay notifications stream correctly.
See [10](10-on-load-session-rewrite.md) and
[12](12-inline-vs-legacy-comparison.md) for details.

## Three phases

```
Server load rewrite (this PR)
        │
        ▼
Phase 1: adapter + notification wiring  ────┐
        │                                    │
        ▼                                    │
Phase 2: conditional swap   ─────────────────┘
        │                          (Phase 1 is hard prereq)
        ▼
Phase 3: verification + doc update
```

Phase 1 is independent of the server PR — can run in parallel.

Phase 2 hard-depends on Phase 1 (no useful swap without the adapter).

Phase 3 follows Phase 2.

## Phase 1 — Desktop foundation

Prerequisites; no production behavior change.

| # | Work | File | Approx |
|---|---|---|---|
| 1.1 | Install ACP notification router at app boot, idempotently | `ui/desktop/src/acp/sessionNotificationRouter.ts` | ~20 lines |
| 1.2 | Extend `AcpSessionNotificationAdapter` for all replay event types: `agent_thought_chunk`, `tool_call`, `tool_call_update`, `usage_update`, `session_info_update` (in addition to text chunks it already handles) | `ui/desktop/src/acp/sessionNotificationAdapter.ts` | Biggest single piece |
| 1.3 | Wire `extNotification` in ACP client callbacks for `_goose/session/update` (custom notification carrying accumulated tokens + cost) | `ui/desktop/src/acp/acpConnection.ts` | ~30 lines — parallel handler interface (`AcpGooseNotificationHandler`), `setAcpGooseNotificationHandler(...)`, and routing in `createClientCallbacks` |
| 1.4 | Bridge adapter events into existing `useChatStream` reducer: `messages` → `SET_MESSAGES`, `usage` → `SET_TOKEN_STATE`, `sessionInfo` → `SET_SESSION`, `extensionResults` → `showExtensionLoadResults(...)` | `ui/desktop/src/hooks/useChatStream.ts` | Bridge layer |

**Acceptance criteria for Phase 1:**

- App boots with notification router installed.
- Existing REST path unchanged in production.
- A debug/feature-flag side path can subscribe an adapter to an ACP
  session and confirm all event types arrive and parse correctly,
  without affecting the production state machine.

## Phase 2 — Conditional swap

The behavior change. Lands once Phase 1 is in production.

| # | Work | File | Approx |
|---|---|---|---|
| 2.1 | In `useChatStream`, before load, look up `sessionListItem.hasRecipe` from the session-list cache | `ui/desktop/src/hooks/useChatStream.ts` | ~10 lines |
| 2.2 | New branch: if `hasRecipe === false`, subscribe to session notifications, call `acpLoadSession`, dispatch state from adapter + response | new branch in `useChatStream` | ~80 lines |
| 2.3 | If `hasRecipe === true` (or unknown): keep `resumeAgent` REST path unchanged | existing path | no edit |
| 2.4 | Typed extractor for `LoadSessionResponse._meta` (mirrors `sessionInfoToListItem`): pull `recipe`, `userRecipeValues`, `extensionResults`, `workingDir` | `ui/desktop/src/acp/sessions.ts` | ~30 lines |
| 2.5 | Call `showExtensionLoadResults(meta.extensionResults)` post-load — same helper REST uses today | reuse existing helper | ~3 lines |
| 2.6 | Read provider/model from `LoadSessionResponse.models.current_model_id` and mode from `LoadSessionResponse.modes.current_mode_id` | session shape mapping | ~10 lines |

**Acceptance criteria for Phase 2:**

- Open a non-recipe session → ACP path used → conversation paints from
  replay → extension toasts fire on failures → cost chip updates as
  messages stream.
- Open a recipe-bearing session → REST `resumeAgent` path (status quo,
  no regression).
- Cancel / reattach / name-polling continue to work via existing REST
  paths (those don't migrate yet per the broader plan in 05).

## Phase 3 — Verification + cleanup

| # | Work |
|---|---|
| 3.1 | Test matrix: non-recipe session with tool calls; with thinking content; with failed extensions; with no extensions enabled; fresh session vs old session. All paint correct conversation. |
| 3.2 | Verify cost chip updates correctly from `_goose/session/update`. |
| 3.3 | Verify "session not found" returns clean error in UI if session was deleted. |
| 3.4 | Verify extension-failure toast fires when an extension has `success: false` in `_meta.extensionResults`. |
| 3.5 | Update [05-conversation-load.md](05-conversation-load.md) to mark "non-recipe load via ACP" as shipped. |

## Risks specific to this slice

- **Adapter event type mismatch.** Phase 1 must cover every event the
  server emits during replay. Any unhandled type silently drops content.
  Mitigation: test Phase 1 against a known-rich session before swapping.
- **`hasRecipe` not on a particular session.** Older sessions may not
  have `hasRecipe` populated on `SessionListItem._meta`. Defensive
  default: treat `undefined` as `true` (fall back to REST). Safer than
  accidentally hitting ACP for a recipe session.
- **Concurrent load.** A user double-clicking a session row could
  trigger two loads. ACP server-side is fine (last insert wins on
  `self.sessions`), but the desktop's subscriber wiring may need
  idempotency.
- **Subscriber registration timing.** Phase 2 must subscribe to session
  notifications **before** calling `acpLoadSession`, or replay events
  arrive before there's a subscriber and get dropped. The notification
  router has a pre-subscribe buffer
  ([`PRE_SUBSCRIBE_BUFFER_CAPACITY`](../../crates/goose/src/acp/transport/connection.rs#L23))
  on the server, but the client should also subscribe early.

## Rollback

- **Phase 1**: revert by removing the adapter handlers; production path
  unchanged because Phase 1 doesn't alter prod behavior. Safe.
- **Phase 2**: gated by `hasRecipe` check. If a class of session
  surfaces a bug, can be reverted by hardcoding the conditional to
  always pick REST, then ship. Or gate behind a separate feature flag
  (e.g., `GOOSE_DESKTOP_ACP_LOAD_NONRECIPE`) for staged rollout.
- **Server side**: the in-process `GOOSE_ACP_LEGACY_LOAD=1` env var
  still routes server-side loads back to legacy. Independent of desktop
  rollback.

## Recommended sequencing

1. This PR (server load rewrite) lands.
2. Phase 1 desktop PR — adapter + notification wiring. No prod behavior
   change. Can start now in parallel with this PR's review.
3. Phase 2 desktop PR — conditional swap. Lands after Phase 1.
4. Phase 3 verification + doc update.
5. Recipe RPC follow-up (server) + Phase 4 desktop migration of
   recipe sessions to ACP. Out of scope for this slice.