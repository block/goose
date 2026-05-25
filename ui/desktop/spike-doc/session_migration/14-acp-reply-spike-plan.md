# 14 ACP Reply Spike Plan

## Context

Desktop session history now loads through ACP `session/load` and reconstructs
messages from ACP `session/update` notifications. The remaining mixed-lifecycle
problem is follow-up prompts:

```text
ACP session/load -> UI renders history -> REST /sessions/{id}/reply -> Provider not set
```

The failure happens because the loaded session is initialized on the ACP side,
but replies still use the REST-side agent lifecycle.

## Goal

Introduce an incremental ACP reply path using `session/prompt`, starting with a
small spike that proves ACP-loaded sessions can continue through ACP without
REST `sessionReply`.

## Current Status

ACP reply is now implemented incrementally for the normal desktop chat path.
The original spike code is not throwaway; it has become the migration path.

Implemented:

- Normal follow-up prompts use ACP `session/prompt`.
- REST `sessionReply` remains only for the edit/fork `overrideConversation`
  path.
- ACP `session/update` notifications drive message rendering through
  `sessionNotificationAdapter`.
- Tool calls and tool responses render in the existing desktop tool card shape.
- ACP MCP app/resource metadata is adapted to the existing desktop MCP UI shape.
- ACP `session/cancel` is used for active ACP prompts.
- ACP `configOptions` from `session/load` and `config_option_update`
  notifications are stored in desktop state.
- Active session mode writes use ACP `session/set_config_option` with
  `configId: "mode"`.
- Bottom-bar session mode reads from ACP config options when available, with the
  hardcoded Goose mode list retained as fallback.
- ACP permission requests are bridged into the existing inline tool approval UI.
- Approval buttons resolve ACP `requestPermission` when a pending ACP request
  exists, and fall back to REST `confirmToolAction` for legacy REST
  confirmation flows.
- Text chunk merging is idempotent so repeated full/cumulative ACP chunks do not
  duplicate text in the same bubble.

Validated manually:

- Text replies continue after ACP `session/load`.
- Tool prompts run through ACP without the previous `Provider not set` failure.
- Manual mode shows inline approval buttons for tool calls.
- Allow/deny decisions continue the prompt turn through ACP.
- MCP app/resource rendering still works in active chat and standalone app views.

Validated with focused checks:

- ACP adapter unit tests.
- Desktop typecheck.

Remaining migration work:

- ACP new-session recipe and extension-override parity. Plain non-recipe
  sessions already use ACP `session/new`.
- Recipe-session load/new-session behavior.
- REST edit/fork history mutation migration or explicit retention. The old
  `overrideConversation` reply branch has been removed from `useChatStream`.
- Elicitation response migration is done for active ACP sessions through
  `_goose/elicitation/respond`; REST `sessionReply` remains only as the
  non-ACP fallback.
- Reattach/resume behavior for in-flight ACP prompts.
- Additional cancel cleanup validation for pending tool cards.
- Optional stop-reason UX for non-cancel ACP stop reasons.

Next lifecycle slice: see `15-acp-new-session-plan.md` for the guarded ACP
`session/new` migration plan.

## Research Notes

- The ACP SDK exposes `client.prompt(params)`, which sends
  `session/prompt`.
- `PromptRequest` shape:

  ```ts
  {
    sessionId: string;
    prompt: ContentBlock[];
    messageId?: string | null;
  }
  ```

- `PromptResponse` resolves when the turn finishes and includes:
  - `stopReason`
  - optional `usage`
  - optional `userMessageId`
- Streaming content is not returned from `client.prompt`. It arrives through
  ACP `session/update` notifications.
- Goose ACP server `on_prompt` uses the ACP `sessionId` as the Goose thread id
  and calls `get_session_agent`, so it should use the live ACP agent initialized
  by `session/load`.
- ACP cancellation exists through `client.cancel({ sessionId })`.

## Proposed UI Shape

Keep REST reply machinery separate from ACP reply machinery.

```text
REST reply:
  sessionReply -> REST /sessions/{id}/events SSE -> createEventProcessor

ACP reply:
  client.prompt -> ACP session/update notifications -> sessionNotificationAdapter
```

Do not adapt REST `request_id` / `chat_request_id` routing to ACP. ACP prompt
already has a request lifecycle through the JSON-RPC call and session-scoped
notifications.

## Initial Spike Scope

The first patch was intentionally narrow:

- Add an ACP prompt helper in `ui/desktop/src/acp/sessions.ts`.
- Convert desktop user messages to ACP `ContentBlock[]`.
- Add a new `submitToSessionViaAcp` path in `useChatStream.ts`.
- Seed `createAcpSessionNotificationAdapter(currentMessages)`.
- Subscribe to:
  - `subscribeToAcpSession(sessionId, ...)`
  - `subscribeToAcpGooseSession(sessionId, ...)`
- Call `acpPrompt`.
- Apply adapter updates directly to `messages`, `tokenState`, and chat state.
- Clean up subscriptions when prompt resolves or errors.

Originally out of scope for the spike:

- REST active request reattach parity.
- Full cancellation parity.
- Replacing every REST `getSession` title refresh.
- Recipe/new-session migration.
- Permission UX redesign.

## Message Conversion

Support the current desktop user input surface:

- desktop `text` content -> ACP `{ type: "text", text }`
- desktop `image` content -> ACP `{ type: "image", data, mimeType }`

Ignore or reject non-user content types in the initial helper. The ACP prompt
request should be built from the newly submitted user message only, not the
full conversation; Goose already has the session history server-side.

## State Flow

Suggested first implementation flow:

```text
handleSubmit
  -> create desktop user Message
  -> optimistic UI adds user message / sets Streaming
  -> submitToSessionViaAcp(sessionId, userMessage, currentMessages)
      -> create adapter seeded with currentMessages
      -> subscribe to ACP notifications
      -> call acpPrompt(sessionId, userMessage)
      -> notifications update messages/tokenState
      -> prompt response resolves
      -> run existing finish behavior
      -> unsubscribe
```

Important: subscribe before calling `acpPrompt` so early user/agent chunks are
not missed.

## Error Handling

For the spike:

- If `client.prompt` throws, call the same visible error path as REST submit.
- If `stopReason === "cancelled"`, finish without showing a submit error.
- If `stopReason` is `max_tokens`, `max_turn_requests`, or `refusal`, preserve
  rendered content and surface a concise completion status only if the current
  UI has an established place for it.

## Cancellation Follow-Up

After the spike works, migrate `stopStreaming` for ACP active prompts:

```ts
client.cancel({ sessionId });
```

The server should cancel the prompt token and resolve the original prompt with
`stopReason: "cancelled"`.

## Acceptance Criteria For Spike

Manual:

- Load an existing session through ACP `session/load`.
- Send a follow-up prompt through ACP `session/prompt`.
- Confirm the previous `Provider not set` failure does not occur.
- Confirm assistant text streams or appears correctly.
- Confirm token usage updates still render.
- Confirm a simple tool call still renders in the expected tool card shape.

Automated:

- Add or update tests for the desktop-message-to-ACP-content conversion.
- Add a focused hook/helper test if the ACP prompt path can be isolated without
  needing the full Electron runtime.
- Keep existing ACP adapter tests passing.

## Follow-Up Hardening

Current hardening status:

- Session mode updates have been migrated to ACP config options for active
  sessions.
- ACP cancel support has been added to `stopStreaming`.
- ACP permission UI has been bridged into existing inline approval rendering.

Still remaining:

- Decide how ACP active prompt reattach should work, if at all.
- Replace REST `getSession` title refresh with ACP session info updates or a
  Goose custom request.
- Migrate or explicitly retain REST for edit/fork history mutation.
- Keep REST `sessionReply` only for the non-ACP elicitation response fallback
  until REST sessions are retired.

## Mode And Permission Parity

ACP reply uses the ACP-side live agent. Any setting that changes REST session
state but not ACP session state can make the UI look configured while ACP runs
with stale behavior.

Previously known issue:

```text
Settings/manual mode or bottom-bar/manual mode
  -> REST config/updateSession changes
  -> ACP prompt still runs with old ACP agent mode
  -> tools may auto-run
```

The first fix was mode propagation, not permission UI.

Mode propagation implementation:

- Desktop uses an ACP helper around `client.setSessionConfigOption(...)`.
- Active session mode is written with:

   ```ts
   client.setSessionConfigOption({
     sessionId,
     configId: 'mode',
     value: newMode,
   });
   ```

- Desktop captures `configOptions` from ACP `session/load`.
- Desktop handles ACP `config_option_update` session notifications in
  `sessionNotificationAdapter`.
- Active-session mode controls render from the ACP mode config option when it
  is available:
  - find option by `category === "mode"` or `id === "mode"`
  - current value comes from `currentValue`
  - selectable values come from the option list
  - known Goose mode ids keep existing desktop labels
  - unknown/custom mode ids use ACP labels/descriptions
- The hardcoded Goose mode list remains as a fallback for:
  - older ACP servers without config options
  - unloaded sessions
  - global default settings before a session exists
- Global config writes still define defaults for future sessions.

Goose server support already exists for the final path:

- `session/new` and `session/load` return config options for provider, mode,
  and model.
- `session/set_config_option` dispatches `configId: "mode"` through the same
  `on_set_mode` path as legacy `session/set_mode`.
- The server responds with the full updated config option list and also sends a
  `config_option_update` notification.

Permission UI implementation:

- `useChatStream` registers `setAcpPermissionHandler(...)` during an active ACP
  prompt.
- ACP `RequestPermissionRequest` is converted into the existing desktop
  `actionRequired/toolConfirmation` shape.
- Existing inline approval UI renders the buttons beside the tool card.
- Approval buttons resolve the ACP request with the selected option:
  - `allow_once`
  - `allow_always`
  - `reject_once`
  - `reject_always`
- If no ACP pending request exists for the tool id, approval buttons fall back
  to REST `confirmToolAction`.
- Prompt cleanup resolves outstanding ACP permission requests with
  `cancelled`.

Manual approval is no longer an open ACP reply parity gap for the normal prompt
path. It still needs regression coverage for edit/fork once that path migrates
off REST.

## Prompt Turn Compliance Audit

Reference: <https://agentclientprotocol.com/protocol/prompt-turn>

Current desktop ACP reply implementation is mostly aligned with the ACP
prompt-turn lifecycle for the normal chat path.

### What Already Matches

- Desktop sends `session/prompt` with `sessionId` and `prompt:
  ContentBlock[]`.
- Desktop converts the current input surface into ACP prompt content:
  - desktop text -> ACP `TextContent`
  - desktop image -> ACP `ImageContent`
- Desktop subscribes to ACP session notifications before calling
  `session/prompt`, so early `session/update` notifications are not missed.
- Desktop applies standard ACP output notifications through the existing
  session notification adapter:
  - `agent_message_chunk`
  - `agent_thought_chunk`
  - `tool_call`
  - `tool_call_update`
  - `session_info_update`
- Desktop applies Goose usage updates from `_goose/session/update`.
- Desktop sends ACP `session/cancel` for active ACP prompt cancellation.
- Desktop handles ACP `requestPermission` through inline approval UI and returns
  the selected ACP permission option.

### Completed Parity Work

#### 1. ACP Mode Propagation

This was required because desktop already has user-facing mode controls. If the
UI says manual/approve mode but the ACP-side live agent still runs in auto
mode, tools can execute without approval. That is a behavior regression, not
just missing polish.

Relevant UI surfaces:

- bottom-bar session mode selector
- Settings mode section
- new session defaults

Implemented behavior:

- active ACP sessions receive mode changes through ACP config options
- global config writes still define defaults for future sessions
- an ACP-loaded session uses the selected mode during
  `session/prompt`

#### 2. ACP Permission UI

This was required because desktop already supports manual tool approval. ACP
`requestPermission` must be wired to the UI so manual approval works during ACP
prompt turns.

Previously:

- `setAcpPermissionHandler(...)` exists
- no desktop approval handler is registered
- fallback returns `cancelled`

Implemented behavior:

- convert ACP `RequestPermissionRequest` into a visible approval UI state
- resolve the ACP request with the selected permission option:
  - `allow_once`
  - `allow_always`
  - `reject_once`
  - `reject_always`
- cancellation resolves any pending ACP permission request as `cancelled`

### Required Before Closing The Migration Slice

#### 3. Cancel Cleanup Validation

Required enough to avoid stuck UI state. ACP `session/cancel` is sent, but we
still need to verify:

- prompt resolves with `stopReason: "cancelled"`
- chat state returns to idle
- pending permission requests are cancelled
- pending tool cards do not remain indefinitely loading

If manual testing shows pending tool cards stuck after cancel, add explicit UI
cleanup for unfinished tool calls.

### Deferred Items

Deferred means these do not block the ACP reply migration slice. They are either
not clearly supported by current REST desktop UX, not needed for the active
chat/tool flows, or only affect edge-case polish.

#### Stop Reason UX

ACP `PromptResponse.stopReason` can be:

- `end_turn`
- `max_tokens`
- `max_turn_requests`
- `refusal`
- `cancelled`

Current desktop only special-cases `cancelled`; other stop reasons finish like
a normal turn.

Reason to defer:

- normal content still renders
- current desktop does not appear to have a rich established UX for every stop
  reason
- this is user feedback polish, not core prompt execution

#### Prompt Capability Gating

The ACP spec expects clients to adapt to prompt capabilities. Current desktop
sends text and images.

Reason to defer:

- Goose advertises image prompt support
- desktop currently only sends text/images from chat input
- no broader multi-agent ACP compatibility work is in scope for this slice

Revisit before supporting agents with different prompt capabilities or new
input block types.

#### Plan Updates

The ACP prompt-turn spec includes plan/session update concepts. The current
adapter ignores unknown update types.

Reason to defer:

- no clear existing desktop chat UI surface for ACP plan updates
- ignoring unknown updates is safe for text/tool reply parity
- should be revisited only if Goose emits plan updates that users need to see

#### Explicit Cancelled Tool Card State

Desktop sends `session/cancel`, but does not yet proactively mark all
unfinished tool cards as cancelled.

Reason to defer:

- may not be visible if the server sends final tool updates before prompt
  resolution
- can be addressed after manual cancel testing
- not needed to validate the basic ACP reply path
