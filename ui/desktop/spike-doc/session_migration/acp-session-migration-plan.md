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

Working agreement: `working-agreement.md`

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

- Permission must land before live prompt. The ACP client currently returns
  `cancelled` from `requestPermission` if no handler is registered
  (`ui/desktop/src/acp/acpConnection.ts`), so the first prompt that needs
  approval would silently die.
- Creation and live prompt must ship together, not separately. A new
  ACP-created session whose first message goes through REST `sessionReply`
  hits the same hazard as point 2.

### 1. Harden ACP Client — done

Detailed plan: `01-harden-acp-client.md`

`ui/desktop/src/acp/acpConnection.ts` exposes `setAcpNotificationHandler`,
`setAcpPermissionHandler`, and forwards ACP `sessionUpdate` notifications.

### 2. ACP Wrapper + Notification Router + Text Adapter — done

Detailed plans: `02-acp-session-wrapper.md`, `03-notification-adapter.md`

- `acpLoadSession` and the rest of the session helpers live in
  `ui/desktop/src/acp/sessions.ts`.
- The session-scoped router (`installAcpSessionNotificationRouter`,
  `subscribeToAcpSession`) lives in
  `ui/desktop/src/acp/sessionNotificationRouter.ts`.
- A minimal text adapter
  (`ui/desktop/src/acp/sessionNotificationAdapter.ts`) handles
  `user_message_chunk`, `agent_message_chunk`, and a stub
  `session_info_update`.

These pieces are not yet referenced by production code. The load slice
(step 4) is what wires them in.

### 3. Session List Slice — done

Detailed plan: `02-acp-session-wrapper.md`

Session list migration landed first and is in production. ACP exports:
`acpListSessions`, `acpRenameSession`, `acpDeleteSession`, `acpForkSession`,
`acpExportSession`, `acpImportSession`. `sessionInfoToListItem` maps
`SessionInfo` into desktop `SessionListItem`. The session list is the
authoritative metadata source for the load slice (`workingDir`, title,
provider, model).

### 4. Conversation Load Slice — current focus

Detailed plan: `05-conversation-load.md`. Field-level audit:
`ui/desktop/spike-doc/useChatStream-acp-data-gap.md`.

Replace REST `resumeAgent` cold-load with ACP `session/load` +
`SessionUpdate` notifications. This slice expands the adapter beyond text
(thinking, tool calls, tool updates, usage, full session info) and bridges
adapter output into `useChatStream`'s reducer.

Server prerequisites:

- Attach `recipe` and `user_recipe_values` to `LoadSessionResponse._meta`
  (data carrying).
- Apply the rendered recipe to the agent's system prompt during ACP agent
  setup (lifecycle). The DB stores only the raw recipe + values; the
  rendered prompt is computed at runtime and currently pushed by REST
  `update_from_session`. ACP has no equivalent, so without this step
  ACP-loaded recipe sessions render the recipe UI but the agent silently
  behaves as a plain chat. Either auto-apply during `spawn_agent_setup`
  or expose a Goose-custom ACP request mirroring `update_from_session`.
- Decide and implement the extension-load-result surface
  (`SessionUpdate::ExtensionLoadResult` variant or
  `SessionInfoUpdate._meta.extensionResults`). This models
  provider/extension setup readiness — separate from conversation replay
  completion.

No replay-complete notification needed. ACP `session/load` only resolves
after replay notifications have been sent, so the request resolution
itself is the authoritative replay-complete boundary.

Already in place: the rich usage payload (`accumulated_input_tokens`,
`accumulated_output_tokens`, `accumulated_cost`) is emitted via the existing
custom `_goose/session/update` notification (`GooseSessionNotification`).
The client work below picks it up via `Client.extNotification`.

Client prerequisite specific to this slice:

- Wire `Client.extNotification(method, params)` in `acpConnection.ts` so
  the custom `_goose/session/update` notification is dispatched alongside
  standard ACP `sessionUpdate`.

Do not make `useChatStream` depend on both `resumeAgent.session.conversation`
and ACP replay as competing sources. REST live prompt, cancel, reattach,
and name polling stay alive — this slice only swaps the cold-load path.

### 5. Tool Permission Bridge

Detailed plan: `08-tool-permissions.md`

Must land before live prompt. Without it, any prompt that triggers a tool
approval will return `cancelled` from `requestPermission` and die.

Bridge the ACP `requestPermission` callback into the existing UI approval
pattern. Map approve / reject / cancel into ACP outcomes.

Can be drafted in parallel with the load slice — it touches a different
ACP integration point.

### 6. Live Prompt Slice + Session Creation Slice — paired

Detailed plans: `06-live-prompt-streaming.md`, `04-session-creation.md`,
`03-notification-adapter.md`.

These ship together. A new ACP-created session whose first user message
goes through REST `sessionReply` hits an untested agent-lifecycle path
(REST handler against ACP-spawned agent). The cleanest cut is:
ACP-created sessions only ever take live prompts via ACP.

Live prompt:

- Add `promptAcpSession`.
- Reuse the router and adapter wired in the load slice.
- Set streaming state on submit, handle ACP completion / errors / late
  notifications.
- Drop REST request-ID routing — ACP scopes by `sessionId`.
- Replace REST session-name refresh with `SessionInfoUpdate` handling.

Session creation:

- Add `createAcpSession`.
- Server prerequisites: accept `recipe` / `recipe_id` and
  `extension_overrides` on ACP `session/new`. Currently `on_new_session`
  only honors `_meta.provider`, `_meta.projectId`, `_meta.client`.
- Preserve `AppEvents.SESSION_CREATED`, `AppEvents.ADD_ACTIVE_SESSION`,
  and the `setView('pair', ...)` navigation.
- Confirm recipe-deeplink and extension-override entry points still work.

### 7. Cancellation Slice

Detailed plan: `07-cancellation.md`

Once live prompt is on ACP, replace REST `sessionCancel` with
`acpCancelSession`. Preserve stop-button semantics: no-op when idle, clean
return to idle on cancel, no stale active-request state.

### 8. Remove Desktop REST Session Usage

Detailed plan: `09-rest-cleanup.md`

After the ACP flows above are proven, remove unused desktop REST session
imports: `resumeAgent`, `getSession` (for migrated chat loading),
`startAgent`, `sessionReply`, `sessionCancel`, `sessionEvents`. Update
tests that mocked them. Keep unrelated REST APIs untouched. Do not manually
edit `ui/desktop/openapi.json`.

## Detail Docs

The detail files split work by subsystem. Their filenames predate the current
slice numbering — the mapping is:

| Slice | Detail file(s) |
|---|---|
| 1. Harden ACP client | `01-harden-acp-client.md` |
| 2. Wrapper + router + text adapter | `02-acp-session-wrapper.md`, `03-notification-adapter.md` |
| 3. Session list | `02-acp-session-wrapper.md` |
| 4. Conversation load | `05-conversation-load.md`, `03-notification-adapter.md`, `useChatStream-acp-data-gap.md` |
| 5. Tool permission bridge | `08-tool-permissions.md` |
| 6. Live prompt + session creation | `06-live-prompt-streaming.md`, `04-session-creation.md`, `03-notification-adapter.md` |
| 7. Cancellation | `07-cancellation.md` |
| 8. REST cleanup | `09-rest-cleanup.md` |

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
