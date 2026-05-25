# Desktop ACP Session Migration Plan

## Goal

Move `ui/desktop` session and chat runtime behavior from REST/OpenAPI to ACP.

This plan assumes the end goal is ACP-only for migrated session behavior. It does
not use a feature flag for each migrated call. REST remains only for areas that
have not moved yet.

## Scope

In scope:

- Session creation.
- Session loading and conversation replay.
- Live prompt streaming.
- Prompt cancellation.
- ACP session update handling.
- ACP tool permission handling.
- Desktop-side REST session import cleanup after the UI no longer uses those calls.

Out of scope:

- Extension list migration.
- Extension mutation migration.
- Removing server REST endpoints in the same change, unless usage is clearly gone
  and the change remains easy to verify.

## Current REST Session Shape

Important desktop files:

- `ui/desktop/src/sessions.ts`
  - `createSession`
  - `startNewSession`
  - `resumeSession`
- `ui/desktop/src/hooks/useChatStream.ts`
  - `resumeAgent`
  - `getSession`
  - `sessionReply`
  - `sessionCancel`
  - `updateFromSession`
- `ui/desktop/src/hooks/useSessionEvents.ts`
  - `GET /sessions/{id}/events` SSE
  - `ActiveRequests`
  - `request_id` / `chat_request_id` routing

The current UI expects REST/SSE events such as:

- `Message`
- `Finish`
- `Error`
- `Notification`
- `UpdateConversation`
- `ActiveRequests`

## Target ACP Session Shape

Use the existing ACP WebSocket client path from `ui/desktop/src/acp`.

ACP methods to use:

- `session/new`
- `session/load`
- `session/prompt`
- `session/cancel`
- `session/list`, if session list is included in the change

ACP notifications to handle:

- `session/update`
- `agent_message_chunk`
- `user_message_chunk`
- `agent_thought_chunk`
- `tool_call`
- `tool_call_update`
- `usage_update`
- `session_info_update`
- `config_option_update`

ACP client requests to handle:

- `requestPermission`

## No Feature Flag Rule

Do not add a REST/ACP feature flag for the same migrated behavior.

Instead:

- A behavior that has moved to ACP should call ACP directly.
- REST remains only for behaviors not yet migrated.
- Do not add automatic REST fallback for ACP chat failures.
- ACP errors should surface as ACP errors so parity gaps are fixed directly.

This avoids keeping REST response objects and ACP response objects alive at the
same call site.

## Vertical Slice Plan

The migration proceeds as vertical slices. Each slice should prove one
user-visible path end to end before adding more ACP surface area. ACP session
behavior is notification-driven, so a wrapper can typecheck without proving
that chat actually works — the proof comes from wiring the wrapper, router,
adapter, and React state together against a real session.

Current status and next steps live in `progress.md`.

### Current Sequencing Decision

The remaining work should proceed in this order:

1. Close message/content parity gaps.
   - Start with `systemNotification` replacement and live status updates.
   - Then audit image replay, `redactedThinking`, `frontendToolRequest`, and
     legacy `toolConfirmationRequest`.
2. Defer recipe parity.
   - Keep recipe creation, recipe deeplink, recipe parameter persistence, and
     recipe prompt application on REST for now.
   - Revisit recipe migration after deciding the ACP shape for persisting
     values, rendering the recipe, and applying the rendered prompt to the
     agent system prompt.

### Why load-first

Load is the right entry point for the chat-runtime migration. The reasoning,
in priority order:

1. **Rollback boundary.** Load only touches the cold-read path. A regression
   means falling back to REST `resumeAgent` with one import swap; submitting
   messages still works because submit was never touched. Creation, by
   contrast, is the entry point to every new chat — a regression there
   blocks all new work.
2. **Lifecycle hazard with create-first.** ACP `on_new_session`
   (`crates/goose/src/acp/server.rs`) spawns its own agent setup under
   `self.sessions` keyed by ACP `SessionId`. REST `startAgent` uses the REST
   agent registry. Migrating creation while leaving live prompt on REST
   means REST `sessionReply` would run against an ACP-spawned agent — an
   untested combination. Load avoids that entanglement entirely.
3. **Adapter de-risk.** Load exercises the notification router and adapter
   in a deterministic replay context where you can A/B against
   `resumeAgent`. Save bug-discovery for a read path; don't combine it with
   the live-prompt cutover.
4. **Gap shape favors load.** Load's remaining server gaps (recipe /
   extension-results on the load response) are data-carrying additions to
   existing payloads. Create's server gaps (recipe input + extension
   overrides on `session/new`) change agent setup behavior, which is
   structurally riskier.

Concrete sequencing implications:

- Permission did need to land before live prompt. The visible ACP prompt path
  now registers a permission handler; the no-handler fallback still returns
  `cancelled`.
- Plain ACP creation and live prompt now work together: an ACP-created plain
  session's first message uses ACP `session/prompt`, not REST `sessionReply`.

### 1. Harden ACP Client — done

`ui/desktop/src/acp/acpConnection.ts` exposes `setAcpNotificationHandler`,
`setAcpPermissionHandler`, and forwards ACP `sessionUpdate` notifications.

### 2. ACP Wrapper + Notification Router + Text Adapter — done

- `acpLoadSession` and the rest of the session helpers live in
  `ui/desktop/src/acp/sessions.ts`.
- The session-scoped router (`installAcpSessionNotificationRouter`,
  `subscribeToAcpSession`) lives in
  `ui/desktop/src/acp/sessionNotificationRouter.ts`.
- A minimal text adapter
  (`ui/desktop/src/acp/sessionNotificationAdapter.ts`) handles
  `user_message_chunk`, `agent_message_chunk`, and a stub
  `session_info_update`.

These pieces are now used by the production chat load and prompt paths.

### 3. Session List Slice — done

Session list migration landed first and is in production. ACP exports:
`acpListSessions`, `acpRenameSession`, `acpDeleteSession`, `acpForkSession`,
`acpExportSession`, `acpImportSession`. `sessionInfoToListItem` maps
`SessionInfo` into desktop `SessionListItem`. The session list is the
authoritative metadata source for the load slice (`workingDir`, title,
provider, model).

### 4. Conversation Load Slice — mostly done

Detailed plan: `05-conversation-load.md`.

REST `resumeAgent` cold-load has been replaced with ACP `session/load` +
`SessionUpdate` notifications. The adapter now handles text, thinking, tool
calls, tool updates, usage, config options, elicitation updates, and the custom
`_goose/session/update` usage payload.

Server prerequisites:

- Done: attach `recipe`, `userRecipeValues`, `extensionResults`, and
  `workingDir` to `LoadSessionResponse._meta` in inline ACP load.
- Still open: apply the rendered recipe to the agent's system prompt during ACP agent
  setup (lifecycle). The DB stores only the raw recipe + values; the
  rendered prompt is computed at runtime and currently pushed by REST
  `update_from_session`. The desktop still calls REST `updateFromSession`
  after load, so recipe behavior is preserved but not ACP-native.

No replay-complete notification needed. ACP `session/load` only resolves
after replay notifications have been sent, so the request resolution
itself is the authoritative replay-complete boundary.

Already in place: the rich usage payload (`accumulated_input_tokens`,
`accumulated_output_tokens`, `accumulated_cost`) is emitted via the existing
custom `_goose/session/update` notification (`GooseSessionNotification`).
The client picks it up via `Client.extNotification`.

Client status:

- Done: `Client.extNotification(method, params)` dispatches
  `_goose/session/update` through a Goose-session router.
- Done: `useChatStream` subscribes before `acpLoadSession`, dispatches
  `SESSION_LOADED` on ACP request resolution, shows extension results, stores
  ACP config options, and populates `resultsCache`.
- Open: the loaded desktop `Session` snapshot still does not merge
  model/provider fields from `LoadSessionResponse.models`, and progressive
  replay painting during a long `session/load` remains optional.

Do not make `useChatStream` depend on both `resumeAgent.session.conversation`
and ACP replay as competing sources. REST reattach, recipe apply, edit/fork,
app-cache population, and name polling stay alive; live prompt and active
prompt cancel have moved to ACP.

### 5. Tool Permission Bridge — done for visible ACP prompts

ACP `requestPermission` is bridged into the existing inline tool approval UI
for the active visible ACP prompt. The bridge adapts permission requests into
`actionRequired/toolConfirmation` messages and resolves ACP outcomes by option
`kind`.

Remaining hardening is app-level durability for hidden/background sessions or
permission requests that outlive the current chat hook instance.

### 6. Live Prompt Slice + Session Creation Slice — partially done

Normal live prompts now use ACP `session/prompt`, so ACP-created plain sessions
do not fall back to REST `sessionReply` for their first message. Plain
non-recipe session creation also uses ACP `session/new`; recipe and extension
override creation paths intentionally remain on REST.

Live prompt:

- Done: add `promptAcpSession`.
- Done: reuse the router and adapter wired in the load slice.
- Done: set streaming state on submit, handle ACP completion / errors / late
  notifications.
- Done for normal ACP prompts: drop REST request-ID routing — ACP scopes by
  `sessionId`.
- Open: replace REST session-name refresh with `SessionInfoUpdate` handling.

Session creation:

- Done for guarded plain sessions: `createSession` calls ACP `session/new`
  when there is no recipe, recipe deeplink, explicit extension config, or
  extension override state.
- Done: preserve `AppEvents.SESSION_CREATED`, `AppEvents.ADD_ACTIVE_SESSION`,
  and the `setView('pair', ...)` navigation.
- Open: recipe, recipe-deeplink, and extension-override parity for ACP
  `session/new`.
- Confirm recipe-deeplink and extension-override entry points still work.

### 7. Cancellation Slice — done for active ACP prompts

Active ACP prompts now use `session/cancel` through `acpCancelPrompt`.
REST `sessionCancel` remains for REST/SSE active-request reattach paths.

## Detail Docs

Only active/reference detail docs remain in this folder. Completed historical
slice plans were deleted after their status was folded into `progress.md` and
this plan.

| Slice | Detail file(s) |
|---|---|
| Current status | `progress.md` |
| Overall plan | `acp-session-migration-plan.md` |
| Conversation load | `05-conversation-load.md` |
| Inline load backend rationale | `10-on-load-session-rewrite.md` |
| ACP reply reference | `14-acp-reply-spike-plan.md` |
| ACP new-session reference | `15-acp-new-session-plan.md` |
| Message parity | `16-acp-message-parity-audit.md` |

## Main Risks

### Message Conversion

The highest-risk part is converting ACP session updates into the current desktop
message model. The raw ACP method calls are comparatively small.

### Tool Permission

If `requestPermission` is not wired, tool calls that require approval will fail
or cancel. This should be treated as a blocker for live ACP chat.

### Recipe and Extension Override Parity

Current REST session creation supports recipe and extension override inputs. ACP
session creation may need backend additions before it can fully replace REST
creation for all desktop entry points.

Recipe migration is intentionally deferred until after message parity. Avoid
partially migrating recipe creation without a settled server-side
render-and-apply mechanism.

### Reattach Semantics

The REST path has `ActiveRequests` and request ID based reattach logic. ACP uses
the session-scoped WebSocket notification stream instead. If reattach after view
remount is required, design it around ACP session state rather than preserving
REST request IDs.

### Session Name Refresh

The REST path refreshes session names with `getSession` after early replies. ACP
should prefer `session_info_update` or another ACP-backed session metadata path.

## Suggested Verification

Manual checks:

- Create a new session.
- Load an existing session with text, thinking, and tool calls.
- Send a prompt and receive streamed assistant text.
- Run a prompt that triggers tool approval.
- Approve and reject a tool request.
- Cancel a running prompt.
- Navigate away from and back to an active or recently completed session.
- Confirm session name updates still appear.

Automated checks:

- Unit test the ACP notification adapter with representative update sequences.
- Add tests for text chunk accumulation.
- Add tests for tool call and tool call update conversion.
- Add tests for usage update conversion.
- Add tests for permission request outcome mapping.

## Review Boundary

The PR should present one clear architecture change:

```text
Desktop session/chat runtime moves from REST/SSE to ACP WebSocket.
ACP adapter owns protocol translation.
Existing UI components continue to consume desktop chat state.
```

Keep UI component changes minimal where possible. Most complexity should live in
the ACP session wrapper and notification adapter.
