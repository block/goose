# ACP Session Migration Handoff

This is the source of truth for carrying the desktop ACP session migration into
the next PR. It consolidates the former slice docs so future work does not have
to reconcile scattered status, design notes, and follow-up lists.

## Current State

Desktop chat is ACP-backed for the normal session lifecycle:

- session list
- cold conversation load
- normal prompt streaming
- active-prompt cancellation
- active session mode changes
- plain non-recipe session creation
- visible inline ACP tool permission approval
- ACP elicitation for active ACP sessions

REST remains only in targeted compatibility paths:

- recipe / recipe-deeplink / extension-override session creation
- recipe parameter persistence and recipe prompt application
  (`updateFromSession`)
- edit/fork history mutation
- REST/SSE reattach and buffer-overrun recovery
- app-cache population
- metadata fallbacks such as name polling and mode fallback reads
- non-ACP elicitation response fallback

Current PR status:

- ACP load uses inline setup by default; `GOOSE_ACP_LEGACY_LOAD=1` remains as a
  rollback switch.
- Inline ACP load ignores request `cwd`, returns `_meta.workingDir`, rejects
  non-empty `mcpServers`, replays conversation notifications, builds the agent
  synchronously, emits usage updates, sends available commands, and resolves
  `session/load` only after replay is complete.
- Inline ACP load returns `_meta.recipe`, `_meta.userRecipeValues`,
  `_meta.extensionResults`, and `_meta.workingDir`.
- Desktop subscribes before `acpLoadSession`, applies ACP and
  `_goose/session/update` notifications through the session adapter, dispatches
  `SESSION_LOADED` on request resolution, shows extension results, stores config
  options, and populates `resultsCache`.
- Desktop disposes stale ACP load subscribers during React effect cleanup and
  treats duplicate identical image replay chunks as idempotent, because
  overlapping `session/load` calls can still replay to the current subscriber.
- `redactedThinking`, `frontendToolRequest`, and legacy
  `toolConfirmationRequest` are intentionally omitted from desktop ACP replay.
- Live `systemNotification` status uses `_goose/session/update`
  `status_message`. Persisted legacy `systemNotification` rows are skipped by
  inline ACP load.

## Recommended Next PR

Finish making `systemNotification` structurally live-only.

Why this first:

- It is the only remaining message/content parity follow-up directly in the
  current migration path.
- The user-visible decision is already made: durable acknowledgements should be
  normal assistant text; `systemNotification` is live status.
- It is small enough to land independently before recipe or reattach work.

Tasks:

1. Add code-level documentation on `SystemNotificationContent` and/or its
   constructors:
   - `systemNotification` is live UI/session status.
   - Durable command acknowledgements must use normal assistant `text` with
     `userVisible: true` and `agentVisible: false`.
   - Existing persisted `systemNotification` content is legacy compatibility
     only.
2. Audit producers of `MessageContent::SystemNotification`.
   - Confirm `/clear` and `/compact` durable acknowledgements no longer create
     persisted system notifications.
   - Confirm remaining producers are live status/progress or terminal prompt
     error translation.
3. Add a persistence-boundary guard or focused test so new
   `systemNotification` content is not accidentally stored as normal
   conversation history.
4. Keep read/render compatibility for legacy sessions that already contain
   persisted `systemNotification`.
5. Leave inline ACP load behavior unchanged unless the audit finds a real
   historical compatibility need.
   - Current behavior intentionally skips persisted legacy system notifications.
   - If compatibility is needed later, project old inline notifications to
     plain assistant text in a targeted follow-up, not to live
     `status_message`.

## Completed Slices

### ACP Client Foundation

Done:

- Added ACP notification and permission handler integration points.
- Routed ACP `session/update` notifications to a registered handler.
- Added a Goose custom notification path for `_goose/session/update`.
- Kept reconnect cleanup behavior.

Important files:

- `ui/desktop/src/acp/acpConnection.ts`
- `ui/desktop/src/acp/sessionNotificationRouter.ts`

### ACP Session API, Router, And Adapter

Done:

- Added `ui/desktop/src/acp/sessions.ts` helpers for session list, load,
  prompt, cancel, create, rename, delete, fork, export, and import.
- Added session-scoped ACP and Goose notification routers.
- Added `sessionNotificationAdapter` to reconstruct desktop `Message[]`,
  usage, session info, config options, and interaction state from ACP
  notifications.
- Adapter handles text, image, thinking, tool call, tool update, standard ACP
  usage, Goose accumulated usage/cost, session info, config options,
  permissions, status rows, and elicitation updates.

Design rules:

- Subscribe before calling `session/load` or `session/prompt` so early
  notifications are not missed.
- `session/load` request resolution is the authoritative replay-complete
  boundary. No extra Goose replay-complete notification is needed.
- Progressive replay painting during a long load is optional polish. The final
  loaded state should still be committed when `acpLoadSession` resolves.

### Session List

Done:

- Session list, rename, delete, fork, export, and import use ACP.
- ACP `SessionInfo` maps into desktop `SessionListItem`.
- Session list is the metadata source for load where available
  (`workingDir`, title, provider, model).

### Conversation Load

Done:

- REST `resumeAgent` cold-load has been replaced with ACP `session/load` and
  notification replay for normal sessions.
- Inline load is default and legacy load remains behind
  `GOOSE_ACP_LEGACY_LOAD=1`.
- Inline load replays persisted text, images, thinking, tool requests, tool
  responses, pending elicitations, and usage/session/config updates.
- Inline load ignores request `cwd`; session `working_dir` is source of truth.
- Inline load rejects non-empty `mcpServers` because Goose owns MCP/extension
  config server-side.
- Inline load returns `recipe`, `userRecipeValues`, `extensionResults`, and
  `workingDir` in `_meta`.
- Desktop shows extension load results after load resolves.
- Desktop caches loaded results for fast remounts.

Open load polish:

- Merge provider/model fields from `LoadSessionResponse.models` into the loaded
  desktop `Session` snapshot.
- Optionally flush adapter snapshots while `session/load` is pending so large
  histories paint progressively.
- Replace remaining REST session-name refresh/polling with
  `SessionInfoUpdate`.
- Audit remaining `getSession` callers and migrate each to session cache,
  `SessionInfoUpdate`, or an ACP custom request.
- Validate ACP `session/load` re-entrancy before using it as a recovery path on
  the same session/connection. If ACP clients cannot safely call
  `session/load` twice, recovery should use tear-down + reconnect or a
  Goose-specific replay request rather than assuming repeat load works.

### Live Prompt

Done:

- Normal text/image prompt submission uses ACP `session/prompt`.
- Desktop converts current chat input into ACP `ContentBlock[]`:
  - desktop text -> ACP text
  - desktop image -> ACP image
- Desktop applies standard ACP prompt output notifications through the adapter:
  - `agent_message_chunk`
  - `agent_thought_chunk`
  - `tool_call`
  - `tool_call_update`
  - `session_info_update`
- Desktop applies Goose usage updates from `_goose/session/update`.
- REST request-id routing is dropped for normal ACP prompts; ACP scopes by
  `sessionId`.
- Generic prompt failures currently surface through the ACP prompt catch path.
- Credits-exhausted prompt errors use structured JSON-RPC error data and render
  the existing credits card.

Open prompt polish:

- Rename generic ACP prompt catch wording from `Submit error:` to
  `Stream error:` or `Agent error:`.
- Verify non-cancel ACP stop reasons (`max_tokens`, `max_turn_requests`,
  `refusal`) do not need richer UX.
- Revisit prompt capability gating before supporting agents with different
  prompt capabilities or new input block types.
- Revisit ACP plan/session update concepts if Goose starts emitting plan
  updates that users need to see.

### Cancellation

Done:

- Active ACP prompts use ACP `session/cancel`.
- Desktop tracks active ACP prompt state.
- Stop is a no-op when there is no active prompt.
- Active prompt refs are cleared on cancel.
- Outstanding ACP permission requests are resolved as `cancelled` during
  prompt cleanup.
- REST `sessionCancel` remains for REST/SSE active-request reattach paths.

Still verify:

- Prompt resolves with `stopReason: "cancelled"`.
- Chat state returns to idle.
- Pending permission requests are cancelled.
- Pending tool cards do not remain indefinitely loading.
- If manual testing shows pending tool cards stuck after cancel, add explicit
  UI cleanup for unfinished tool calls.

### Mode And Config Options

Done:

- Desktop captures `configOptions` from ACP `session/load`.
- Desktop handles ACP `config_option_update` notifications.
- Active-session mode writes use `session/set_config_option` with
  `configId: "mode"`.
- Active-session mode controls render from ACP mode config option when
  available:
  - find option by `category === "mode"` or `id === "mode"`
  - current value comes from `currentValue`
  - selectable values come from the option list
  - known Goose mode ids keep desktop labels
  - unknown/custom mode ids use ACP labels/descriptions
- Hardcoded Goose mode list remains as fallback for older ACP servers,
  unloaded sessions, and global default settings before a session exists.
- Global config writes still define defaults for future sessions.

### Tool Permission

Done for visible active ACP prompts:

- `useChatStream` registers `setAcpPermissionHandler(...)` during an active ACP
  prompt.
- ACP `requestPermission` converts into the existing desktop
  `actionRequired.toolConfirmation` UI shape.
- Approval buttons resolve ACP with the selected option:
  - `allow_once`
  - `allow_always`
  - `reject_once`
  - `reject_always`
- Approval buttons fall back to REST `confirmToolAction` only when no ACP
  pending request exists for the tool id.
- Prompt cleanup resolves outstanding ACP permission requests as `cancelled`.

Remaining hardening:

- App-level durability for hidden/background sessions or permission requests
  that outlive the current hook instance.
- Regression coverage once edit/fork migrates off REST.

### Elicitation

Done for active ACP sessions:

- Live prompt emits `_goose/session/update` with
  `sessionUpdate: "interaction_update"` and `interaction.type: "elicitation"`
  when an elicitation is pending.
- Desktop renders the form from `interaction.requestedSchema`.
- Desktop submits through `_goose/unstable/elicitation/respond` with `sessionId`,
  `elicitationId`, and `userData`.
- Server submits to `ActionRequiredManager`, persists the hidden response
  message, and emits `interaction.state: "submitted"`.
- Original prompt stream continues after the response is accepted.
- Load session may emit the same `interaction_update` shape for any persisted
  elicitation request that is still pending.
- REST `sessionReply` remains only as the fallback for non-ACP sessions.

### Plain Session Creation

Done:

- Plain non-recipe `createSession(...)` uses ACP `session/new`.
- ACP create path is guarded. REST `startAgent` remains when any are true:
  - recipe session
  - recipe deeplink session
  - explicit extension configs
  - extension override state exists and must be consumed/cleared
- ACP-created plain sessions route through the existing chat view and use
  ACP `session/load` for state setup.
- First prompt after ACP new-session creation uses ACP `session/prompt`.
- Existing desktop events and navigation are preserved:
  - `SESSION_CREATED`
  - `ADD_ACTIVE_SESSION`
  - `setView('pair', { resumeSessionId })`

Open creation work:

- Decide whether ACP `session/new` should support recipe session creation
  directly, or whether recipe creation should remain REST until recipes have an
  ACP-specific design.
- Decide how desktop extension overrides map to ACP:
  - convert override `ExtensionConfig[]` into ACP `mcpServers`, where possible
  - add a Goose-specific ACP `_meta` field for extension override semantics
  - keep override sessions on REST
- Decide whether ACP new-session response should seed `resultsCache` to avoid
  immediate `session/load`.
- Verify default enabled extensions in ACP-created sessions match REST-created
  plain sessions, including platform/developer extension behavior.
- `on_new_session` still uses the older deferred setup path; apply
  `mcpServers` rejection and inline setup policy there only when that path is
  rewritten.

## Message Content Decisions

### Covered Mappings

| Goose content type | ACP load / live behavior | Desktop behavior |
|---|---|---|
| `text` | `user_message_chunk` / `agent_message_chunk` | Converts to desktop text |
| `image` | ACP image chunks with Goose replay metadata; prompt input supports user images | Converts to desktop image and de-duplicates identical overlapping replay chunks |
| `toolRequest` | `tool_call` | Converts to `toolRequest` |
| `toolResponse` | `tool_call_update` | Converts to `toolResponse` |
| `thinking` | `agent_thought_chunk` | Converts to `thinking` |
| `actionRequired.elicitation` | `_goose/session/update` `interaction_update` for pending requests | Converts to `actionRequired.elicitation` |
| `actionRequired.elicitationResponse` | Submitted response is hidden and unblocks the original prompt | Not rendered as visible transcript |
| live `systemNotification.inlineMessage` | `_goose/session/update` `status_message.notice` | Local inline status row |
| live `systemNotification.thinkingMessage` | `_goose/session/update` `status_message.progress` | Local progress/thinking status row |
| credits exhausted | structured `session/prompt` JSON-RPC error with `data.reason = "credits_exhausted"` | Existing credits card |

### Intentional Omissions

- `redactedThinking` is intentionally omitted from desktop ACP replay.
  - It is opaque provider context, not displayable reasoning text.
  - REST desktop does not render it as visible thinking content.
  - Provider continuity comes from the stored backend conversation, not from
    desktop transcript replay.
- `frontendToolRequest` is intentionally omitted from desktop ACP replay.
  - It is provider/frontend-tool plumbing.
  - REST desktop does not expose it as a visible transcript row.
  - Revisit only if legacy REST sessions are found where it was relied on as
    user-visible transcript content.
- Legacy `toolConfirmationRequest` is intentionally omitted from desktop ACP
  replay.
  - Current persisted approval content uses
    `actionRequired.toolConfirmation`.
  - Current live ACP approval uses `requestPermission`.
  - Old sessions can still load and continue; only a legacy pending approval
    row would be absent from replay, and a cold-loaded pending approval from an
    old session is not reliably actionable.

### Image Replay

Manual testing confirmed user image history was missing after ACP load even
though image content was persisted in `sessions.db`.

Current behavior:

- Inline ACP load replays stored `MessageContent::Image` as ACP image chunks
  with replay metadata.
- Desktop reconstructs them as Goose image content.
- Desktop disposes stale load subscriptions and treats duplicate identical
  image chunks for the same message as idempotent.

Remaining:

- Manually verify whether assistant image output can appear in Goose sessions.
  The server replay path routes images by message role, and desktop can render
  ACP image chunks, but assistant-image rendering has not been manually proven.

### System Notifications

Decision:

- Durable command acknowledgements that should remain visible after resume use
  normal assistant `text` with `userVisible: true` and `agentVisible: false`.
- `systemNotification` is live UI/session status, not durable transcript
  content.
- Live status travels through `_goose/session/update` `status_message`.
- Credits exhausted is not `status_message`; it is a structured ACP prompt
  error.
- Inline ACP load intentionally skips old persisted `systemNotification` rows.
- If a historical compatibility need appears, project old inline notifications
  to plain assistant text in a targeted follow-up, not to live status replay.

Next PR should make this structurally enforced. See "Recommended Next PR".

## Message Identity Contract

ACP clients should not have to guess which Goose message an update belongs to.
Every ACP update that contributes to, annotates, or represents a Goose
`Message` should carry:

```json
{
  "_meta": {
    "goose": {
      "messageId": "msg_...",
      "created": 1700000000
    }
  }
}
```

Rules:

- Same `messageId` means the update belongs to the same Goose message.
- Different `messageId` means the update belongs to a different Goose message.
- Missing `messageId` means the update is uncorrelated or synthetic.
- Clients should not merge missing-id chunks into an existing row unless the
  update type has its own grouping semantics.
- `created` is the Goose message creation timestamp.

Current identity state:

- Load replay uses persisted `Message.id` and `Message.created`.
- `interaction_update` pending elicitation includes the persisted/live
  action-required message id when present.
- `status_message`, usage, session info, and config updates are session-level
  and do not carry message identity.

Open identity work:

- Normalize live prompt messages so Goose-originated messages have ids before
  ACP emits updates. Some live messages are created with `Message::assistant()`
  and default to `id: None`.
- Live `tool_call` should merge `_meta.goose.messageId/created` for the owning
  assistant tool-request message.
- Live `tool_call_update` should merge `_meta.goose.messageId/created` for the
  owning user/tool-response message.
- Synthetic title/summary updates should preserve tool identity metadata and
  include message identity if they update a known Goose message.
- Desktop should keep defensive fallback:
  - prefer ACP `messageId` or `_meta.goose.messageId`
  - create standalone rows or merge only into known transcript rows when no id
    exists
  - never merge id-less transcript chunks into live-only status rows

## Recipe And Session Creation Parity

Recipe parity is intentionally deferred. Do not partially migrate recipe
creation until the server-side render-and-apply mechanism is agreed.

Current REST dependency:

- ACP load returns raw recipe data.
- Desktop still calls REST `updateFromSession` after load so recipe behavior is
  preserved for now.
- Recipe / recipe-deeplink / extension-override session creation still uses
  REST `startAgent`.

Needed ACP shape:

- Apply rendered recipe prompts ACP-native during load/create setup when values
  are present.
- Add a Goose custom recipe request, likely `_goose/recipe/apply`, that:
  - optionally accepts submitted values
  - persists submitted recipe values
  - renders with `build_recipe_with_parameter_values`
  - applies with `apply_recipe_to_agent` and
    `agent.extend_system_prompt("recipe", ...)`
  - returns enough data for desktop to update local session state
- Replace REST `updateSessionUserRecipeValues` and `updateFromSession` for ACP
  sessions.
- Extend ACP `session/new` for recipe and recipe-id entry points.
- Return resolved recipe data on `NewSessionResponse._meta.recipe` so deeplink
  initial-message handling can read `recipe.prompt`.
- Decide how desktop extension overrides map to ACP `session/new`.

Recipe design decisions already made:

- `LoadSessionResponse._meta.recipe` carries full recipe JSON.
- `LoadSessionResponse._meta.userRecipeValues` carries persisted values.
- `_meta` keys use top-level camelCase, matching existing `session_meta`
  convention.
- No recipe sanitization difference from REST: REST returns `recipe` unchanged,
  so ACP matches REST behavior.
- Mid-session recipe param editing does not exist in the UI today. The custom
  recipe RPC is needed at most once per session between setup and first message
  on fresh sessions.

Verify later:

- recipe deeplink launch
- recipe-id launch
- recipe parameter form submit
- rendered recipe is re-applied to the agent system prompt
- extension override flow

## Edit/Fork History Mutation

The old desktop `overrideConversation` reply branch has been removed from the
normal ACP prompt path, but edit/fork still needs an ACP-native history mutation
story.

Preferred direction:

- use ACP `unstable_forkSession` for session copy
- add a Goose custom history method, likely `_goose/session/truncate`
- prefer truncating by `messageId` if the server can support it cleanly
- keep timestamp support only as a compatibility bridge if needed

Desktop fork-edit flow should become:

1. ACP `unstable_forkSession`
2. ACP `_goose/session/truncate` on the forked session
3. navigate to the forked session
4. existing ACP `session/prompt` submits the edited message

Desktop edit-in-place flow should become:

1. ACP `_goose/session/truncate` on the current session
2. ACP `session/load` to rebuild UI and ACP-side session state from truncated
   DB history
3. existing ACP `session/prompt` submits the edited message

Do not add a desktop-specific ACP method that directly mirrors the old REST
fork request shape (`copy` + `truncate` + `timestamp`) unless there is no
cleaner protocol shape available.

## MCP And Extension Ownership

Goose owns MCP/extension config server-side. ACP clients should pass
`mcpServers: []`.

Decision:

- Keep Goose's current server-owned extension architecture.
- Do not migrate to client-owned MCP config unless a real third-party ACP
  client integration creates demand.
- Reject non-empty `mcpServers` instead of silently persisting them.

Current state:

- Inline ACP load rejects non-empty `mcpServers`.
- Known clients already pass `[]`.
- `_goose/extensions/add` is the explicit extension addition path.

Remaining:

- Advertise ownership at handshake through `InitializeResponse._meta`, for
  example:

  ```json
  {
    "goose": {
      "mcpServersOwnership": "agent",
      "mcpServersOnNewBehavior": "reject-if-non-empty",
      "mcpServersOnLoadBehavior": "reject-if-non-empty"
    }
  }
  ```

- Reject non-empty `mcpServers` on `session/new` as well as load.
- Document the Goose ACP divergence for third-party clients.
- Add a comment at the rejection site so future readers find the rationale.
- Future Zed-style clients that want transient MCP servers should use an
  explicit Goose extension path such as `_goose/extensions/add` after load, with
  explicit lifecycle semantics.

Rejected alternatives:

- silently ignore `mcpServers`
- warn-and-ignore
- honor request MCPs as additive overlay
- migrate Goose to client-owned MCP config now

## ACP Error Handling

Protocol rule:

- ACP method failures should fail the original JSON-RPC request.
- Goose should not invent generic `session/update` error notifications for
  prompt failures.

Credits exhausted:

- Provider credit exhaustion is actionable domain state and a terminal prompt
  failure.
- ACP should return a structured JSON-RPC error:

  ```json
  {
    "error": {
      "code": -32603,
      "message": "Please add credits to your account, then resend your message to continue.",
      "data": {
        "reason": "credits_exhausted",
        "url": "https://router.tetrate.ai/billing"
      }
    }
  }
  ```

- `error.message` is user-facing.
- `error.data.reason` is machine-readable.
- `error.data.url` is optional.
- Desktop renders this as the existing credits-exhausted card.
- `status_message` remains reserved for non-terminal live status/progress.

Remaining:

- Rename generic ACP prompt catch wording from `Submit error:` to
  `Stream error:` or `Agent error:`.
- Verify cancellation edge cases:
  - expected cancellation resolves with `stopReason: "cancelled"`
  - expected cancellation does not surface as a JSON-RPC error
- Consider a structured internal `AgentEvent::PromptError` so ACP can map
  network and provider failures without parsing assistant text.

## Inline Load Design Decisions

Why inline load:

- Deferred setup did not speed up conversation paint; replay notifications
  streamed during the call either way.
- Deferred setup hid latency. A prompt sent immediately after load still
  blocked server-side waiting for setup.
- Deferred setup had no client-visible extension setup completion signal.
- Deferred setup dropped recipe application unless desktop called REST.
- Inline load makes `session/load` resolution mean replay and agent setup are
  complete.

Policy decisions:

- `GOOSE_ACP_LEGACY_LOAD=1` opts back into legacy load. Default is inline.
- Legacy load remains a rollback path and intentionally keeps old behavior,
  including request `cwd` overwrite.
- Inline load ignores request `cwd`; explicit working-dir changes use
  `_goose/working_dir/update`.
- Inline load rejects non-empty `mcpServers`; `session/new` rejection is a
  follow-up when new-session setup is rewritten.
- Inline load returns full `extensionResults` to match REST.
- Per-extension failures do not fail the whole load; they are included as
  `success: false` entries.
- Legacy ACP `UsageUpdate` continues alongside `_goose/session/update` until
  legacy load is retired.

Client impact:

- Desktop reads `_meta.extensionResults`, `_meta.recipe`,
  `_meta.userRecipeValues`, and `_meta.workingDir`.
- Goose-internal can keep its existing prepared-session cache semantics.
- Goose-internal may later replace follow-up provider-setting calls by reading
  `LoadSessionResponse.models.current_model_id`.
- The old goose-internal `"~"` cwd fallback is no longer dangerous in inline
  load because request `cwd` cannot overwrite saved `working_dir`.
- Vanilla ACP clients that ignore `_meta` still load and replay conversation
  normally.

Performance trade-off:

- `session/load` resolution includes provider init and extension load, so the
  loading spinner may stay longer.
- Conversation paint is not slower because replay notifications stream during
  the call.
- Time to first successful prompt is not materially worse; inline moves the
  wait from the first prompt to the load boundary.

Potential optimizations, only if measurement shows a real problem:

- Prewarm globally-enabled extensions at ACP `initialize` time.
- Share `extension_manager` across sessions in a connection.

Cleanup later:

- Remove `GOOSE_ACP_LEGACY_LOAD`, legacy load dispatcher, and
  `on_load_session_legacy` after confidence in inline load.
- Remove or rewrite async setup machinery only after `on_new_session` and the
  fork/duplicate site no longer need it:
  - `AgentHandle::Loading`
  - `AgentSetupSignal`
  - `AgentSetupProgress`
  - `spawn_agent_setup`
  - `get_session_agent_provider_ready`
  - `get_agent_or_receiver`
  - `add_mcp_extensions`

PR #9317 follow-ups marked `(TODO in next PR)`:

- Revisit `loadSession` `mcpServers` semantics
  ([discussion](https://github.com/aaif-goose/goose/pull/9317#discussion_r3297214483)).
  Current desktop/Goose flow
  relies on server-owned config extensions and rejects non-empty
  `mcpServers`, but ACP common clients may expect load-time MCP servers to be
  restored. Decide whether Goose keeps this divergence, advertises it, or adds
  an explicit transient extension path. Re-enable
  `crates/goose/tests/acp_server_test.rs` `test_load_session_mcp`, currently
  ignored with `TODO(lifei)`, when this contract is settled.
- Decide the `loadSession.cwd` contract
  ([discussion](https://github.com/aaif-goose/goose/pull/9317#discussion_r3297215270)).
  Inline load currently treats the
  stored session working directory as authoritative and ignores request `cwd`;
  legacy load updated the persisted working directory from request `cwd`.
  Align server behavior, generated API expectations, and tests around one
  contract. Re-enable
  `crates/goose/tests/acp_custom_requests_test.rs`
  `test_load_session_passes_load_cwd_to_provider_factory`, currently ignored
  with `TODO(lifei)`, when this contract is settled.
- Refactor `loadSession` setup boundaries
  ([discussion](https://github.com/aaif-goose/goose/pull/9317#discussion_r3297236594)).
  Decide whether replay/read-only
  session restore should succeed even when provider resolution/auth/setup
  fails, or whether `loadSession` should continue to mean "history replayed and
  agent ready for prompt".
- Revisit `forkSession` cwd ownership
  ([discussion](https://github.com/aaif-goose/goose/pull/9317#discussion_r3297410789)).
  ACP requires request `cwd`, so desktop
  currently sends `session.workingDir`; this can be stale if the original
  directory moved. Prefer a server-side contract where fork can use the source
  session working directory or a safe default without forcing clients to echo
  historical cwd values.

## Larger Backend Cleanup Path

Optional Path B after the migration settles:

- Adopt goose-core helpers plus a per-instance ACP `AgentManager`.
- Replace `build_agent_for_session` manual provider/extension setup with
  `agent.restore_provider_from_session` and
  `agent.load_extensions_from_session`.
- Use an ACP-instantiated `AgentManager` on `GooseAcpAgent` for LRU caching and
  auto-restore on miss, not the REST singleton.
- Parameterize `AgentManager` for `GoosePlatform` and
  `session_name_update_tx`.
- Resolve scheduler ownership, currently constructed inside
  `AgentManager::new` but probably process-wide.
- Handle AcpTools `developer` wrap as a post-load overwrite, or enhance
  `load_extensions_from_session` to accept exclusions.
- Decide `Config::global()` vs per-instance `config_dir`.
- Decide whether provider fallback restore should be read-only. The existing
  core helper may persist fallback provider to the DB on registry miss, which
  conflicts with `loadSession` as read-only.

## Remaining Work Queue

Recommended order:

1. Make `systemNotification` structurally live-only.
2. Replace REST session-name refresh/polling with `SessionInfoUpdate`.
3. Verify and harden ACP cancellation/remount behavior.
4. Normalize live prompt message identity and tool metadata.
5. Design and implement ACP-native recipe apply/session creation.
6. Design ACP-native edit/fork history mutation.
7. Finish MCP ownership advertisement and `session/new` rejection.
8. Retire legacy load and async setup machinery after confidence.

Detailed backlog:

- `systemNotification` structural guard and producer audit.
- Assistant image output manual check.
- Provider/model merge from `LoadSessionResponse.models`.
- Progressive replay paint during long loads.
- Remaining REST metadata refresh migration.
- Active prompt reattach semantics.
- Hidden/background permission request durability.
- Recipe apply custom request and REST recipe removal.
- ACP recipe and recipe-id `session/new`.
- Extension override mapping.
- ACP edit/fork history mutation.
- MCP ownership `_meta` advertisement.
- Generic ACP prompt error wording.
- Cancellation edge-case verification.
- Structured internal `AgentEvent::PromptError`.
- Broader integration tests after adapter and hook migration settle.
- REST endpoint removal after desktop no longer depends on them.
- Trim unused `TokenState` fields once REST is removed:
  - `inputTokens`
  - `outputTokens`
  - `accumulatedTotalTokens`
- Unify session-list token count vs schedule detail view on
  `accumulated_total_tokens`.
- Document final `goosed` bridge removal requirements once desktop
  session/chat is ACP-backed.

## Verification Carryover

Already manually verified:

- load existing session with text
- send a prompt and receive streamed assistant text
- load thinking content
- load tool calls
- load persisted user image content
- tool approval prompt
- approve tool request
- reject tool request
- create plain non-recipe ACP session
- ACP elicitation live flow
- ACP elicitation pending replay

Still verify later:

- cancel a running ACP prompt
- pending tool-card cleanup after cancel
- navigate away/back during an active ACP prompt
- navigate away/back after a completed ACP session
- session name updates without REST polling
- assistant image output, if Goose can produce it
- recipe deeplink and recipe-id flows
- extension override flow
- recipe parameter submit re-applies rendered prompt
- default enabled extensions in ACP-created sessions match REST-created plain
  sessions, including platform/developer extension behavior

Focused automated coverage already exists for:

- session router dispatch/subscription behavior
- adapter text chunk accumulation
- adapter thinking conversion
- adapter tool call conversion
- adapter tool update conversion
- adapter usage conversion
- adapter session info conversion
- permission approve/reject/cancel mapping
- elicitation pending/submitted updates
- duplicate identical image replay chunks

Suggested future tests:

- `systemNotification` persistence-boundary guard.
- Inline load `mcpServers` rejection.
- Inline load ignores request `cwd` and preserves stored `working_dir`.
- Inline load returns `extensionResults`, `recipe`, `userRecipeValues`, and
  `workingDir`.
- Existing ACP load suite with `GOOSE_ACP_LEGACY_LOAD=1`, while the legacy path
  exists.
- Recipe application custom request.
- ACP `session/new` recipe and extension-override routing.
- Cancel/remount behavior.
