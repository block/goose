# 17 ACP Elicitation Plan

## Goal

Make MCP elicitation work in ACP desktop sessions without using the REST
`sessionReply` response path.

Elicitation is a blocking user interaction, but it is not tool permission.
Tool permission should continue to use standard ACP `requestPermission`.
Elicitation should use Goose custom ACP messages because ACP prompt content does
not represent Goose `actionRequired.elicitationResponse`.

## Current Status

Implemented for ACP sessions.

- `_goose/session/update` accepts `interaction_update`.
- ACP load replays pending persisted elicitations and suppresses requests that
  already have a matching hidden response.
- The desktop adapter converts pending elicitation interactions into the
  existing `actionRequired.elicitation` message shape.
- ACP sessions submit responses through `_goose/elicitation/respond`.
- REST elicitation remains unchanged for non-ACP sessions.

## Compatibility

This migration must be additive.

- Keep REST elicitation unchanged for non-ACP sessions.
- Keep `actionRequired.elicitation` and
  `actionRequired.elicitationResponse` as Goose persisted message content.
- Keep ACP tool confirmation on standard `requestPermission`.
- Do not change existing `_goose/session/update` `usage_update`.
- Unknown custom session updates should be ignored by older desktop clients.
- Old persisted elicitation messages should be interpreted during ACP load:
  pending request without response becomes `interaction_update pending`; request
  with matching response must not show an active form.

## Wire Shape

Custom notification:

```ts
type GooseSessionUpdate =
  | UsageUpdate
  | InteractionUpdate;

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
- `cancelled` and `expired` may include `message`.

Custom request:

```ts
// method: _goose/elicitation/respond
type ElicitationRespondRequest = {
  sessionId: string;
  elicitationId: string;
  userData: unknown;
};
```

Response is empty.

## Server Implementation

1. Done: extend `crates/goose-sdk/src/custom_notifications.rs`.
   - Add `InteractionUpdate`.
   - Add `Interaction`.
   - Add `InteractionState`.
   - Add `GooseSessionUpdate::InteractionUpdate`.
   - Update the discriminator mapping.

2. Done: extend `crates/goose-sdk/src/custom_requests.rs`.
   - Add `ElicitationRespondRequest`.
   - Use method `_goose/elicitation/respond`.
   - Response type is `EmptyResponse`.

3. Done: add custom request dispatch.
   - Register `ElicitationRespondRequest` in
     `crates/goose/src/acp/server/custom_dispatch.rs`.
   - Implement handler on the ACP server.
   - Submit to `ActionRequiredManager::global().submit_response(...)`.
   - Persist a hidden `actionRequired.elicitationResponse` message.
   - Emit `interaction_update submitted`.

4. Done: emit pending elicitation during ACP prompt.
   - In `crates/goose/src/acp/server.rs`, when `handle_message_content` sees
     `ActionRequiredData::Elicitation`, send `_goose/session/update` with
     `interaction_update pending`.
   - Keep `ActionRequiredData::ToolConfirmation` on ACP `requestPermission`.

5. Done: add ACP load replay after the live path works.
   - Scan persisted messages for `actionRequired.elicitation`.
   - Scan persisted hidden responses for matching
     `actionRequired.elicitationResponse`.
   - Emit `pending` only when there is no matching response.

## Desktop Implementation

1. Done: widen `parseGooseSessionNotification`.
   - Accept `usage_update`.
   - Accept `interaction_update`.
   - Ignore unknown updates.

2. Done: adapt `interaction_update pending`.
   - Convert to the existing desktop `actionRequired.elicitation` message
     shape.
   - Reuse `ElicitationRequest`.

3. Partial: adapt `interaction_update submitted`.
   - Mark the pending form complete.
   - Prefer using the existing hidden response message shape if that avoids new
     UI state.

4. Done: change ACP submit path.
   - ACP sessions call `_goose/elicitation/respond`.
   - REST sessions keep current `sessionReply`.

## Incremental Order

1. Done: server schema and custom request definitions.
2. Done: server live pending emission.
3. Done: server response handler.
4. Done: desktop notification parser and adapter.
5. Done: desktop submit path.
6. Done: ACP load replay for pending persisted elicitations.
