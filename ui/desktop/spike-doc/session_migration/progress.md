# ACP Session Migration Progress

## Status

Current status: desktop chat is ACP-backed for the normal session lifecycle:
session list, cold conversation load, normal prompt streaming, active-prompt
cancel, active session mode changes, plain non-recipe session creation, and
inline ACP tool permission approval.

REST remains in targeted places: recipe / recipe-deeplink / extension-override
session creation, recipe parameter persistence and recipe prompt application
(`updateFromSession`), edit/fork `overrideConversation`, REST/SSE reattach and
buffer-overrun recovery paths, app-cache population, and a few metadata
fallbacks such as name polling and mode fallback reads.

Next: decide whether to remove or intentionally retain those remaining REST
paths. The biggest functional gaps are recipe-session parity and ACP reattach /
recovery semantics.

Owner: Codex

Last updated: 2026-05-25

Plan: `ui/desktop/spike-doc/session_migration/acp-session-migration-plan.md`

## Progress Checklist

### 1. Harden ACP Client

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

This foundation is now used by production chat load and prompt paths.

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

### 4. Conversation Load Slice (done for normal load)

Detailed plan: `05-conversation-load.md`.

Server prerequisites:

- [x] Attach `recipe` and `user_recipe_values` to
  `LoadSessionResponse._meta` for recipe rendering and param submission.
- [ ] Apply the rendered recipe to the agent's system prompt during ACP
  agent setup. Current desktop still calls REST `updateFromSession` after
  session load, so recipe prompt application is not yet ACP-native.
- [x] Decide on the extension-load-result surface and implement it. Current
  inline ACP load attaches `_meta.extensionResults` to `LoadSessionResponse`.
- [x] Attach `working_dir` to `LoadSessionResponse._meta` for entry points
  without a cached `SessionListItem`.

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

- [x] Wire `Client.extNotification(method, params)` in
  `ui/desktop/src/acp/acpConnection.ts` so `_goose/session/update` is
  dispatched alongside standard ACP `sessionUpdate`. Add a parallel
  `AcpGooseNotificationHandler` interface +
  `setAcpGooseNotificationHandler(...)` to mirror the existing
  notification-handler pattern.
- [x] Extend `sessionNotificationRouter` (or add a sibling router) to
  route `_goose/session/update` notifications by `sessionId`.
- [x] Install the notification router(s) once (chat hook mount or app boot).
- [x] Before calling `acpLoadSession`, subscribe to the session via
  `subscribeToAcpSession(sessionId, ...)` so replay notifications are not
  missed.
- [ ] Seed an `AcpSessionNotificationAdapter` from the matching
  `SessionListItem` (title, updatedAt, messageCount, providerId, modelId).
  Current code uses a minimal snapshot and gets final `workingDir` from
  `LoadSessionResponse._meta`.
- [x] Resolve `workingDir` safely. Inline ACP load ignores `args.cwd` and
  returns `_meta.workingDir`; the old destructive cwd update only exists behind
  `GOOSE_ACP_LEGACY_LOAD=1`.
- [x] Extend the adapter beyond text — `agent_thought_chunk`, `tool_call`,
  `tool_call_update`, standard ACP `usage_update` (used/size), custom
  `_goose/session/update` `usage_update` (accumulated tokens + cost),
  `session_info_update` (name / updatedAt / messageCount; also
  `extensionResults` if that surface is chosen).
- [ ] Read `provider_name` / `model_name` / `model_id` from
  `LoadSessionResponse.models` directly — ACP config options are consumed, but
  the loaded desktop `Session` snapshot does not yet merge model/provider
  fields from `models`.
- [x] Read `recipe` and `user_recipe_values` from
  `LoadSessionResponse._meta`.
- [x] Bridge adapter `messages` / `usage` / `sessionInfo` updates into the
  existing `SESSION_LOADED` / `SET_TOKEN_STATE` / `SET_SESSION` dispatches in
  `useChatStream`. Reuse `showExtensionLoadResults` for extension results.
- [x] Replace the REST `resumeAgent` conversation load path with
  `acpLoadSession(sessionId, workingDir)`.
- [x] Populate `resultsCache` after replay completes so re-mounts stay fast
  (the REST path already does this).
- [x] Preserve loading conversation state and load error state.
- [x] Treat `acpLoadSession` resolution as the conversation-replay-complete
  boundary. Do one final adapter flush, then dispatch `SESSION_LOADED`,
  call `onSessionLoaded`, and flip out of `LoadingConversation`. Do not
  leave `LoadingConversation` earlier even if progressive replay has
  already painted messages.
- [ ] Optionally flush the adapter snapshot on a short throttle while
  `acpLoadSession` is pending so long replays paint progressively.
- [x] Keep REST alive where it is still needed. Live prompt and active-prompt
  cancel have moved to ACP; REST remains for reattach, edit/fork, recipe apply,
  app-cache population, and metadata fallbacks.
- [x] Add adapter unit tests for thinking, tool calls, tool updates,
  standard usage, custom goose usage, and session info.
- [x] Manually verify an existing session loads through ACP with text,
  thinking, tool calls, recipe, cost chip, and extension errors all
  preserved.

Notes:

- ACP load now uses `crates/goose/src/acp/server/load_session.rs` inline mode
  by default, with `GOOSE_ACP_LEGACY_LOAD=1` as a rollback switch.
- Inline load rejects client-supplied `mcpServers`, replays conversation
  notifications, builds the agent synchronously, stores `AgentHandle::Ready`,
  emits usage updates, and returns `_meta.recipe`, `_meta.userRecipeValues`,
  `_meta.extensionResults`, and `_meta.workingDir`.
- The client load path creates an ACP adapter, subscribes before
  `acpLoadSession`, collects ACP and `_goose/session/update` notifications,
  dispatches `SESSION_LOADED`, shows extension results, and caches the result.
- Still not ACP-native: rendered recipe prompt application is covered by the
  existing REST `updateFromSession` effect.

### 5. Tool Permission Bridge (mostly done for visible ACP prompts)

`ui/desktop/src/acp/acpConnection.ts` still returns `cancelled` if no handler
is registered, but the normal ACP prompt path now registers a handler while the
prompt is active.

- [x] Define permission request UI state.
- [x] Bridge ACP `requestPermission` into chat UI.
- [x] Render tool approval request to user.
- [x] Map approve-once decision to ACP selected outcome.
- [x] Map approve-always decision to ACP selected outcome, if option exists.
- [x] Map reject-once decision to ACP selected outcome.
- [x] Map reject-always decision to ACP selected outcome, if option exists.
- [x] Map dismiss/cancel to ACP cancelled outcome.
- [x] Avoid assuming fixed ACP option IDs unless backend guarantees them.
- [x] Verify approved tool call continues.
- [x] Verify rejected tool call is handled cleanly.

Notes:

- Implemented in the visible chat prompt path: `useChatStream` registers
  `setAcpPermissionHandler`, adapts requests into existing
  `actionRequired/toolConfirmation` messages, and resolves by matching ACP
  option `kind`, not hardcoded option IDs.
- Remaining hardening: permission requests for hidden/background sessions or
  requests that outlive the current hook instance still need an app-level
  durable bridge if those scenarios become supported.

### 6. Live Prompt + Session Creation (paired)

These ship together. A new ACP-created session whose first user message
goes through REST `sessionReply` hits an untested agent-lifecycle path
(REST handler against ACP-spawned agent). ACP-created sessions must take
live prompts via ACP from their first message onward.

Live prompt:

- [x] Add `promptAcpSession`.
- [x] Reuse the session-scoped router for live prompt notifications.
- [x] Replace REST `sessionReply` for the normal text/image prompt path.
- [x] Set streaming state on submit; handle ACP completion and errors.
- [x] Preserve task completion desktop notification behavior.
- [x] Preserve `AppEvents.MESSAGE_STREAM_FINISHED` if still needed.
- [ ] Replace REST session-name refresh with `SessionInfoUpdate` handling.
- [x] Drop REST request-ID routing for normal ACP prompts — ACP scopes by
  `sessionId`.
- [x] Manually verify a text prompt streams through ACP.

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
- [x] Add `createAcpSession` / `acpNewSession`.
- [x] Replace REST `startAgent` usage in `createSession` for guarded plain
  non-recipe sessions.
- [x] Preserve `AppEvents.SESSION_CREATED`.
- [x] Preserve `AppEvents.ADD_ACTIVE_SESSION`.
- [x] Preserve `setView('pair', ...)` behavior.
- [x] Verify launcher / new chat flow.
- [ ] Verify recipe deeplink and recipe-ID flows (deeplink-launch reads
  `recipe.prompt` off the create response).
- [ ] Verify extension override flow.
- [ ] Verify recipe param-form submit re-applies the rendered recipe to
  the agent system prompt.

Notes:

- ACP creation intentionally excludes recipe, recipe-deeplink, explicit
  extension config, and extension-override paths. Those still use REST
  `startAgent`.
- Normal first prompts after ACP-created sessions use ACP `session/prompt`.

### 7. Cancellation Slice

- [x] Add `cancelAcpSession` / `acpCancelPrompt`.
- [x] Replace REST `sessionCancel` with ACP `session/cancel` for active ACP
  prompts.
- [x] Track ACP active prompt state.
- [x] Make stop a no-op when there is no active prompt.
- [x] Clear active prompt refs on cancel.
- [x] Return UI to idle or cancelled state.
- [ ] Verify cancellation does not leave stale loading/thinking state.

Notes:

- REST `sessionCancel` is still retained for REST/SSE active request reattach
  paths.

## Verification Checklist

Manual:

- [x] Load an existing session with plain text.
- [x] Send a prompt and receive streamed assistant text.
- [ ] Cancel a running prompt.
- [x] Load an existing session with thinking content.
- [x] Load an existing session with tool calls.
- [x] Send a prompt that triggers tool approval.
- [x] Approve a tool request.
- [x] Reject a tool request.
- [x] Create a new plain non-recipe session.
- [ ] Navigate away from and back to an active session.
- [ ] Navigate away from and back to a completed session.
- [ ] Confirm session name updates still appear.

Automated:

- [x] Router test for session-scoped dispatch.
- [x] Router test for multiple subscribers.
- [x] Router test for unsubscribe and double-unsubscribe.
- [x] Adapter test for text chunk accumulation.
- [x] Adapter test for thinking chunk conversion.
- [x] Adapter test for tool call conversion.
- [x] Adapter test for tool call update conversion.
- [x] Adapter test for usage update conversion.
- [x] Adapter test for session info update conversion.
- [x] Permission mapping test for approve.
- [x] Permission mapping test for reject.
- [x] Permission mapping test for cancel.

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
- [ ] Make recipe prompt application ACP-native and remove REST
  `updateFromSession` from loaded ACP sessions.
- [ ] Migrate or explicitly retain REST edit/fork `overrideConversation`.
- [ ] Document final `goosed` bridge removal requirements once desktop session/chat is ACP-backed.
- [ ] Add broader integration tests after the adapter and hook migration settle.
- [ ] Trim unused `TokenState` fields (`inputTokens`, `outputTokens`,
  `accumulatedTotalTokens`) once REST is removed — they're never read by the
  UI.
- [ ] Unify session-list token count vs schedule detail view on
  `accumulated_total_tokens`.
