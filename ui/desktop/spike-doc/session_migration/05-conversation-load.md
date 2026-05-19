# 05 Conversation Load

## Goal

Replace REST `getSession` / `resumeAgent` cold-load with ACP `session/load`.

Do not partially read `resumeAgent.session` while also replaying ACP messages.
The session-list slice already moved to ACP, so `SessionListItem` is the
authoritative metadata source for cold load. Keep REST for live prompt,
cancel, reattach, and name polling — those are out of scope for this slice.

## State of the World

- `ui/desktop/src/acp/sessions.ts` already exports `acpLoadSession`,
  `acpListSessions`, and friends.
- `ui/desktop/src/acp/sessionNotificationRouter.ts` exports
  `installAcpSessionNotificationRouter` and `subscribeToAcpSession` but is not
  yet referenced by production code.
- `ui/desktop/src/acp/sessionNotificationAdapter.ts` only handles
  `user_message_chunk`, `agent_message_chunk`, and a minimal
  `session_info_update`. It needs to grow before this slice ships.
- Server `on_load_session` (`crates/goose/src/acp/server.rs`) emits per-message
  replay chunks (`UserMessageChunk`, `AgentMessageChunk`, `ToolCall`,
  `ToolCallUpdate`, `AgentThoughtChunk`), then `UsageUpdate`, then
  `available_commands_update`, then resolves the `session/load` request. Per
  ACP, `session/load` resolution is the replay-complete boundary.
- `on_load_session` overwrites the session's saved `working_dir` from the
  request `cwd`. Passing the wrong `cwd` is destructive.
- `LoadSessionResponse.models` already carries `current_model_id` /
  `ModelInfo`, so provider and model can be read directly from the response —
  no metadata seeding needed.
- The detailed field-by-field audit lives in
  `ui/desktop/spike-doc/useChatStream-acp-data-gap.md`. Pin that doc when
  implementing — the table below summarises but the gap doc is authoritative.

## Data Gaps (from useChatStream-acp-data-gap.md)

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

Still missing for UI parity with `resumeAgent`:

| Field | Used for | Suggested server fix |
|---|---|---|
| `recipe` (full JSON) | recipe rendering, param substitution | `LoadSessionResponse._meta.recipe` |
| `user_recipe_values` | recipe params submit | `LoadSessionResponse._meta.userRecipeValues` |
| `extension_results` | extension load error toasts | new `SessionUpdate::ExtensionLoadResult` variant, or `SessionInfoUpdate._meta.extensionResults` after setup |
| rendered recipe → agent system prompt | recipe behavior on load (today: REST `update_from_session` after `resumeAgent`) | auto-apply during ACP `spawn_agent_setup`, or expose a Goose-custom ACP request that mirrors `update_from_session` |

Until these land, the load slice cannot fully replace `resumeAgent` without
visible UI regressions (broken recipe UI, missing extension-error toasts)
or silent behavioral regression (recipe UI renders but agent does not
follow the recipe).

## Files

- `ui/desktop/src/hooks/useChatStream.ts`
- `ui/desktop/src/acp/sessionNotificationAdapter.ts`

## Implementation Steps

### Server prerequisites (do these first)

1. `crates/goose/src/acp/server.rs` `on_load_session`: attach `recipe`,
   `user_recipe_values`, and (until session-list metadata seeding is wired)
   `working_dir` to `LoadSessionResponse._meta` so the response is
   self-contained for deep-link / cold-open.
2. Apply the rendered recipe to the agent's system prompt during ACP agent
   setup. The DB only persists the raw `recipe` + `user_recipe_values`; the
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
3. Decide the extension-load-result surface: a new
   `SessionUpdate::ExtensionLoadResult` variant or
   `SessionInfoUpdate._meta.extensionResults` after agent setup completes.
   This should model provider/extension setup readiness, not conversation
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

4. Wire `Client.extNotification(method, params)` in
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

5. Extend the adapter beyond text in the same change as the hook migration:
   - `agent_thought_chunk` → thinking content on the assistant message
   - `tool_call` → desktop tool request shape
   - `tool_call_update` → desktop tool response/update shape
   - standard ACP `usage_update` → `TokenState.totalTokens` + context limit
   - custom `_goose/session/update` `usage_update` → `TokenState`
     accumulated tokens and cost
   - `session_info_update` → session name / updatedAt / messageCount merge;
     also `extensionResults` if that surface is chosen

   Text-only replay is not a useful UI milestone for any non-trivial session.

6. Install the router once. Pick the chat hook mount or app boot — the call
   is idempotent (`installed` guard inside the module). React hooks must not
   call `setAcpNotificationHandler` directly.

7. Inside `useChatStream`, before calling `acpLoadSession`:
   - Look up the matching `SessionListItem` for `sessionId`.
   - If no entry has a `workingDir`, fall back to
     `LoadSessionResponse._meta.workingDir` once server step 2 lands; until
     then, dispatch a load error.
   - Subscribe to the session via `subscribeToAcpSession(sessionId, ...)`.
   - Seed a fresh `AcpSessionNotificationAdapter` from that list item's
     metadata (title, updatedAt, messageCount, providerId, modelId) so the
     UI has session info before replay finishes.

8. Replace the initial `resumeAgent` call with
   `acpLoadSession(sessionId, workingDir)`.
   - Read `provider_name` / `model_name` / `model_id` from
     `LoadSessionResponse.models` directly.
   - Read `recipe` / `user_recipe_values` from
     `LoadSessionResponse._meta.recipe` / `_meta.userRecipeValues`.
   - Treat `acpLoadSession` resolution as "conversation replay complete."
     Do one final adapter flush, then call `onSessionLoaded` and flip out of
     `LoadingConversation`.

9. Bridge adapter updates into the existing reducer:
   - `messages` → `SET_MESSAGES`
   - `usage` → `SET_TOKEN_STATE`
   - `sessionInfo` → `SET_SESSION` (merged onto the seeded `Session` shape)
   - `error` → `SET_SESSION_LOAD_ERROR`
   - `extensionResults` → `showExtensionLoadResults(...)` (reuse the
     existing toast helper)
   - on `acpLoadSession` resolution: dispatch `SESSION_LOADED`, call
     `onSessionLoaded`.

10. Populate `resultsCache` after the load completes. The REST path does this
    for fast re-mounts; the ACP path should too.

11. Reset adapter state when `sessionId` changes (`RESET_FOR_NEW_SESSION`),
    and call the unsubscribe returned by `subscribeToAcpSession` in effect
    cleanup so navigation away unsubscribes cleanly.

12. Keep `useSessionEvents`, `sessionReply`, `sessionCancel`, and the name
    polling REST calls alive — they belong to slices 3, 4, and the metadata
    follow-up.

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
