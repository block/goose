# ACP Session Migration Progress

## Status

Current status: steps 1, 2A, 2B, 2C, and the session-list slice (step 8) are
complete. The router and adapter exist but are not yet consumed by any
production code. Next: the conversation-load slice (2D), which now needs both
server-side data additions and client work — see
`useChatStream-acp-data-gap.md` for the field-by-field audit and
`05-conversation-load.md` for the revised step list.

Owner: Codex

Last updated: 2026-05-19

Plan: `ui/desktop/spike-doc/session_migration/acp-session-migration-plan.md`

Working agreement: `ui/desktop/spike-doc/session_migration/working-agreement.md`

## Progress Checklist

### 1. Harden ACP Client

Detailed plan: `01-harden-acp-client.md`

- [x] Add `AcpNotificationHandler` interface.
- [x] Add `setAcpNotificationHandler(...)`.
- [x] Route ACP `sessionUpdate` notifications to the registered handler.
- [x] Add ACP permission handler interface.
- [x] Add `setAcpPermissionHandler(...)`.
- [x] Replace silent always-cancel permission behavior before live chat is enabled.
- [x] Confirm ACP reconnect still clears cached client state.

Notes:

- Updated `ui/desktop/src/acp/acpConnection.ts`.
- No behavior migration yet; Step 1 only adds integration points.
- If no permission handler is registered, ACP permission requests now warn before
  returning `cancelled`.
- Reconnect cleanup remains unchanged.
- Ran `pnpm exec prettier --write src/acp/acpConnection.ts`.
- Ran `pnpm run typecheck`; passed with the existing Node engine warning
  (`^24.10.0` wanted, shell has `v25.8.1`).

### 2. ACP Wrapper + Notification Router + Text Adapter

Detailed plans: `02-acp-session-wrapper.md`, `03-notification-adapter.md`

This is the foundation step. The router and adapter exist but are not yet
referenced by production code — step 4 (conversation load) is what wires
them in.

#### 2A. Minimal Session API Wrapper

- [x] Create `ui/desktop/src/acp/sessions.ts`.
- [x] Add `loadAcpSession`.
- [x] Keep load response narrow; do not infer messages from the direct response.
- [x] Keep wrapper free of React state and UI side effects.

#### 2B. Session Notification Router

- [x] Create `ui/desktop/src/acp/sessionNotificationRouter.ts`, or equivalent.
- [x] Add explicit router install function that registers one global ACP
  notification router with `setAcpNotificationHandler`.
- [x] Add session-scoped subscription/unsubscription routed by ACP `sessionId`.
- [x] Add router tests for session routing, multiple subscribers, unsubscribe,
  double-unsubscribe, and no-subscriber dispatch.

#### 2C. Minimal Text Adapter

- [x] Create `ui/desktop/src/acp/sessionNotificationAdapter.ts`.
- [x] Convert `user_message_chunk`.
- [x] Convert `agent_message_chunk`.
- [x] Convert minimal `session_info_update`, if needed for load.
- [x] Add adapter tests for text-only replay.

Notes:

- 2A added only `loadAcpSession(sessionId, workingDir)`. It calls ACP
  `session/load` with `sessionId`, `cwd`, and `mcpServers: []`, then returns the
  ACP `LoadSessionResponse` directly. Conversation content still must come from
  `session/update` notifications in later 2B-2D work.
- Ran `pnpm exec prettier --write src/acp/sessions.ts`.
- Ran `pnpm run typecheck`; passed with the existing Node engine warning
  (`^24.10.0` wanted, shell has `v25.8.1`).
- 2B added `installAcpSessionNotificationRouter()` and
  `subscribeToAcpSession(sessionId, listener)`. Router registration is explicit,
  not a module-load side effect.
- The installed router dispatches by `notification.sessionId`.
- Router unsubscribe is idempotent and removes empty session entries.
- Ran `pnpm exec prettier --write src/acp/sessionNotificationRouter.ts
  src/acp/sessionNotificationRouter.test.ts`.
- Ran `pnpm exec vitest run src/acp/sessionNotificationRouter.test.ts`; 6 tests
  passed.
- Ran `pnpm run typecheck`; passed with the existing Node engine warning
  (`^24.10.0` wanted, shell has `v25.8.1`).
- 2C added `createAcpSessionNotificationAdapter()`, which converts ACP text
  notifications into desktop `Message[]` updates and returns updates immediately
  per chunk. Progressive rendering still depends on hook integration dispatching
  each adapter update as notifications arrive.
- The minimal adapter intentionally ignores non-text content for this slice.
- Ran `pnpm exec prettier --write src/acp/sessionNotificationAdapter.ts
  src/acp/__tests__/sessionNotificationAdapter.test.ts`.
- Ran `pnpm exec vitest run src/acp/__tests__/sessionNotificationAdapter.test.ts
  src/acp/__tests__/sessionNotificationRouter.test.ts`; 12 tests passed.
- Ran `pnpm run typecheck`; passed with the existing Node engine warning
  (`^24.10.0` wanted, shell has `v25.8.1`).
### 3. Session List Slice (done)

Detailed plan: `02-acp-session-wrapper.md`

- [x] Add `acpListSessions`, `acpRenameSession`, `acpDeleteSession`,
  `acpForkSession`, `acpExportSession`, `acpImportSession` in
  `ui/desktop/src/acp/sessions.ts`.
- [x] Map ACP `SessionInfo` into desktop `SessionListItem`
  (`sessionInfoToListItem`), preserving `workingDir` for the load slice.
- [x] Migrate the session list UI off REST in `useNavigationSessions`,
  `SessionListView`, `SessionsList`, and `SessionsInsights`.

Notes:

- Landed first; now provides the metadata source the load slice depends on
  for `workingDir` and initial title/updatedAt/messageCount/provider/model.

### 4. Conversation Load Slice (current focus)

Detailed plan: `05-conversation-load.md`. Field-level audit:
`useChatStream-acp-data-gap.md` (authoritative reference).

Server prerequisites:

- [ ] Attach `recipe` and `user_recipe_values` to
  `LoadSessionResponse._meta` for recipe rendering and param submission.
- [ ] Apply the rendered recipe to the agent's system prompt during ACP
  agent setup (DB stores only the raw recipe + values; rendered prompt is
  computed at runtime via `build_recipe_with_parameter_values` +
  `apply_recipe_to_agent`, which today lives in REST `update_from_session`).
  Either auto-apply during `spawn_agent_setup` when recipe + all params
  are present, or expose a Goose-custom ACP request mirroring
  `update_from_session`. Without this, ACP-loaded recipe sessions render
  the recipe UI but the agent silently behaves as a plain chat.
- [ ] Decide on the extension-load-result surface (new
  `SessionUpdate::ExtensionLoadResult` variant vs.
  `SessionInfoUpdate._meta.extensionResults`) and implement it.
- [ ] (Optional cold-open improvement) attach `working_dir` to
  `LoadSessionResponse._meta` for entry points without a cached
  `SessionListItem`.

No replay-complete notification is needed. ACP `session/load` resolves only
after replay notifications have been sent, so the request resolution itself
is the authoritative replay-complete boundary. The extension-load-result
surface above models provider/extension setup readiness — that's a separate
concern from conversation replay.

Not needed: `accumulated_*` tokens and `accumulated_cost` are already
emitted via the existing custom `_goose/session/update` notification
(`GooseSessionNotification` in `crates/goose-sdk/src/custom_notifications.rs`,
called from `build_usage_updates` in `crates/goose/src/acp/server.rs`).

Client work:

- [ ] Wire `Client.extNotification(method, params)` in
  `ui/desktop/src/acp/acpConnection.ts` so `_goose/session/update` is
  dispatched alongside standard ACP `sessionUpdate`. Add a parallel
  `AcpGooseNotificationHandler` interface +
  `setAcpGooseNotificationHandler(...)` to mirror the existing
  notification-handler pattern.
- [ ] Extend `sessionNotificationRouter` (or add a sibling router) to
  route `_goose/session/update` notifications by `sessionId`.
- [ ] Install the notification router(s) once (chat hook mount or app boot).
- [ ] Before calling `acpLoadSession`, subscribe to the session via
  `subscribeToAcpSession(sessionId, ...)` so replay notifications are not
  missed.
- [ ] Seed an `AcpSessionNotificationAdapter` from the matching
  `SessionListItem` (title, updatedAt, messageCount, providerId, modelId).
- [ ] Resolve `workingDir` from `SessionListItem.workingDir` (or
  `LoadSessionResponse._meta.workingDir` once server step lands); surface a
  load error if neither has it. Do not fall back to
  `getInitialWorkingDir()` because Goose updates the saved working directory
  from the ACP `session/load` request
  (`crates/goose/src/acp/server.rs` `on_load_session`).
- [ ] Extend the adapter beyond text — `agent_thought_chunk`, `tool_call`,
  `tool_call_update`, standard ACP `usage_update` (used/size), custom
  `_goose/session/update` `usage_update` (accumulated tokens + cost),
  `session_info_update` (name / updatedAt / messageCount; also
  `extensionResults` if that surface is chosen).
- [ ] Read `provider_name` / `model_name` / `model_id` from
  `LoadSessionResponse.models` directly — first-class, no `_meta` seeding.
- [ ] Read `recipe` and `user_recipe_values` from
  `LoadSessionResponse._meta`.
- [ ] Bridge adapter `messages` / `usage` / `sessionInfo` updates into the
  existing `SESSION_LOADED` / `SET_TOKEN_STATE` / `SET_SESSION` dispatches in
  `useChatStream`. Reuse `showExtensionLoadResults` for extension results.
- [ ] Replace the REST `resumeAgent` conversation load path with
  `acpLoadSession(sessionId, workingDir)`.
- [ ] Populate `resultsCache` after replay completes so re-mounts stay fast
  (the REST path already does this).
- [ ] Preserve loading conversation state and load error state.
- [ ] Treat `acpLoadSession` resolution as the conversation-replay-complete
  boundary. Do one final adapter flush, then dispatch `SESSION_LOADED`,
  call `onSessionLoaded`, and flip out of `LoadingConversation`. Do not
  leave `LoadingConversation` earlier even if progressive replay has
  already painted messages.
- [ ] Optionally flush the adapter snapshot on a short throttle while
  `acpLoadSession` is pending so long replays paint progressively.
- [ ] Keep REST alive for live prompt, cancel, reattach, and name polling —
  the load slice intentionally does not touch them.
- [ ] Add adapter unit tests for thinking, tool calls, tool updates,
  standard usage, custom goose usage, and session info.
- [ ] Manually verify an existing session loads through ACP with text,
  thinking, tool calls, recipe, cost chip, and extension errors all
  preserved.

Notes:

- TBD

### 5. Tool Permission Bridge

Detailed plan: `08-tool-permissions.md`

Must land before live prompt — `ui/desktop/src/acp/acpConnection.ts`
currently returns `cancelled` if no handler is registered, so the first
prompt that needs approval would silently die. Can be drafted in parallel
with the load slice.

- [ ] Define permission request UI state.
- [ ] Bridge ACP `requestPermission` into chat UI.
- [ ] Render tool approval request to user.
- [ ] Map approve-once decision to ACP selected outcome.
- [ ] Map approve-always decision to ACP selected outcome, if option exists.
- [ ] Map reject-once decision to ACP selected outcome.
- [ ] Map reject-always decision to ACP selected outcome, if option exists.
- [ ] Map dismiss/cancel to ACP cancelled outcome.
- [ ] Avoid assuming fixed ACP option IDs unless backend guarantees them.
- [ ] Verify approved tool call continues.
- [ ] Verify rejected tool call is handled cleanly.

Notes:

- Originally sequenced after live prompt; moved earlier because permission
  is a prerequisite, not a follow-up.

### 6. Live Prompt + Session Creation (paired)

Detailed plans: `06-live-prompt-streaming.md`, `04-session-creation.md`,
`03-notification-adapter.md`.

These ship together. A new ACP-created session whose first user message
goes through REST `sessionReply` hits an untested agent-lifecycle path
(REST handler against ACP-spawned agent). ACP-created sessions must take
live prompts via ACP from their first message onward.

Live prompt:

- [ ] Add `promptAcpSession`.
- [ ] Reuse the session-scoped router for live prompt notifications.
- [ ] Replace REST `sessionReply` for the text prompt path.
- [ ] Set streaming state on submit; handle ACP completion and errors.
- [ ] Preserve task completion desktop notification behavior.
- [ ] Preserve `AppEvents.MESSAGE_STREAM_FINISHED` if still needed.
- [ ] Replace REST session-name refresh with `SessionInfoUpdate` handling.
- [ ] Drop REST request-ID routing — ACP scopes by `sessionId`.
- [ ] Manually verify a text prompt streams through ACP.

Session creation:

- [ ] Server: accept `recipe` / `recipe_id` and `extension_overrides` on
  ACP `session/new` (currently `on_new_session` only honors
  `_meta.provider`, `_meta.projectId`, `_meta.client`).
- [ ] Server: return the resolved `recipe` on
  `NewSessionResponse._meta.recipe` so `App.tsx`
  `resolveSessionInitialMessage` can read `recipe.prompt` for the
  deeplink-launch initial-message path. `user_recipe_values` is not
  needed on the create response (it's empty on a fresh session and
  populated later via param submit).
- [ ] Server: apply the rendered recipe to the agent's system prompt
  during create-time `spawn_agent_setup` when all params are present
  (same mechanism as slice 4's recipe-apply prereq — reuse whichever
  path that slice picks).
- [ ] Replace REST `updateSessionUserRecipeValues` with an ACP equivalent
  for the recipe-param-submit path. After param submit, the agent's
  system prompt must be re-rendered + re-applied — today this happens
  via `updateFromSession` triggered by `useEffect` on `state.session`.
  ACP should expose either a Goose-custom request that updates values
  AND re-applies, or pair a values-update notification with a
  re-apply mechanism.
- [ ] Add `createAcpSession`.
- [ ] Replace REST `startAgent` usage in `createSession`.
- [ ] Preserve `AppEvents.SESSION_CREATED`.
- [ ] Preserve `AppEvents.ADD_ACTIVE_SESSION`.
- [ ] Preserve `setView('pair', ...)` behavior.
- [ ] Verify launcher / new chat flow.
- [ ] Verify recipe deeplink and recipe-ID flows (deeplink-launch reads
  `recipe.prompt` off the create response).
- [ ] Verify extension override flow.
- [ ] Verify recipe param-form submit re-applies the rendered recipe to
  the agent system prompt.

Notes:

- TBD

### 7. Cancellation Slice

Detailed plan: `07-cancellation.md`

- [ ] Add `cancelAcpSession`.
- [ ] Replace REST `sessionCancel` with ACP `session/cancel`.
- [ ] Track ACP active prompt state.
- [ ] Make stop a no-op when there is no active prompt.
- [ ] Clear active prompt refs on cancel.
- [ ] Return UI to idle or cancelled state.
- [ ] Verify cancellation does not leave stale loading/thinking state.

Notes:

- TBD

### 8. Remove Desktop REST Session Usage

Detailed plan: `09-rest-cleanup.md`

- [ ] Search for session REST imports in `ui/desktop/src`.
- [ ] Remove replaced `sessionReply` usage.
- [ ] Remove replaced `sessionCancel` usage.
- [ ] Remove replaced `sessionEvents` usage.
- [ ] Remove replaced `resumeAgent` usage.
- [ ] Remove replaced `getSession` usage for migrated chat loading.
- [ ] Remove replaced `startAgent` usage.
- [ ] Drop unused `tokenState.{inputTokens, outputTokens,
  accumulatedTotalTokens}` (gap doc: never read by the UI).
- [ ] Update tests that mocked REST session APIs.
- [ ] Keep unrelated REST APIs untouched.
- [ ] Do not manually edit `ui/desktop/openapi.json`.

Notes:

- TBD

## Verification Checklist

Manual:

- [ ] Load an existing session with plain text.
- [ ] Send a prompt and receive streamed assistant text.
- [ ] Cancel a running prompt.
- [ ] Load an existing session with thinking content.
- [ ] Load an existing session with tool calls.
- [ ] Send a prompt that triggers tool approval.
- [ ] Approve a tool request.
- [ ] Reject a tool request.
- [ ] Create a new session.
- [ ] Navigate away from and back to an active session.
- [ ] Navigate away from and back to a completed session.
- [ ] Confirm session name updates still appear.

Automated:

- [ ] Router test for session-scoped dispatch.
- [ ] Router test for multiple subscribers.
- [ ] Router test for unsubscribe and double-unsubscribe.
- [ ] Adapter test for text chunk accumulation.
- [ ] Adapter test for thinking chunk conversion.
- [ ] Adapter test for tool call conversion.
- [ ] Adapter test for tool call update conversion.
- [ ] Adapter test for usage update conversion.
- [ ] Adapter test for session info update conversion.
- [ ] Permission mapping test for approve.
- [ ] Permission mapping test for reject.
- [ ] Permission mapping test for cancel.

## Blockers

- None recorded.

## Decisions

- No feature flag for migrated session behavior.
- No automatic REST fallback for ACP chat errors.
- REST remains only for session behavior not migrated yet.
- Extension migration is out of scope for this plan.

## Follow-Up

Track work that should not block the first ACP session migration PR.

- [ ] Remove server REST session endpoints after desktop no longer depends on them.
- [ ] Decide whether ACP should support full recipe deeplink/session creation parity.
- [ ] Decide whether ACP should support extension override inputs during session creation.
- [ ] Replace any remaining REST session metadata refresh with ACP metadata APIs.
- [ ] Review reattach semantics for prompts that continue while the view is remounted.
- [ ] Document final `goosed` bridge removal requirements once desktop session/chat is ACP-backed.
- [ ] Add broader integration tests after the adapter and hook migration settle.
- [ ] Trim unused `TokenState` fields (`inputTokens`, `outputTokens`,
  `accumulatedTotalTokens`) once REST is removed — they're never read by the
  UI.
- [ ] Unify session-list token count vs schedule detail view on
  `accumulated_total_tokens` (see
  `ui/desktop/spike-doc/session-list-token-count-inconsistency.md`).
