# 05 Conversation Load

## Goal

Replace REST `getSession` / `resumeAgent` cold-load with ACP `session/load`.

Status as of 2026-05-25: the normal cold-load path now uses ACP
`session/load`; REST `resumeAgent` is no longer the source of the conversation
snapshot. REST remains for recipe prompt application (`updateFromSession`),
reattach / SSE recovery, edit/fork, app-cache population, and a few metadata
fallbacks.

## State of the World

- `ui/desktop/src/acp/sessions.ts` already exports `acpLoadSession`,
  `acpListSessions`, and friends.
- `ui/desktop/src/acp/sessionNotificationRouter.ts` exports
  `installAcpSessionNotificationRouters`, `subscribeToAcpSession`, and
  `subscribeToAcpGooseSession`; `App.tsx` installs the routers once.
- `ui/desktop/src/acp/sessionNotificationAdapter.ts` now handles text,
  thinking, tool calls, tool updates, Goose usage notifications, config option
  updates, pending elicitation interactions, and ACP permission display
  messages.
- Server `on_load_session` dispatches to inline load by default through
  `crates/goose/src/acp/server/load_session.rs`; `GOOSE_ACP_LEGACY_LOAD=1`
  routes back to the legacy implementation.
- Inline ACP load emits replay chunks, builds the agent synchronously, stores a
  ready agent handle, emits usage updates, sends available commands, and then
  resolves `session/load`. Per ACP, `session/load` resolution is the
  replay-complete boundary.
- Inline ACP load intentionally ignores `args.cwd` and returns
  `LoadSessionResponse._meta.workingDir`; the old saved-working-dir overwrite
  only exists in legacy mode.
- `LoadSessionResponse.models` already carries `current_model_id` /
  `ModelInfo`, so provider and model can be read directly from the response —
  no metadata seeding needed.

## Data Gaps

`SessionListItem` covers: `id`, `name`, `workingDir`, `updatedAt`,
`messageCount`, `providerId`, `modelId`, `userSetName`, `hasRecipe`.

Inline on `LoadSessionResponse`: `modes`, `models` (provider + model_id),
`config_options`.

Streamed via `SessionUpdate`: conversation messages, tool calls, thinking,
`UsageUpdate.used`, `UsageUpdate.size`.

Already emitted by the server (no server change needed):

- `_goose/session/update` (`GooseSessionNotification`) carries the rich usage
  payload: `used`, `context_limit`, `accumulated_input_tokens`,
  `accumulated_output_tokens`, `accumulated_cost`. Defined in
  `crates/goose-sdk/src/custom_notifications.rs`. The client just needs to
  receive it (see Client work below).

Original gaps and current status:

| Field | Used for | Suggested server fix |
|---|---|---|
| `recipe` (full JSON) | recipe rendering, param substitution | Done: `LoadSessionResponse._meta.recipe` |
| `user_recipe_values` | recipe params submit | Done: `LoadSessionResponse._meta.userRecipeValues` |
| `extension_results` | extension load error toasts | Done: `LoadSessionResponse._meta.extensionResults` |
| `working_dir` | safe deep-link / cold-open load | Done: `LoadSessionResponse._meta.workingDir` |
| rendered recipe → agent system prompt | recipe behavior on load | Still open for ACP-native behavior; desktop preserves current behavior by calling REST `updateFromSession` |

The load slice has replaced `resumeAgent`, but recipe prompt application still
depends on REST.

## Recommendation (ACP shape)

Grounded in the ACP extensibility doc
(https://agentclientprotocol.com/protocol/extensibility.md):

- `_meta` = attach custom data to existing messages without changing their
  semantics.
- Custom methods/notifications = new protocol behaviors. Method names MUST
  start with `_` and be namespaced.
- ACP does **not** sanction custom variants of standard enums like
  `SessionUpdate`. New event types belong in custom notifications, not in
  new variants of existing ones.

### Conventions (existing in the goose ACP layer)

- Custom methods: `_goose/<area>/<verb>` — already established by
  `_goose/session/update`, `_goose/extensions/add`,
  `_goose/working_dir/update`, `_goose/session/rename`,
  `_goose/preferences/save`, etc. (see
  `crates/goose-sdk/src/custom_requests.rs`,
  `crates/goose-sdk/src/custom_notifications.rs`).
- `_meta` keys: **top-level camelCase**, matching the existing
  `session_meta` convention in
  `crates/goose/src/acp/server.rs:270-289` (`messageCount`, `createdAt`,
  `archivedAt`, `userSetName`). New fields added here join that same
  flat namespace.
- Do **not** introduce a reverse-DNS namespace (`block.xyz/goose`) — it
  conflicts with the conventions already in the repo.

`_meta` keys introduced by this slice (all top-level, camelCase):

- `LoadSessionResponse._meta.recipe` — full recipe JSON (sanitized).
- `LoadSessionResponse._meta.userRecipeValues` — persisted param values
  for the session's recipe, when present.
- `LoadSessionResponse._meta.extensionResults` — array of
  `{ name, success, error? }` returned after inline agent setup completes.
- `LoadSessionResponse._meta.workingDir` — persisted working directory.

### Refined field-by-field plan

| Need | Mechanism | Why |
|---|---|---|
| `recipe` (full JSON) | `LoadSessionResponse._meta.recipe` | Pure data, no new semantics, optional. Vanilla clients that ignore `_meta` still load the session correctly. |
| `user_recipe_values` (already persisted) | `LoadSessionResponse._meta.userRecipeValues` | Bundled with recipe; one round-trip. |
| Rendered recipe → system prompt (resumed session with values present) | Auto-apply inside `spawn_agent_setup` | Server already has the values in the DB. No protocol surface needed. |
| Rendered recipe → system prompt (fresh session, user just filled params) | **Custom request** `_goose/recipe/apply { values }` | Replaces today's `PUT /sessions/{id}/user_recipe_values` + `update_from_session` in one call. Persists, renders, applies as system prompt extension, returns the rendered recipe. Invoked at most once per session today (no mid-session edit UI). Naming follows the existing `_goose/<area>/<verb>` style; `apply` captures the activation side effect (not just data write). |
| `extension_results` | Implemented as `LoadSessionResponse._meta.extensionResults` after inline setup completes | One-shot transient signal, not persisted, only fires UI toasts. Returning it on load matches the now-synchronous inline setup. |
| Auto-rename push (replaces current REST name polling) | `SessionInfoUpdate` (standard, no `_meta`) | Standard ACP info-update semantics; eliminates the polling loop in `useChatStream`. |

Total extension surface: **one custom RPC, three `_meta` fields, two
`SessionInfoUpdate` pushes.** No new `SessionUpdate` variants. No new
namespace conventions.

### Findings that shape this plan

- **Mid-session recipe param editing does not exist in the UI today.**
  `ParameterInputModal` only renders when `!session?.user_recipe_values`
  (`ui/desktop/src/components/BaseChat.tsx:539`); once values are
  persisted, no UI surfaces them again. So a custom `set_recipe_values`
  RPC is needed at most once per session (between setup and first message
  on fresh sessions only). Resumed sessions never call it.
- **`extension_results` is fully transient.** Not persisted to the session
  DB; held only in `state.extension_loading_tasks` (a one-shot rendezvous,
  see `crates/goose-server/src/state.rs`). The only consumer is
  `showExtensionLoadResults` (`ui/desktop/src/utils/extensionErrorUtils.ts`)
  which fires toasts and discards the array. `SessionInfoUpdate._meta`
  cleanly fits this lifecycle — no need for a streaming notification or
  persistence.
- **The UI does not call `getSession` after `resumeAgent` to populate the
  conversation.** It already takes `loadedSession.conversation` straight
  off the resume response (`ui/desktop/src/hooks/useChatStream.ts:737-748`).
  Other `getSession` callers (name polling, edit/fork, sidebar) are
  unrelated to the cold-load path and migrate independently:
  - Name polling → replace with `SessionInfoUpdate` push.
  - Edit/fork → already has the message list in memory; no server read
    needed.
  - `reloadConversation` (SSE buffer overrun recovery) → re-`loadSession`
    or tear-down + reconnect. Open question: ACP spec is silent on whether
    `session/load` can be called twice on the same session within one
    connection. Verify against Zed's impl before relying on it; otherwise
    a custom `_block.xyz/goose/replay_from(messageId)` RPC is the fallback.

### Open risks specific to this shape

- **`_meta` recipe payload may carry secrets** (extension API keys, etc.).
  Confirm the same sanitization that today's REST `getSession` applies is
  also applied before serializing into `LoadSessionResponse._meta`.
- **Ordering of `SessionInfoUpdate.extensionResults` vs history replay.**
  Extensions load during setup, so the info-update may arrive interleaved
  with `SessionUpdate` history-replay events. Toast firing must not couple
  to "history replay complete." Surface them independently in the
  reducer.
- **`session/load` re-entrancy** (above). Validate before adding a
  recovery path that depends on it.

### Phased rollout

1. Ship `_meta.recipe` + `_meta.userRecipeValues` on `LoadSessionResponse`.
   Unblocks recipe rendering immediately.
2. Auto-apply rendered recipe inside `spawn_agent_setup` for resumed
   sessions where all params are present. Removes the desktop's
   `update_from_session` REST dependency for resumed sessions.
3. Add the custom RPC `_goose/recipe/apply` for the fill-in-then-submit
   case on fresh sessions. Retire the REST
   `PUT /sessions/{id}/user_recipe_values`.
4. Emit `extension_results` via `SessionInfoUpdate._meta` once setup
   completes. Wire toasts.
5. Push auto-renames via `SessionInfoUpdate`; delete the name-polling
   loop in `useChatStream`.
6. Audit remaining `getSession` callers; migrate each to either the
   client-side session cache or a `SessionInfoUpdate` subscription.

Steps 1–2 are independent and unblock the load slice. Steps 3–5 can land
in any order. Step 6 is cleanup.

## Files

- `ui/desktop/src/hooks/useChatStream.ts`
- `ui/desktop/src/acp/sessionNotificationAdapter.ts`

## Implementation Steps And Status

### Server prerequisites (do these first)

1. Done: `crates/goose/src/acp/server/load_session.rs` inline load attaches
   `recipe`, `userRecipeValues`, `extensionResults`, and `workingDir` to
   `LoadSessionResponse._meta`.
2. Still open for ACP-native behavior: apply the rendered recipe to the
   agent's system prompt during ACP agent setup. The DB only persists the raw
   `recipe` + `user_recipe_values`; the
   rendered prompt is computed at runtime via
   `build_recipe_with_parameter_values` and pushed to the agent via
   `apply_recipe_to_agent` (today this lives in REST `update_from_session`,
   which the desktop calls after every `resumeAgent`). ACP currently has no
   equivalent, so ACP-loaded recipe sessions would silently behave as plain
   chats. Pick one:
   - Auto-apply during `spawn_agent_setup` when `session.recipe.is_some()`
     and all params are present, or
   - Expose a Goose-custom ACP request that mirrors `update_from_session`
     so the desktop can drive it explicitly (mirrors the existing
     useEffect-on-session shape).
3. Done: extension load results are returned as
   `LoadSessionResponse._meta.extensionResults` after inline agent setup
   completes. This models provider/extension setup readiness, not conversation
   replay completion.

No replay-complete notification is needed for conversation replay. ACP already
requires `session/load` to resolve only after replay notifications have been
sent.

Not needed — the rich usage payload (`accumulated_input_tokens`,
`accumulated_output_tokens`, `accumulated_cost`) is already emitted via the
custom `_goose/session/update` notification (`GooseSessionNotification`).
The client just needs to receive it.

### Replay completion

ACP `session/load` defines the replay boundary. The server sends historical
conversation entries as `session/update` notifications, then responds to the
original `session/load` request only after replay is complete.

Therefore the client should not require a Goose-specific
`SessionInfoUpdate._meta.replayComplete` flag.

Client behavior:

1. Install the ACP notification router before loading.
2. Subscribe/register replay handling for `sessionId` before calling
   `acpLoadSession`.
3. Mark the session as replay-loading.
4. Buffer ACP replay notifications into the session adapter while
   `acpLoadSession` is pending.
5. Optionally flush the adapter snapshot on a short throttle so history can
   appear progressively during long replays.
6. When `acpLoadSession` resolves, do one final adapter flush.
7. Dispatch `SESSION_LOADED`, call `onSessionLoaded`, and leave
   `LoadingConversation`.

Important: progressive replay display is only a paint optimization. The
authoritative "conversation loaded" boundary remains `acpLoadSession`
resolution. Provider/extension setup may still be running after replay
completes; track that separately if the UI needs setup readiness or
extension-load errors.

### Client work

4. Done: wire `Client.extNotification(method, params)` in
   `ui/desktop/src/acp/acpConnection.ts` to dispatch the custom
   `_goose/session/update` notification. Without this, the UI never sees
   `accumulated_input_tokens` / `accumulated_output_tokens` /
   `accumulated_cost` and the CostTracker chip silently regresses.
   - Add a parallel handler interface (e.g. `AcpGooseNotificationHandler`)
     and `setAcpGooseNotificationHandler(...)`.
   - In `createClientCallbacks`, implement `extNotification` to route
     `_goose/session/update` payloads (typed as `GooseSessionNotification`
     from `@aaif/goose-sdk`).
   - Extend `sessionNotificationRouter` (or add a sibling router) so each
     subscribed session receives the goose-specific updates scoped by
     `sessionId`.

5. Done: extend the adapter beyond text in the same change as the hook migration:
   - `agent_thought_chunk` → thinking content on the assistant message
   - `tool_call` → desktop tool request shape
   - `tool_call_update` → desktop tool response/update shape
   - standard ACP `usage_update` → `TokenState.totalTokens` + context limit
   - custom `_goose/session/update` `usage_update` → `TokenState`
     accumulated tokens and cost
   - `session_info_update` → session name / updatedAt / messageCount merge;
     also `extensionResults` if that surface is chosen

   Text-only replay is not a useful UI milestone for any non-trivial session.

6. Done: install the router once. `App.tsx` calls
   `installAcpSessionNotificationRouters()`.

   Historical guidance: pick the chat hook mount or app boot — the call
   is idempotent (`installed` guard inside the module). React hooks must not
   call `setAcpNotificationHandler` directly.

7. Mostly done: inside `useChatStream`, before calling `acpLoadSession`:
   - Still open: look up the matching `SessionListItem` for `sessionId`.
   - Current code starts from a minimal session snapshot and then uses
     `LoadSessionResponse._meta.workingDir`.
   - Subscribe to the session via `subscribeToAcpSession(sessionId, ...)`.
   - Seed a fresh `AcpSessionNotificationAdapter` from that list item's
     metadata (title, updatedAt, messageCount, providerId, modelId) so the
     UI has session info before replay finishes.

8. Done: replace the initial `resumeAgent` call with
   `acpLoadSession(sessionId, workingDir)`.
   - Still open: read `provider_name` / `model_name` / `model_id` from
     `LoadSessionResponse.models` directly.
   - Done: read `recipe` / `user_recipe_values` from
     `LoadSessionResponse._meta.recipe` / `_meta.userRecipeValues`.
   - Treat `acpLoadSession` resolution as "conversation replay complete."
     Do one final adapter flush, then call `onSessionLoaded` and flip out of
     `LoadingConversation`.

9. Done: bridge adapter updates into the existing reducer:
   - `messages` → `SET_MESSAGES`
   - `usage` → `SET_TOKEN_STATE`
   - `sessionInfo` → `SET_SESSION` (merged onto the seeded `Session` shape)
   - `error` → `SET_SESSION_LOAD_ERROR`
   - `extensionResults` → `showExtensionLoadResults(...)` (reuse the
     existing toast helper)
   - on `acpLoadSession` resolution: dispatch `SESSION_LOADED`, call
     `onSessionLoaded`.

10. Done: populate `resultsCache` after the load completes. The REST path does this
    for fast re-mounts; the ACP path should too.

11. Done: reset adapter state when `sessionId` changes (`RESET_FOR_NEW_SESSION`),
    and call the unsubscribe returned by `subscribeToAcpSession` in effect
    cleanup so navigation away unsubscribes cleanly.

12. Partially obsolete: live `sessionReply` and active-prompt cancel have moved
    to ACP. REST remains for `useSessionEvents` reattach/recovery,
    edit/fork, recipe apply, and name polling.

13. Drop unused `tokenState.{inputTokens, outputTokens, accumulatedTotalTokens}`
    from the client `TokenState` once REST is removed (gap doc notes these
    are never read).

## Behavior To Preserve

- loading state
- session load errors
- initial conversation display (text, thinking, tool calls)
- token state
- tool call history
- session name/info

## Completion Criteria

- Cold-loading an existing conversation no longer calls `resumeAgent` /
  `getSession`.
- ACP replayed updates produce the same visible conversation state as REST
  did, including thinking and tool history.
- `onSessionLoaded` still runs at the expected time.
- `resultsCache` is populated after replay completes.
- Live prompt, cancel, reattach, and name polling continue to work via the
  existing REST paths.

## Risks

- Replay notifications can be missed if handler registration happens too late.
  Mitigation: install the router and subscribe before calling `acpLoadSession`.
- `acpLoadSession` resolution is the ACP replay-complete boundary. Do not leave
  `LoadingConversation` or call `onSessionLoaded` before it resolves, even if
  progressive replay has already painted messages.
- Provider/extension setup may still be running after replay completes. Track
  that separately if the UI needs setup readiness or extension-load errors.
- Passing the wrong `cwd` to ACP `session/load` overwrites the saved working
  directory on the server. Mitigation: only use `SessionListItem.workingDir`;
  hard-fail on missing.
- If `useChatStream` reads `resumeAgent.session` and ACP replay at the same
  time, the load path has two competing sources of truth. Avoid this — the
  load slice is a clean swap, not a bridge.
- Extension load failures will be silent on ACP-loaded sessions until the
  follow-up ACP notification exists. Acceptable for now; flagged in
  follow-ups.
