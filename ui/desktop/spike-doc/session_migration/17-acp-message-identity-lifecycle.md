# ACP Message Identity Lifecycle

## Goal

ACP clients should not have to guess which Goose message an update belongs to.
Every ACP update that contributes to, annotates, or represents a Goose
`Message` should carry Goose message identity:

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
- Missing `messageId` means the update is uncorrelated or synthetic. Clients
  should not merge it into an existing message row unless the update type has
  its own grouping semantics.
- `created` is the Goose message creation timestamp.

## Prompt Lifecycle

ACP `session/prompt` receives ACP prompt content, converts it into a Goose
`Message`, and calls `agent.reply(...)`.

The agent emits live `AgentEvent::Message(message)` events before all assistant
messages are persisted. ACP server converts each live Goose message into one or
more ACP notifications.

Current prompt flow:

1. ACP prompt input is converted into a Goose user `Message`.
2. The agent persists the user message before model work.
3. Provider and agent logic emit assistant/user `AgentEvent::Message(message)`
   values live.
4. ACP server reads `message.id` and `message.created`.
5. ACP server emits ACP session updates.
6. The agent later persists messages collected during the turn.

Important consequence:

- ACP live prompt does not wait for persistence and does not get an id back
  from storage.
- If the live Goose `Message` has `id: Some(...)`, ACP can return that id.
- If the live Goose `Message` has `id: None`, ACP can only return `created`
  unless the message is assigned an id before ACP conversion.

Desired invariant:

- Every Goose message should get an id before it is either emitted live or
  persisted.

## Load Session Lifecycle

ACP `session/load` reads the persisted conversation and replays it as ACP
session notifications.

For load-session replay:

- `_meta.goose.messageId` comes from the persisted Goose `Message.id`.
- `_meta.goose.created` comes from the persisted Goose `Message.created`.
- Therefore, load-session ids are persisted ids.

Inline load should not replay live-only status as transcript history. Persisted
legacy `systemNotification` content is skipped in inline load.

## Update Matrix

| Update | Load Session Identity | Prompt Identity | Prompt Id Persistence |
| --- | --- | --- | --- |
| `user_message_chunk` | `_meta.goose.messageId` from persisted user message | Usually not emitted for prompt input | N/A |
| `agent_message_chunk` | `_meta.goose.messageId` from persisted assistant message | Should include `_meta.goose.messageId` from live Goose message when present | Same as persisted only if the live message id is also persisted |
| `agent_thought_chunk` | `_meta.goose.messageId` from persisted thinking message | Should include `_meta.goose.messageId` from live Goose message when present | Same as persisted only if the live message id is also persisted |
| `tool_call` | `_meta.goose.messageId` from persisted assistant tool-request message | Should include owning assistant message id | Tool request messages often have generated ids, but live ACP must merge that id into metadata |
| `tool_call_update` | `_meta.goose.messageId` from persisted user tool-response message | Should include owning user/tool-response message id | Tool response messages often have generated ids, but live ACP must merge that id into metadata |
| `interaction_update` pending elicitation | `_meta.goose.messageId` from persisted assistant action-required message | Includes live action-required message id when present | Same as persisted only if the live message id is also persisted |
| `interaction_update` submitted elicitation | Not replayed as pending; submitted responses suppress stale pending requests | Includes generated user elicitation response message id | Response path creates/persists the response message |
| `status_message` from `SystemNotification` | Not replayed; live-only status | Should include source message id when present | Many current status producers use `Message::assistant()` with no id unless normalized |
| `usage_update` | Session-level, no message id | Session-level, no message id | N/A |
| `session_info_update` | Session-level, no message id | Session-level, no message id | N/A |
| `config_option_update` | Session-level, no message id | Session-level, no message id | N/A |

## Current Gaps

### Prompt Chunks Without Guaranteed IDs

Live prompt text/thinking/status updates can only pass through a message id if
the source Goose `Message` already has one.

Some prompt messages are created with `Message::assistant()`, which defaults to
`id: None`.

Gap:

- ACP prompt needs a normalization step so Goose-originated messages have ids
  before ACP emits updates.

### Live Tool Metadata

Load-session tool replay already merges persisted message identity into
`tool_call` and `tool_call_update`.

Live prompt tool updates still need the same treatment:

- `tool_call` should merge `_meta.goose.messageId/created` for the owning
  assistant tool-request message.
- `tool_call_update` should merge `_meta.goose.messageId/created` for the
  owning user/tool-response message.
- Synthetic title/summary updates should preserve tool identity metadata and
  include message identity if they are updating a known Goose message.

### Status Messages

`status_message` is live UI/session status, not transcript content.

It may still include `_meta.goose.messageId/created` for correlation,
ordering, de-duplication, and debugging. Clients should not replay it as
conversation history.

### Client Fallback

ACP allows chunks without ids. Desktop should keep a defensive fallback, but it
must not merge id-less chunks into live-only status rows.

Client rule:

- Prefer ACP `messageId` or `_meta.goose.messageId`.
- If no id exists, create a standalone row or merge only into a known
  transcript row.
- Never merge id-less transcript chunks into a live-only status row.

## Desired Interface Contract

For Goose-originated ACP updates:

- Message-derived updates carry `_meta.goose.messageId/created`.
- Session-level updates do not carry message identity.
- Live-only status updates may carry message identity for correlation, but
  remain non-transcript.
- Missing identity is reserved for synthetic or uncorrelated updates and should
  be rare.

