# ACP Error Handling

## Protocol Baseline

ACP uses JSON-RPC 2.0 request errors for method failures.

- Requests return either `result` or `error`.
- Errors include `code` and `message`, with optional `data`.
- Notifications do not receive success or error responses.

Relevant ACP docs:

- https://agentclientprotocol.com/protocol/overview#error-handling
- https://agentclientprotocol.com/protocol/prompt-turn

## Prompt Failure Shape

For generic prompt-stream failures, Goose ACP should fail the original
`session/prompt` request with a JSON-RPC error.

Current server behavior:

```rust
Err(e) => {
    return Err(agent_client_protocol::Error::internal_error()
        .data(format!("Error in agent response stream: {}", e)));
}
```

Conceptual JSON-RPC response:

```json
{
  "jsonrpc": "2.0",
  "id": "...",
  "error": {
    "code": -32603,
    "message": "Internal error",
    "data": "Error in agent response stream: ..."
  }
}
```

This follows the ACP method-error model. Goose should not invent a generic
`session/update` error notification for these failures.

## Actionable Domain Errors

Some provider failures are actionable domain state and also terminal prompt
failures. These should use the normal ACP JSON-RPC method error shape with
small structured `data`.

Example: credits exhausted.

Historical behavior before the ACP prompt-error translation:

- Provider returns `ProviderError::CreditsExhausted`.
- Agent emits `SystemNotificationType::CreditsExhausted`.
- ACP mapped that to an actionable `status_message` variant.

Preferred ACP behavior:

```json
{
  "jsonrpc": "2.0",
  "id": "...",
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

Without a recovery URL:

```json
{
  "jsonrpc": "2.0",
  "id": "...",
  "error": {
    "code": -32603,
    "message": "Please check your account with your provider to add more credits, then resend your message to continue.",
    "data": {
      "reason": "credits_exhausted"
    }
  }
}
```

Rules:

- `error.message` is the user-facing message.
- `error.data.reason` is the machine-readable reason.
- `error.data.url` is an optional recovery URL.
- Desktop should render this as the existing credits-exhausted card, not as a
  generic blocking error.
- `status_message` should be reserved for non-terminal live status/progress.

## Desktop Behavior

REST/SSE generic stream errors used to arrive as:

```json
{
  "type": "Error",
  "error": "..."
}
```

Desktop displayed them via:

```ts
onFinish('Stream error: ' + errorMsg)
```

ACP generic prompt failures currently reject `acpPromptSession(...)` and
desktop displays:

```ts
onFinish('Submit error: ' + errorMessage(error))
```

This is functional, but the wording is not ideal after a prompt has already
started and streamed updates.

Desktop now has a client-side parser for the preferred structured ACP
credits-exhausted error shape. When `error.data.reason === "credits_exhausted"`
arrives from `session/prompt`, desktop appends the existing
`creditsExhausted` system notification card and finishes the prompt without
showing the generic submit error.

## Follow-Ups

1. Rename the ACP prompt catch label from `Submit error:` to `Stream error:` or
   `Agent error:` for better parity with REST and clearer user wording.
2. Verify cancellation edge cases:
   - expected cancellation should resolve `session/prompt` with
     `stopReason: "cancelled"`
   - expected cancellation should not surface as a JSON-RPC error in desktop
3. Introduce a structured internal `AgentEvent::PromptError` so ACP can map
   network and generic provider failures without parsing assistant text.
