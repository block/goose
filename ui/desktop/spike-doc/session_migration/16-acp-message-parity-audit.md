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
| `image` | Native `MessageContent` | Not currently replayed | Prompt input conversion supports user images; live image output not mapped | Converts ACP image chunks if received | Partial |
| `toolRequest` | Native `MessageContent` | `tool_call` | `tool_call` | Converts to `toolRequest` | Covered |
| `toolResponse` | Native `MessageContent` | `tool_call_update` | `tool_call_update` | Converts to `toolResponse` | Covered |
| `thinking` | Native `MessageContent` | `agent_thought_chunk` | `agent_thought_chunk` | Converts to `thinking` | Covered |
| `actionRequired.toolConfirmation` | Native `MessageContent` | Not replayed as `actionRequired` | ACP permission request path | Creates `actionRequired.toolConfirmation` from permission request | Partial |
| `actionRequired.elicitation` | Native `MessageContent` | `_goose/session/update` `interaction_update` for pending persisted elicitations | `_goose/session/update` `interaction_update` | Creates `actionRequired.elicitation` from pending interaction updates | Covered for pending requests |
| `actionRequired.elicitationResponse` | Native hidden message | Hidden/submitted responses are not replayed as visible content | ACP sessions submit via `_goose/elicitation/respond`; REST remains for non-ACP sessions | Response is not rendered as a normal visible message | Covered for ACP sessions, hidden by design |
| `systemNotification.inlineMessage` | Native `MessageContent` | Dropped | Dropped | No ACP mapping | Gap |
| `systemNotification.thinkingMessage` | Native `MessageContent` | Dropped | Dropped | No ACP mapping | Gap |
| `systemNotification.creditsExhausted` | Native `MessageContent` | Dropped | Dropped | No ACP mapping | Gap |
| `redactedThinking` | Native `MessageContent` | Dropped | Dropped | No ACP mapping | Gap or defer after provider check |
| `frontendToolRequest` | Native `MessageContent` | Dropped | Dropped | No ACP mapping | Gap or defer after production check |
| `toolConfirmationRequest` | Native legacy content | Dropped | Dropped | Existing REST UI helpers can read it | Likely defer/legacy |

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

ACP currently drops these notifications during both load replay and live prompt
streaming. This can make ACP sessions look different from REST sessions after
commands, compaction, or billing/context-limit events.

Current `systemNotification` variants:

- `inlineMessage`
  - UI behavior: small inline status row.
  - Current persisted uses:
    - `/clear`: `Conversation cleared`
    - `/compact`: `Compaction complete`
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
- Existing persisted `systemNotification.inlineMessage` rows are legacy
  compatibility cases. ACP load replay may project them to plain
  `agent_message_chunk` text if needed.

This keeps ACP transcript replay simple and keeps UI/status concepts out of
standard ACP assistant message chunks.

### Goose Status Updates

Live Goose UI/session status should use the existing custom notification
channel:

- method: `_goose/session/update`
- payload type: `GooseSessionNotification`

Add a typed `status_update` variant to the existing `GooseSessionUpdate` union:

```ts
type GooseSessionUpdate =
  | UsageUpdate
  | StatusUpdate
  | InteractionUpdate;

type StatusUpdate = {
  sessionUpdate: 'status_update';
  status: SessionStatus;
};

type SessionStatus =
  | {
      type: 'info';
      message: string;
    }
  | {
      type: 'progress';
      message: string;
      activity?: 'compaction';
    }
  | {
      type: 'action_required';
      action: 'add_credits';
      message: string;
      url?: string | null;
    };
```

Mapping from current `systemNotification`:

- `inlineMessage` -> `status.type = 'info'`
- `thinkingMessage` -> `status.type = 'progress'`,
  `activity = 'compaction'` when it represents compaction
- `creditsExhausted` -> `status.type = 'action_required'`,
  `action = 'add_credits'`, `url = data.top_up_url`

Example compaction progress update:

```json
{
  "method": "_goose/session/update",
  "params": {
    "sessionId": "s1",
    "update": {
      "sessionUpdate": "status_update",
      "status": {
        "type": "progress",
        "activity": "compaction",
        "message": "goose is compacting the conversation..."
      }
    }
  }
}
```

Example credits update:

```json
{
  "method": "_goose/session/update",
  "params": {
    "sessionId": "s1",
    "update": {
      "sessionUpdate": "status_update",
      "status": {
        "type": "action_required",
        "action": "add_credits",
        "message": "Please add credits to your account, then resend your message to continue.",
        "url": "https://..."
      }
    }
  }
}
```

This schema describes domain state, not presentation. Desktop can map:

- `info` to an inline notice or other local presentation
- `progress` with `activity = 'compaction'` to compacting/loading state
- `action_required/add_credits` to the credits warning UI

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
    state: 'pending' | 'submitted' | 'cancelled' | 'expired';
    message?: string;
    requestedSchema?: unknown;
  };
};
```

Rules:

- `pending` includes `message` and `requestedSchema`.
- `submitted`, `cancelled`, and `expired` only require `id` and `state`.
- `cancelled` and `expired` may include `message` if there is a useful reason
  to show.

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

ACP load replay currently does not replay stored image message content. ACP
adapter can reconstruct image chunks if the server emits them, but the server
does not currently emit image chunks during replay. Need to verify whether
assistant image output can appear in Goose sessions and whether user image
history must be visible after reload.

### Redacted Thinking

`redactedThinking` is persisted by some provider formats. ACP currently drops
it. Need to decide whether this is acceptable because the content is redacted,
or whether the UI should render a placeholder equivalent to REST behavior.

### Frontend Tool Request

`frontendToolRequest` appears in provider formatting and context management, but
ACP currently drops it. Need to verify whether desktop sessions can contain it
as user-visible content. If it is internal/provider-facing only, defer.

## Proposed Priority

1. Stop creating new persisted `systemNotification.inlineMessage` command
   acknowledgements.
   - Persist `/clear` and `/compact` acknowledgements as normal assistant text.
   - Preserve `userVisible: true`, `agentVisible: false`.

2. Add custom Goose `status_update` for live session status.
   - Use `_goose/session/update`.
   - Cover live compaction progress and credits-exhausted UI first.
   - Desktop prompt handling should listen to the existing Goose custom
     notification router.

3. Optionally add ACP load backcompat for old persisted
   `systemNotification.inlineMessage`.
   - Project legacy inline notifications to plain `agent_message_chunk` text.
   - Do not add nested Goose message-content metadata unless exact historical
     desktop styling becomes required.

4. Add ACP handling for image replay if manual testing confirms user image
   history is missing after ACP load.

5. Done for ACP sessions: use `_goose/elicitation/respond` and surface
   pending/submitted state through `_goose/session/update`
   `interaction_update`.
   - REST remains as the fallback for non-ACP sessions.
   - Keep tool confirmation on standard ACP `requestPermission`; do not
     duplicate it in the custom interaction update unless load replay later
     needs legacy pending permission compatibility.

6. Audit `redactedThinking`, `frontendToolRequest`, and legacy
   `toolConfirmationRequest` with real sessions or targeted tests before
   implementing mappings.

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
- Treat live UI/session status as `status_update` on `_goose/session/update`.
- Use plain ACP text projection only as backward compatibility for older
  persisted inline system notifications.

This aligns with the slash-command lifecycle discussion in
`aaif-goose/goose#9261`: lifecycle/status should not be hidden inside
`agent_message_chunk` metadata. Standard ACP message chunks should remain
transcript content, while Goose-specific session state travels through typed
Goose custom session updates.
