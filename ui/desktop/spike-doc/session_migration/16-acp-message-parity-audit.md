# 16 ACP Message Parity Audit

## Context

Desktop REST session loading receives Goose `Message[]` directly from the
server. ACP `session/load` and `session/prompt` instead stream ACP
`SessionUpdate` notifications, and desktop reconstructs Goose-shaped messages
with `sessionNotificationAdapter`.

That means ACP migration is not complete unless every user-visible Goose
message content type has a deliberate mapping, fallback, or documented reason
to be omitted.

## Source Types

Goose message content variants are defined in:

- `crates/goose/src/conversation/message.rs`

Current variants:

- `text`
- `image`
- `toolRequest`
- `toolResponse`
- `toolConfirmationRequest`
- `actionRequired`
  - `toolConfirmation`
  - `elicitation`
  - `elicitationResponse`
- `frontendToolRequest`
- `thinking`
- `redactedThinking`
- `systemNotification`
  - `thinkingMessage`
  - `inlineMessage`
  - `creditsExhausted`

## Current ACP Mapping

ACP load replay is implemented in:

- `crates/goose/src/acp/server/load_session.rs`

ACP live prompt streaming is implemented in:

- `crates/goose/src/acp/server.rs`

Desktop ACP reconstruction is implemented in:

- `ui/desktop/src/acp/sessionNotificationAdapter.ts`

| Goose content type | REST load | ACP load replay | ACP live prompt | Desktop ACP adapter | Status |
| --- | --- | --- | --- | --- | --- |
| `text` | Native `MessageContent` | `user_message_chunk` / `agent_message_chunk` | `agent_message_chunk` | Converts ACP text to `text` | Covered |
| `image` | Native `MessageContent` | Replayed as ACP image chunks with Goose replay metadata | Prompt input conversion supports user images; live image output should render if emitted | Converts ACP image chunks and de-duplicates identical overlapping replay chunks | Covered for persisted user images; assistant image output still needs manual rendering check |
| `toolRequest` | Native `MessageContent` | `tool_call` | `tool_call` | Converts to `toolRequest` | Covered |
| `toolResponse` | Native `MessageContent` | `tool_call_update` | `tool_call_update` | Converts to `toolResponse` | Covered |
| `thinking` | Native `MessageContent` | `agent_thought_chunk` | `agent_thought_chunk` | Converts to `thinking` | Covered |
| `actionRequired.toolConfirmation` | Native `MessageContent` | Not replayed as `actionRequired` | ACP permission request path | Creates `actionRequired.toolConfirmation` from permission request | Partial |
| `actionRequired.elicitation` | Native `MessageContent` | `_goose/session/update` `interaction_update` for pending persisted elicitations | `_goose/session/update` `interaction_update` | Creates `actionRequired.elicitation` from pending interaction updates | Covered for pending requests |
| `actionRequired.elicitationResponse` | Native hidden message | Hidden/submitted responses are not replayed as visible content | ACP sessions submit via `_goose/elicitation/respond`; REST remains for non-ACP sessions | Response is not rendered as a normal visible message | Covered for ACP sessions, hidden by design |
| `systemNotification.inlineMessage` | Native `MessageContent` | Skipped as live-only legacy status | `_goose/session/update` `status_message.notice` | Converts to local inline notification row | Covered for live status |
| `systemNotification.thinkingMessage` | Native `MessageContent` | Skipped as live-only legacy status | `_goose/session/update` `status_message.progress` | Converts to local thinking notification row | Covered for live status |
| `systemNotification.creditsExhausted` | Native `MessageContent` | Skipped as legacy status/error content | ACP-only prompt error translation: structured `session/prompt` JSON-RPC error with `data.reason = "credits_exhausted"` | Converts structured prompt error to local credits warning row | Covered |
| `redactedThinking` | Native `MessageContent` | Intentionally omitted | Intentionally omitted | No ACP mapping | Omitted by design; opaque provider context, not visible transcript |
| `frontendToolRequest` | Native `MessageContent` | Intentionally omitted | Intentionally omitted | No ACP mapping | Omitted by design; provider/frontend-tool plumbing, not REST-rendered transcript |
| `toolConfirmationRequest` | Native legacy content | Intentionally omitted | Intentionally omitted | Existing REST UI helpers can read it | Omitted by design; current approval uses `actionRequired.toolConfirmation` and ACP `requestPermission` |

## Important Gaps

### System Notifications

`systemNotification` is currently both a persisted message-content type in a
few command paths and a live UI/status signal in other paths. That mixed use is
the main design issue.

System notifications are used for user-visible session state, including:

- `/clear` and compact command responses
- auto-compaction progress
- context-limit compaction progress
- credits-exhausted messages

Desktop already has renderers for these notifications:

- `ProgressiveMessageList`
- `SystemNotificationInline`
- `CreditsExhaustedNotification`

ACP now maps live system notifications to the Goose custom `status_message`
update. Inline ACP load skips persisted legacy `systemNotification` rows
because status is live-only and should not be replayed as transcript history.

Current `systemNotification` variants:

- `inlineMessage`
  - UI behavior: small inline status row.
  - Legacy persisted uses:
    - `/clear`: `Conversation cleared`
    - `/compact`: `Compaction complete`
  - Current durable command acknowledgements:
    - `/clear`: normal assistant text with `userVisible: true`,
      `agentVisible: false`
    - `/compact`: normal assistant text with `userVisible: true`,
      `agentVisible: false`
  - Current live-only uses:
    - auto-compact and context-limit status messages
    - goal/grind notices
- `thinkingMessage`
  - UI behavior: loading/spinner status text, not a chat-history row.
  - Current live-only use:
    - compaction progress, e.g. `goose is compacting the conversation...`
- `creditsExhausted`
  - UI behavior: actionable warning card.
  - Current live-only use:
    - provider credit exhaustion with optional `data.top_up_url`.

Long-term design direction:

- Durable command acknowledgements that should remain visible after resume
  should be normal assistant `text` messages with `userVisible: true` and
  `agentVisible: false`.
  - `/clear` should persist `Conversation cleared` as text.
  - `/compact` should persist `Compaction complete` as text.
- `systemNotification` should be treated as live session status, not durable
  transcript content.
- Existing persisted `systemNotification` rows are legacy compatibility cases.
  Inline ACP load intentionally skips them for now. If historical styling or
  visibility becomes important, add an explicit compatibility projection rather
  than treating them as live `status_message` replay.

This keeps ACP transcript replay simple and keeps UI/status concepts out of
standard ACP assistant message chunks.

### Goose Status Messages

Live Goose UI/session status should use the existing custom notification
channel:

- method: `_goose/session/update`
- payload type: `GooseSessionNotification`

Add a typed `status_message` variant to the existing `GooseSessionUpdate`
union:

```ts
type GooseSessionUpdate =
  | UsageUpdate
  | StatusMessageUpdate
  | InteractionUpdate;

type StatusMessageUpdate = {
  sessionUpdate: 'status_message';
  status: StatusMessage;
};

type StatusMessage =
  | {
      type: 'notice';
      message: string;
    }
  | {
      type: 'progress';
      message: string;
    };
```

`status_message` is live-only UI/session status. It is not conversation
transcript content, and should not be persisted or replayed as history.

Mapping from current `systemNotification`:

- live `inlineMessage` -> `status.type = 'notice'`
- `thinkingMessage` -> `status.type = 'progress'`

Credits exhausted is not a `status_message`. ACP translates live
`SystemNotificationType::CreditsExhausted` into a structured JSON-RPC
`session/prompt` error:

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

See `18-acp-error-handling.md`.

Example compaction progress update:

```json
{
  "method": "_goose/session/update",
  "params": {
    "sessionId": "s1",
    "update": {
      "sessionUpdate": "status_message",
      "status": {
        "type": "progress",
        "message": "goose is compacting the conversation..."
      }
    }
  }
}
```

This schema describes domain state, not presentation. Desktop can map:

- `notice` to an inline notice or other local presentation
- `progress` to loading/progress UI

### Goose Interaction Updates

Blocking user interactions that are not normal transcript content should use a
separate custom update shape. This keeps standard ACP message chunks focused on
conversation history and keeps Goose-specific interaction state out of
assistant text metadata.

Tool approval is the exception: ACP already has a standard
`requestPermission` request/response flow. Goose should continue using that for
live tool confirmation. Desktop may adapt ACP `requestPermission` into the
existing `actionRequired.toolConfirmation` view model locally, but Goose should
not duplicate live tool approval through a custom interaction update.

Elicitation is different from tool approval:

- tool approval answers whether a tool may run
- elicitation collects structured user data needed by a tool or workflow

Minimal custom update shape:

```ts
type InteractionUpdate = {
  sessionUpdate: 'interaction_update';
  interaction: {
    type: 'elicitation';
    id: string;
    state: 'pending' | 'submitted';
    message?: string;
    requestedSchema?: unknown;
  };
};
```

Rules:

- `pending` includes `message` and `requestedSchema`.
- `submitted` only requires `id` and `state`.
- Expiration remains local desktop UI state, not a Goose ACP interaction state.

Example pending elicitation:

```json
{
  "method": "_goose/session/update",
  "params": {
    "sessionId": "s1",
    "update": {
      "sessionUpdate": "interaction_update",
      "interaction": {
        "type": "elicitation",
        "id": "elicitation-1",
        "state": "pending",
        "message": "Please provide deployment details",
        "requestedSchema": {
          "type": "object",
          "properties": {
            "environment": {
              "type": "string",
              "enum": ["local", "staging", "production"]
            }
          },
          "required": ["environment"]
        }
      }
    }
  }
}
```

Example submitted elicitation:

```json
{
  "method": "_goose/session/update",
  "params": {
    "sessionId": "s1",
    "update": {
      "sessionUpdate": "interaction_update",
      "interaction": {
        "type": "elicitation",
        "id": "elicitation-1",
        "state": "submitted"
      }
    }
  }
}
```

### Elicitation

Elicitation requests are represented in REST history as
`actionRequired.elicitation` and rendered by `ElicitationRequest`.

ACP now uses the planned custom interaction path for ACP sessions:

- live prompt emits `_goose/session/update` with
  `sessionUpdate: "interaction_update"` and `interaction.type:
  "elicitation"` when an elicitation is pending
- desktop renders the form from `interaction.requestedSchema`
- desktop submits with the Goose custom method `_goose/elicitation/respond`
- response request includes `sessionId`, `elicitationId`, and `userData`
- server submits to `ActionRequiredManager`, persists the hidden response
  message, and emits `interaction.state: "submitted"`
- the original prompt stream continues after the response is accepted
- load session may emit the same `interaction_update` shape for any persisted
  elicitation request that is still pending

REST `sessionReply` remains only as the fallback for non-ACP sessions.

### Images

Desktop ACP prompt input supports user text and image content.

Manual testing confirmed user image history was missing after ACP load even
though the image content was persisted in `sessions.db`. Inline ACP load now
replays stored `MessageContent::Image` as ACP image chunks with replay metadata,
and the desktop adapter reconstructs them as Goose image content.

React StrictMode can start overlapping `session/load` calls in development. ACP
load notifications are scoped by `sessionId`, so a current subscriber may still
receive replay from an older in-flight load. Desktop now disposes stale ACP load
subscriptions on effect cleanup and treats identical replayed image chunks for
the same message as idempotent.

Remaining question: verify whether assistant image output can appear in Goose
sessions. If it can, the server replay path already routes image chunks by
message role, but user-facing rendering should still be manually checked.

### Redacted Thinking

`redactedThinking` is persisted by some provider formats, notably Bedrock
redacted reasoning content. The payload is opaque provider context, not
displayable reasoning text. Goose stores it so provider history can be
round-tripped, but desktop REST rendering does not expose it as thinking text.

Decision: intentionally omit `redactedThinking` from ACP desktop replay. ACP
load replay reconstructs the user-visible transcript; provider continuity still
comes from the stored backend conversation.

### Frontend Tool Request

`frontendToolRequest` is used by agent/provider plumbing for frontend tool calls
and provider history formatting. Desktop REST types include it as part of the
full `MessageContent` union, but desktop rendering code does not expose it as a
visible transcript row.

Decision: intentionally omit `frontendToolRequest` from ACP desktop replay.
Visible tool UX should continue to use normal tool request/response rendering,
ACP `requestPermission`, and Goose custom interaction/status updates. Revisit
only if legacy REST sessions are found where `frontendToolRequest` was relied on
as user-visible transcript content.

### Legacy Tool Confirmation Request

`toolConfirmationRequest` is a legacy message content shape. ACP live tool
approval uses the standard `requestPermission` request/response path, and
desktop locally projects that into the existing action-required UI. Keep this
deferred unless old persisted sessions need compatibility replay.

Decision: intentionally omit legacy `toolConfirmationRequest` from ACP desktop
replay for now. Current persisted approval content uses
`actionRequired.toolConfirmation`, and current live ACP approval uses
`requestPermission`. Old sessions can still load and continue without replaying
the legacy row; a cold-loaded pending approval from an old session is not
reliably actionable.

## Proposed Priority

This message/content work is now narrowed to replay/audit gaps. Recipe parity
is intentionally deferred and should not block these message-gap fixes.

Done:

- Stop creating new persisted `systemNotification.inlineMessage` command
  acknowledgements.
  - Persist `/clear` and `/compact` acknowledgements as normal assistant text.
  - Preserve `userVisible: true`, `agentVisible: false`.
- Add custom Goose `status_message` for live session status.
  - Use `_goose/session/update`.
  - Cover live compaction progress/status.
  - Desktop prompt handling should listen to the existing Goose custom
    notification router.
  - Status rows are local UI rows and must not be merge targets for later
    id-less transcript chunks.
- Represent credits exhausted as a structured `session/prompt` JSON-RPC error,
  not `status_message`.
- Keep ACP load behavior explicit for old persisted `systemNotification`
  content.
  - Current inline load skips these rows because `status_message` is live-only.
  - If a historical compatibility need appears, project legacy inline
    notifications to plain `agent_message_chunk` text in a targeted follow-up.
- For ACP sessions, use `_goose/elicitation/respond` and surface
  pending/submitted state through `_goose/session/update`
  `interaction_update`.
  - REST remains as the fallback for non-ACP sessions.
  - Keep tool confirmation on standard ACP `requestPermission`; do not
    duplicate it in the custom interaction update unless load replay later
    needs legacy pending permission compatibility.
- Replay stored image content during ACP load.
  - Manual test confirmed persisted user image history was missing after ACP
    reload.
  - Inline ACP load emits ACP image chunks with replay metadata.
  - Desktop treats duplicate identical image replay chunks as idempotent because
    overlapping `session/load` calls can emit duplicate replay for the same
    `sessionId`.
- Omit `redactedThinking` from ACP desktop replay.
  - It is opaque provider context, not displayable thinking text.
  - REST desktop does not render it as visible thinking content.
  - Provider continuity is preserved by the stored backend conversation, not by
    desktop transcript replay.
- Omit `frontendToolRequest` from ACP desktop replay.
  - It is provider/frontend-tool plumbing, not rendered by desktop REST UI.
  - User-visible tool UX is covered by normal tool request/response rendering,
    ACP permission requests, and Goose custom interaction/status updates.
- Omit legacy `toolConfirmationRequest` from ACP desktop replay.
  - Current approval paths are covered by `actionRequired.toolConfirmation` and
    ACP `requestPermission`.
  - Old sessions can still load and continue; only a legacy pending approval row
    would be absent from the replayed transcript.

Next:

1. Follow-up: make `systemNotification` structurally live-only.
   - Add code docs on `SystemNotificationContent` / constructors that durable
     acknowledgements must use normal assistant text with user-only metadata.
   - Audit current producers to confirm no intentional durable
     `systemNotification` remains after `/clear` and `/compact` move to text.
   - Add a persistence-boundary guard or test so new `systemNotification`
     messages are not accidentally stored as conversation history.
   - Keep read/render compatibility for legacy sessions that already contain
     persisted `systemNotification` content.

## Open Design Question

ACP core has generic displayable `ContentBlock` updates, tool updates, thoughts,
and `_meta`, but Goose `systemNotification` is a Goose-specific UI/content
concept. We need to choose a mapping that is useful to external ACP clients
without being desktop-only.

Candidate approaches:

- Encode as an `agent_message_chunk` text block with Goose `_meta` identifying
  the system notification type.
- Emit a Goose custom session notification through ACP `extNotification`.
- Add a broader Goose custom notification for message-content replay.

The first option is likely the smallest compatible bridge, but it may show
notification text as normal assistant text in non-Goose ACP clients. The second
is cleaner for Goose clients but invisible to generic ACP clients unless they
understand Goose extensions.

Current decision:

- Do not add a custom `message_content` replay schema for
  `systemNotification`.
- Treat future persisted `/clear` and `/compact` acknowledgements as normal
  assistant text.
- Treat live UI/session status as `status_message` on `_goose/session/update`.
- Use plain ACP text projection only as backward compatibility for older
  persisted inline system notifications.

This aligns with the slash-command lifecycle discussion in
`aaif-goose/goose#9261`: lifecycle/status should not be hidden inside
`agent_message_chunk` metadata. Standard ACP message chunks should remain
transcript content, while Goose-specific session state travels through typed
Goose custom session updates.
