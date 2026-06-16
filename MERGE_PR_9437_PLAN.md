# Merge Plan: PR 9437 Into `lifei/improve-elicitation-acp`

This document is the step-by-step plan for merging `origin/main` after PR 9437:

https://github.com/aaif-goose/goose/pull/9437

The rule for this merge is: explain one step, get review/approval, then make only that step's changes.

## Goal

Merge PR 9437 without losing the current branch's ACP elicitation implementation.

PR 9437 adds explicit `accept` / `decline` / `cancel` propagation for elicitation responses. This branch adds session-scoped ACP elicitation routing. The merge should combine both behaviors.

## Known Conflict Files

The simulated merge reports conflicts in these files:

1. `crates/goose/src/action_required_manager.rs`
2. `crates/goose/src/agents/agent.rs`
3. `crates/goose/src/agents/mcp_client.rs`
4. `crates/goose/src/acp/server.rs`

Related files likely needing follow-up edits:

1. `crates/goose/src/elicitation.rs`
2. `crates/goose-providers/src/conversation/message.rs`
3. Generated schema/API files, if OpenAPI output changes

## Review-Gated Steps

### Step 0: Safety Check

Explain:

- Confirm the working tree state.
- Create a local backup ref for the current branch before merging.
- Do not change code.

Expected commands:

```bash
git status --short --branch
git branch backup/lifei-improve-elicitation-acp-before-main HEAD
```

Review gate:

- Ask for approval before creating the backup ref.

### Step 1: Start the Merge

Explain:

- Merge `origin/main` into the current branch.
- Prefer merge over rebase to avoid rewriting the already pushed branch.
- Stop immediately after Git reports conflicts.

Expected command:

```bash
git merge origin/main
```

Review gate:

- Ask for approval before running the merge.

### Step 2: Resolve `action_required_manager.rs`

Explain:

- Keep this branch's session-scoped queue and claim model:
  - `session_id` on pending requests
  - `claim_response`
  - `PendingResponseClaim`
  - `drain_requests_for_session`
- Keep PR 9437's semantic action propagation:
  - accepted responses carry user data
  - declined responses carry `Decline`
  - cancelled responses carry `Cancel`
- Do not use PR 9437's older global `request_rx` model.

Target behavior:

- `request_and_wait` returns this branch's `ElicitationResponse` enum.
- Claims still validate the session id before submitting a response.
- Existing branch tests remain meaningful.
- Add or adjust tests for decline/cancel if needed.

Review gate:

- Show the intended diff or a concise before/after explanation.
- Ask for approval before editing the file.

### Step 3: Resolve Message Serialization

Explain:

- PR 9437 added an explicit `action` field to `ActionRequiredData::ElicitationResponse`.
- This branch must stop encoding decline/cancel as fake `user_data`.

Target behavior:

- `crates/goose-providers/src/conversation/message.rs` includes:
  - `action: ElicitationAction`
  - serde default of `Accept` for backward compatibility
  - `action_required_elicitation_response(id, user_data, action)`
- `crates/goose/src/elicitation.rs` builds response messages with the explicit action field.
- Decline/cancel generated messages should use empty object user data plus explicit action.

Review gate:

- Explain exactly how decline/cancel will be represented.
- Ask for approval before editing.

### Step 4: Resolve `agents/agent.rs`

Explain:

- Incoming `ActionRequiredData::ElicitationResponse` now has an explicit `action`.
- The agent should convert that action into the branch's `ElicitationResponse` enum.
- The session-scoped `complete_elicitation_with_message` path should stay.

Target behavior:

- `Accept` becomes `ElicitationResponse::Accept(user_data.clone())`.
- `Decline` becomes `ElicitationResponse::Decline`.
- `Cancel` becomes `ElicitationResponse::Cancel`.
- The response message is persisted once.

Review gate:

- Show the conversion logic.
- Ask for approval before editing.

### Step 5: Resolve `agents/mcp_client.rs`

Explain:

- The MCP client should return the real elicitation action to the MCP server.
- Only accepted responses should include content.

Target behavior:

- `ElicitationResponse::Accept(user_data)` returns MCP `Accept` with content.
- `ElicitationResponse::Decline` returns MCP `Decline` without content.
- `ElicitationResponse::Cancel` returns MCP `Cancel` without content.

Review gate:

- Show the match statement shape.
- Ask for approval before editing.

### Step 6: Resolve `acp/server.rs`

Explain:

- The conflict is delete-vs-edit.
- This branch removed the old `_goose/unstable/elicitation/respond` handler and routes through standard ACP form elicitation.
- PR 9437 edited that old handler to force `Accept`.

Target behavior:

- Keep this branch's standard ACP `handle_form_elicitation` path.
- Do not reintroduce the old custom response handler unless a caller still needs it.
- Make sure the standard ACP path records explicit accept/decline/cancel actions through the shared elicitation helper.

Review gate:

- Explain why the old handler can stay removed or why it must come back.
- Ask for approval before editing.

### Step 7: Format

Explain:

- Run formatting after conflict resolution.

Expected command:

```bash
cargo fmt
```

Review gate:

- Ask for approval before running formatting.

### Step 8: Focused Tests

Explain:

- Run only focused Rust tests first.
- Broader tests can follow if focused tests pass.

Expected commands:

```bash
cargo test -p goose action_required_manager
cargo test -p goose elicitation
```

Review gate:

- Ask for approval before running tests.

### Step 9: Generated API Check

Explain:

- Because PR 9437 changed the message schema, generated API files may need regeneration.
- Only run generation if the merge changes server/OpenAPI schema output.

Expected command if needed:

```bash
just generate-openapi
```

Review gate:

- Explain why generation is or is not needed.
- Ask for approval before running generation.

### Step 10: Final Diff Review

Explain:

- Inspect the final diff against `origin/main`.
- Confirm the merge did not turn decline/cancel into accept.
- Confirm no fake action payload remains.

Expected checks:

```bash
git diff --check
rg -n '"action": "decline"|"action": "cancel"|submit_response|action_required_elicitation_response' crates/goose crates/goose-providers
```

Review gate:

- Present the final summary.
- Ask for approval before any commit or push.

## Merge Invariants

The final merge should preserve these invariants:

1. ACP elicitation requests are session-scoped.
2. A response for the wrong session cannot consume a pending elicitation.
3. `Accept` sends user data to the MCP server.
4. `Decline` sends a decline action to the MCP server.
5. `Cancel` sends a cancel action to the MCP server.
6. Decline/cancel are not represented as fake user data.
7. Transcript serialization remains backward-compatible with older messages that do not have `action`.
