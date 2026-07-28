# State Machine Backlog

This is the remaining work before the state machine can replace the old agent
loop. Tests should be larger lifecycle scenarios. The linked issues explain
the regressions each scenario must catch; they are not requests for separate
tests.

## Correctness

- [ ] Remove hidden execution state from operations. After any applied step it
  must be possible to discard the machine, reconstruct the same pipeline from
  the persisted session, and produce the same next step. This currently fails
  for tool-pair `batch_attempted`, retry attempts and completion, goal nudging,
  in-memory goal and grind values, stop-hook block counts, and inference
  entry-hook accounting.

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

- [ ] Notify the client when tool-pair compaction changes message visibility.
  Persisting `SetMessageVisibility` and summaries without a replacement event
  lets a stale client restore the old history on its next round trip
  ([#7413](https://github.com/aaif-goose/goose/issues/7413)).

- [ ] Discover nested `AGENTS.md` and `.goosehints` files from paths accessed by
  earlier tool calls. Instruction discovery must use conversation state rather
  than mutable path tracking in the prompt manager
  ([#5840](https://github.com/aaif-goose/goose/issues/5840),
  [#4336](https://github.com/aaif-goose/goose/issues/4336)).

- [ ] Preserve reasoning and text when a streamed response also contains tool
  calls, including malformed calls. The complete provider response must be
  persisted once and sent back to the provider once
  ([#9675](https://github.com/aaif-goose/goose/issues/9675),
  [#7425](https://github.com/aaif-goose/goose/issues/7425)).

- [ ] Make cancellation leave a valid conversation. Every in-flight tool
  request needs a matching interrupted response, completed work must remain
  persisted, and a later user turn must succeed
  ([#1541](https://github.com/aaif-goose/goose/issues/1541),
  [#1624](https://github.com/aaif-goose/goose/issues/1624),
  [#2337](https://github.com/aaif-goose/goose/issues/2337),
  [#2947](https://github.com/aaif-goose/goose/issues/2947),
  [#4252](https://github.com/aaif-goose/goose/issues/4252),
  [#7827](https://github.com/aaif-goose/goose/issues/7827)).

## Lifecycle Scenarios

- [ ] Extend the provider lifecycle scenario with reasoning, mixed text and
  tool output, stable and custom system prompts, model-specific prompts,
  unusual valid tool schemas, and recovery on a later turn after provider
  errors
  ([#2431](https://github.com/aaif-goose/goose/issues/2431),
  [#4610](https://github.com/aaif-goose/goose/issues/4610),
  [#1800](https://github.com/aaif-goose/goose/issues/1800),
  [#4879](https://github.com/aaif-goose/goose/issues/4879),
  [#3348](https://github.com/aaif-goose/goose/issues/3348)).

- [ ] Extend the tool lifecycle scenario with reverse-order confirmations,
  deliberately delayed parallel results, cancellation, timeout recovery,
  elicitation decline and cancel, denial guidance, chat-mode advertisement,
  dynamic extension removal, per-tool permissions, audience-filtered output,
  and dangerous-command approval
  ([#5558](https://github.com/aaif-goose/goose/issues/5558),
  [#9461](https://github.com/aaif-goose/goose/issues/9461),
  [#1075](https://github.com/aaif-goose/goose/issues/1075),
  [#9024](https://github.com/aaif-goose/goose/issues/9024),
  [#1971](https://github.com/aaif-goose/goose/issues/1971),
  [#3818](https://github.com/aaif-goose/goose/issues/3818),
  [#5068](https://github.com/aaif-goose/goose/issues/5068),
  [#1858](https://github.com/aaif-goose/goose/issues/1858),
  [#1780](https://github.com/aaif-goose/goose/issues/1780),
  [#6703](https://github.com/aaif-goose/goose/issues/6703)).

- [ ] Extend the compaction lifecycle scenario across proactive, reactive, and
  `/compact` paths. Cover usage replacement, continued inference, repeated
  compaction and reload, provider failure after compaction, failed tool pairs,
  false context-error text, genuinely small models, and large plain and
  structured tool responses
  ([#6588](https://github.com/aaif-goose/goose/issues/6588),
  [#3538](https://github.com/aaif-goose/goose/issues/3538),
  [#4529](https://github.com/aaif-goose/goose/issues/4529),
  [#3779](https://github.com/aaif-goose/goose/issues/3779),
  [#5164](https://github.com/aaif-goose/goose/issues/5164),
  [#1102](https://github.com/aaif-goose/goose/issues/1102),
  [#3944](https://github.com/aaif-goose/goose/issues/3944),
  [#5255](https://github.com/aaif-goose/goose/issues/5255),
  [#6714](https://github.com/aaif-goose/goose/issues/6714),
  [#7027](https://github.com/aaif-goose/goose/issues/7027),
  [#7846](https://github.com/aaif-goose/goose/issues/7846)).

- [ ] Extend the steering lifecycle scenario with FIFO draining, steering
  during inference, cancellation, and compaction near the context limit
  ([#9037](https://github.com/aaif-goose/goose/issues/9037),
  [#1600](https://github.com/aaif-goose/goose/issues/1600),
  [#1513](https://github.com/aaif-goose/goose/issues/1513),
  [#1700](https://github.com/aaif-goose/goose/issues/1700),
  [#6579](https://github.com/aaif-goose/goose/issues/6579),
  [#8406](https://github.com/aaif-goose/goose/issues/8406)).

- [ ] Add an MCP prompt lifecycle scenario that preserves every alternating
  message returned by `/prompt`
  ([#6506](https://github.com/aaif-goose/goose/issues/6506)).

- [ ] Add one recipe and scheduling lifecycle scenario. Cover final-output tool
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

- [ ] Add a reconstruction and isolation scenario that creates a second
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

- [ ] Let `DummyApi` emit reasoning and mixed completion chunks and gate a
  response so cancellation and steering can happen while inference is active.

- [ ] Use the calculator's barrier and existing elicitation support to control
  completion order and cancellation without adding scripted command behavior.

- [ ] Let `test_pipeline` reconstruct a pipeline around an existing
  `SessionManager` and session ID.

- [ ] Add assertions for advertised tools, submitted system prompts,
  `trace_output`, and terminal usage without inspecting raw event fragments.

## API and Migration

- [ ] Settle the construction API. Callers should be able to provide an ordered
  set of steps and use `step`, `apply`, or `run` without depending on `Agent`.
  Keep construction of Goose's standard pipeline beside `Agent::reply`.

- [ ] Move or remove any remaining reply-entry behavior that prevents direct
  state-machine use. Session naming can remain an explicit out-of-band caller
  concern because it does not affect conversation state.

- [ ] Decide which operations and supporting types are public enough for
  callers to assemble a custom pipeline.

- [ ] Verify tracing records the final assistant output and that usage reaches
  the ACP and goosed terminal event after persistence and reload
  ([#8586](https://github.com/aaif-goose/goose/issues/8586),
  [#5604](https://github.com/aaif-goose/goose/issues/5604)).

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
