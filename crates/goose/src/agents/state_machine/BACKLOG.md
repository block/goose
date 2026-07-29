# State Machine Backlog

This is the remaining work before the state machine can replace the old agent
loop. Tests should be larger lifecycle scenarios. The linked issues explain
the regressions each scenario must catch; they are not requests for separate
tests.

## Correctness

- [ ] Remove hidden execution state from operations. After any applied step it
  must be possible to discard the machine, reconstruct the same pipeline from
  the persisted session, and produce the same next step. This currently fails
  for retry attempts and completion, goal nudging, in-memory goal and grind
  values, stop-hook block counts, and inference entry-hook accounting.

- [ ] Add a reconstruction test that discards and rebuilds the machine after
  every applied step in a tool-calling turn. Extend it to retries, stop-hook
  denials, compaction, and cancellation as those paths become stateless.

- [ ] Make applying a `StepResult` atomic or safely resumable. A process crash
  between effects must not leave half-hidden tool pairs, replaced history with
  stale usage, or another state that cannot determine its next step.

- [ ] Define replay semantics for external side effects. A persisted pending
  tool request can be resumed before dispatch, but a crash after the tool ran
  and before its response was persisted cannot provide exactly-once execution
  without idempotency or a durable dispatch protocol. Hooks have the same
  problem.

- [ ] Discover nested `AGENTS.md` and `.goosehints` files from paths accessed by
  earlier tool calls. Instruction discovery must use conversation state rather
  than mutable path tracking in the prompt manager
  ([#5840](https://github.com/aaif-goose/goose/issues/5840),
  [#4336](https://github.com/aaif-goose/goose/issues/4336)).

- [x] Preserve reasoning and text when a streamed response also contains tool
  calls, including malformed calls. The complete provider response must be
  persisted once and sent back to the provider once
  ([#9675](https://github.com/aaif-goose/goose/issues/9675)).

- [x] Make cancellation leave a valid conversation. Every in-flight tool
  request needs a matching interrupted response, completed work must remain
  persisted, and a later user turn must succeed
  ([#1541](https://github.com/aaif-goose/goose/issues/1541),
  [#1624](https://github.com/aaif-goose/goose/issues/1624),
  [#2337](https://github.com/aaif-goose/goose/issues/2337),
  [#2947](https://github.com/aaif-goose/goose/issues/2947),
  [#4252](https://github.com/aaif-goose/goose/issues/4252),
  [#7827](https://github.com/aaif-goose/goose/issues/7827)).

## Lifecycle Scenarios

- [x] Extend the provider lifecycle scenario with the standard prompt after a
  model change and unusual valid tool schemas
  ([#4879](https://github.com/aaif-goose/goose/issues/4879),
  [#3348](https://github.com/aaif-goose/goose/issues/3348)).

- [x] Extend the steering lifecycle scenario with FIFO draining, steering
  during inference, cancellation, and compaction near the context limit
  ([#9037](https://github.com/aaif-goose/goose/issues/9037),
  [#1700](https://github.com/aaif-goose/goose/issues/1700),
  [#6579](https://github.com/aaif-goose/goose/issues/6579),
  [#8406](https://github.com/aaif-goose/goose/issues/8406)).

- [x] Add an MCP prompt lifecycle scenario that preserves every alternating
  message returned by `/prompt`
  ([#6506](https://github.com/aaif-goose/goose/issues/6506)).

- [x] Add one recipe and scheduling lifecycle scenario. Cover final-output tool
  advertisement and invalid schemas, optional and parent parameters, intended
  extension tools, child `max_turns`, scheduled Auto mode, conditional
  scheduler advertisement, invalid schedules, persisted message counts, and
  mode-specific delegation tools
  ([#3700](https://github.com/aaif-goose/goose/issues/3700),
  [#10491](https://github.com/aaif-goose/goose/issues/10491),
  [#6232](https://github.com/aaif-goose/goose/issues/6232),
  [#3730](https://github.com/aaif-goose/goose/issues/3730),
  [#7353](https://github.com/aaif-goose/goose/issues/7353),
  [#6078](https://github.com/aaif-goose/goose/issues/6078),
  [#6198](https://github.com/aaif-goose/goose/issues/6198),
  [#3882](https://github.com/aaif-goose/goose/issues/3882),
  [#6023](https://github.com/aaif-goose/goose/issues/6023),
  [#6405](https://github.com/aaif-goose/goose/issues/6405),
  [#7431](https://github.com/aaif-goose/goose/issues/7431),
  [#10016](https://github.com/aaif-goose/goose/issues/10016),
  [#5140](https://github.com/aaif-goose/goose/issues/5140)).

- [x] Add a reconstruction and isolation scenario that creates a second
  pipeline over the same persisted session. Cover mode, provider, model,
  extension and recipe state, per-model cost, context and turn limits,
  working-directory isolation, elicitation correlation, cache usage fields,
  and tool session IDs
  ([#7603](https://github.com/aaif-goose/goose/issues/7603),
  [#7615](https://github.com/aaif-goose/goose/issues/7615),
  [#5358](https://github.com/aaif-goose/goose/issues/5358),
  [#6141](https://github.com/aaif-goose/goose/issues/6141),
  [#7839](https://github.com/aaif-goose/goose/issues/7839),
  [#7609](https://github.com/aaif-goose/goose/issues/7609),
  [#6909](https://github.com/aaif-goose/goose/issues/6909),
  [#9870](https://github.com/aaif-goose/goose/issues/9870),
  [#4988](https://github.com/aaif-goose/goose/issues/4988),
  [#6308](https://github.com/aaif-goose/goose/issues/6308)).

## Test Support

- [x] Let `DummyApi` gate a response so cancellation and steering can happen
  while inference is active.

- [x] Use the calculator's barrier and existing elicitation support to control
  completion order and cancellation without adding scripted command behavior.

- [x] Let `test_pipeline` reconstruct a pipeline around an existing
  `SessionManager` and session ID.

- [x] Add assertions for advertised tools and submitted system prompts.

- [x] Add assertions for `trace_output` and terminal usage without inspecting
  raw event fragments.

## API and Migration

- [ ] Run the standard pipeline in production long enough to establish parity,
  remove the `GOOSE_STATE_MACHINE` flag, delete `reply_internal`, and remove
  old-loop-only frontend tool handling.

## Deferred Outside the State Machine

- [ ] Improve the shared provider HTTP error mapper for empty response bodies
  ([#5528](https://github.com/aaif-goose/goose/issues/5528)).

- [ ] Add an in-process MCP transport test for wire-level `JsonRpcError`
  handling ([#2884](https://github.com/aaif-goose/goose/issues/2884)).

- [ ] Cover explicit resource ownership and runtime extension naming in the
  extension manager
  ([#8988](https://github.com/aaif-goose/goose/issues/8988),
  [#6188](https://github.com/aaif-goose/goose/issues/6188)).
