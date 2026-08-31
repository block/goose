## Clarified Problem Statement

**Goal:** When a provider hits an output-token limit (`finish_reason=length` / `output_token_limit_reached`), goose should recover like pi: fail truncated tool calls and keep the turn going so the model can re-issue them; compact-and-retry once when the stop is recoverable (output below the intended max); and use a higher default output cap so the warning is rarer. After one bounded recovery, still show the existing incomplete-response warning.

**Constraints:**
- Must-have: both agent loops (`crates/goose/src/agents/agent.rs` and `crates/goose/src/agents/state_machine/` + `crates/goose-agent/src/inference.rs`). `GOOSE_STATE_MACHINE=1` is not optional for loop-behavior changes.
- Must-have: CLI, desktop, and ACP all see the same recovery (warning copy lives in `crates/goose-cli/src/session/output.rs` and `ui/desktop/src/components/GooseMessage.tsx`; ACP maps the flag to `StopReason::MaxTokens` in `crates/goose/src/acp/server.rs`).
- Must-have: truncated tool arguments are never executed. Providers already convert them to `ToolRequest(Err(...))` in `crates/goose-provider-types/src/formats/openai.rs` (and Anthropic / Responses equivalents). Recovery must feed those errors back and continue, not execute them.
- Can't-break: context-window compaction (`ProviderError::ContextLengthExceeded` in the legacy loop) stays separate from output-token recovery.
- Can't-break: existing `output_token_limit_reached` persistence, ACP fallback text, and the user-visible warning after recovery is exhausted.
- One automatic compact-and-retry for recoverable length stops, then warn. No unbounded continuation stitching.

**Non-goals:**
- Hidden “please continue from where you left off” text-stitching of a true max-tokens cutoff.
- Changing ACP wire types or regenerating `ui/desktop/src/api`.
- Opening a PR (local / exploratory only).
- Reworking context-overflow detection beyond reusing existing compaction.

**Success criteria:**
- Truncated tool-call responses: tools are not executed; each gets a truncation error; the agent continues the turn and the model can re-issue complete calls (both loops).
- Recoverable length stop (usage.output < intended `max_output_tokens`, same model): one compact-and-retry, then continue the interrupted turn.
- True max-tokens text cutoff after that bound: existing warning still appears; ACP still reports `StopReason::MaxTokens`.
- Unknown / catalog-missing models no longer silently sit on the 4096 `max_output_tokens()` fallback when a better policy is chosen (see Approach A vs C).
- Tests cover both loops: truncated tools continue; recoverable length compact-retries once; second hit still warns.

## Approaches Considered

### Approach A: Pi-parity recovery + raise unknown-model fallback
- Sketch: Add a recoverable-length check (output tokens < intended max, same model) in both loops. On recoverable length with no executable tools: drop/compact the truncated assistant message once, then continue. On truncated tools: keep current `ToolRequest(Err)` conversion, emit those as tool results, do not `exit_chat` / do not treat as `ends_turn`. Raise `ModelConfig::max_output_tokens()` fallback (4096 today) to a larger constant (e.g. 32k) or omit `max_tokens` when unset so providers use their own default. Canonical catalog limits in `with_canonical_limits` stay as-is.
- Affected files: `crates/goose/src/agents/agent.rs` (stop using `provider_reached_output_token_limit` as a hard turn end; add compact-and-retry beside existing `ContextLengthExceeded` compaction); `crates/goose/src/agents/state_machine/` + `crates/goose-agent/src/inference.rs` / `operation.rs` (`ends_turn` currently ignores `output_token_limit_reached` but also ignores error-only tool requests — truncated `ToolRequest(Err)` may already look like end-of-turn if no `Ok` tools remain); `crates/goose-provider-types/src/model.rs` (`max_output_tokens`); tests in `agent.rs`, `provider_lifecycle.rs`, CLI/UI warning tests.
- Tradeoffs: Closest to pi and to the chosen answers. Fallback bump is a one-line policy change with wide blast radius (every unknown model). Omitting `max_tokens` when unset is closer to Bedrock’s “don’t pin 4096” comment but may surprise providers that require the field.
- Effort: M

### Approach B: Recovery only, leave 4096 fallback
- Sketch: Same truncated-tool continuation and one-shot compact-and-retry as A, but do not touch `max_output_tokens()`. Users who hit 4096 on unknown models still recover once if usage.output < 4096 is false (i.e. they truly hit 4096) — so the “raise the cap” half of 3D is skipped. Warning remains common for uncatalogued models.
- Affected files: same agent-loop files as A; not `model.rs`.
- Tradeoffs: Smaller, safer diff. Does not match 3D. Recoverable-length (output < intended max) still helps when a server cuts off early because the context was full; it does not help when goose itself asked for only 4096.
- Effort: M (slightly smaller than A)

### Approach C: Prefer catalog / omit default max_tokens; recovery as in A
- Sketch: Same loop recovery as A, but instead of raising the 4096 constant, stop sending a default `max_tokens` when unset (`max_output_tokens()` becomes unused for request payloads; providers that already omit when `None` — Bedrock — become the norm). Catalog `limit.output` still applied via `with_canonical_limits` when it is strictly below context. Unknown models inherit the provider’s server default rather than goose’s 4096.
- Affected files: A’s loop files plus every provider payload builder that calls `max_output_tokens()` (e.g. `crates/goose-providers/src/databricks_v2.rs`, OpenAI formatters, Ollama `num_predict`). Tests that assert payload `max_tokens: 4096`.
- Tradeoffs: Best long-term cap policy (matches existing Bedrock comment: don’t pin 4096). Larger provider-surface change; some APIs require `max_tokens` and would need a per-provider fallback. Higher chance of hitting *provider* output caps, which recovery then handles.
- Effort: L

## Recommendation

Approach A. It matches 1B / 2A / 3D / 4A / 5C without ripping out every provider’s `max_tokens` wiring. Implement truncated-tool continuation and one compact-and-retry in **both** loops first; bump the 4096 fallback as a separate, easy follow-through in the same change. Revisit omitting `max_tokens` (C) only if catalog coverage is poor enough that a larger constant still clips real models.

Pi reference (do not copy blindly):
- Truncated tools: `packages/agent/src/agent-loop.ts` `failToolCallsFromTruncatedMessage` — fail all, `terminate: false`, loop continues.
- Recoverable length: `isRecoverableLength` in `packages/ai/src/utils/overflow.ts` — `stopReason === "length" && desiredMaxOutput > 0 && usage.output < desiredMaxOutput`; session then compact-and-retry once (`packages/coding-agent/src/core/agent-session.ts`).
- True max-tokens cutoff: pi also stops; no text stitching.

## Open questions

- Exact new fallback: 32_768 vs 64_768 vs “omit when unset” for OpenAI-compatible providers only. Not blocking if A ships a documented constant.
- Whether recoverable-length compact should reuse `compact_messages` (legacy loop ~agent.rs:2973) or a lighter “drop last assistant + retry” like pi. Prefer reusing compaction so context-full length stops actually make room.
- State-machine: truncated `ToolRequest(Err)` messages may already `ends_turn` (assistant, no `Ok` tool request). Confirm and fix `ends_turn` / toolcalling so error-only truncated tools still produce tool-result messages and another inference.
- Should desktop/CLI hide the warning for the *first* recoverable attempt and only show it after the bound is exhausted? Chosen 5C says yes — keep the string, change when it fires.
- `GOOSE_MAX_TOKENS` already overrides; do not clobber an explicit user cap.

Next: run `/ship --from-brainstorm docs/brainstorms/2026-08-31-output-token-limit-recovery.md` to implement, or `/ship --plan-only` to see a detailed plan first. Do not open a PR.
