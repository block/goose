# State-machine regression backlog

This backlog comes from a review of all 2,349 closed issues in
`aaif-goose/goose` on 2026-07-25. Ten reviewers each screened a disjoint tenth
of the issues, inspected relevant fixes, and compared them with the 43 current
state-machine integration tests.

The goal is not to reproduce every old test. Each item below protects a real
failure that can still occur at the state-machine boundary.

## First: likely current regressions

- [ ] Empty provider responses must not silently end a turn
  ([#10353](https://github.com/aaif-goose/goose/issues/10353),
  [#6470](https://github.com/aaif-goose/goose/issues/6470),
  [#1170](https://github.com/aaif-goose/goose/issues/1170)).
  `InferenceRunner` currently returns `NotApplicable` when the accumulated
  response is empty. Return a visible bounded error, persist no empty assistant
  message, and allow the next user turn to succeed.

- [ ] Duplicate tool-call IDs execute at most once
  ([#9786](https://github.com/aaif-goose/goose/issues/9786),
  [#9756](https://github.com/aaif-goose/goose/issues/9756)).
  The legacy path deduplicates IDs before dispatch. Emit two `calculator__add`
  calls with the same ID and assert one persisted request, one response, and
  one calculator call.

- [ ] Tool-pair compaction reaches the client
  ([#7413](https://github.com/aaif-goose/goose/issues/7413)).
  Visibility changes and summaries are persisted, but currently produce no
  `HistoryReplaced` event. Assert that a stale client cannot restore the
  pre-compaction history on its next round trip.

- [ ] Nested `AGENTS.md` and `.goosehints` files follow accessed paths
  ([#5840](https://github.com/aaif-goose/goose/issues/5840),
  [#4336](https://github.com/aaif-goose/goose/issues/4336)).
  Scan path-bearing arguments from prior tool calls, load matching nested
  instructions, and assert the following provider system prompt contains them.
  This is still a TODO in `ToolExecutionOperation`.

- [ ] Unknown-tool errors include callable alternatives
  ([#8166](https://github.com/aaif-goose/goose/issues/8166)).
  Extend the existing unknown-tool test to require `calculator__add` in the
  error returned to the model.

- [ ] Preserve streamed reasoning through a tool call
  ([#9675](https://github.com/aaif-goose/goose/issues/9675)).
  Stream reasoning chunks followed by a tool request. Assert one complete
  reasoning sequence remains attached to the persisted request and appears
  exactly once in the next provider request.

## Fix the test pipeline

- [ ] Compute the tool-pair cutoff the same way as production
  ([#7415](https://github.com/aaif-goose/goose/issues/7415)).
  `test_pipeline()` currently hardcodes `10`; production uses
  `compute_tool_call_cutoff(context_limit, compaction_threshold)`. Test both
  sides of the calculated cutoff.

- [ ] Make final-output advertisement part of the existing recipe test
  ([#3700](https://github.com/aaif-goose/goose/issues/3700)).
  The dummy API can currently request an unadvertised tool. Assert the first
  API call advertises `recipe__final_output`.

- [ ] Expose the submitted system prompt on `ApiCall`
  ([#2431](https://github.com/aaif-goose/goose/issues/2431),
  [#4610](https://github.com/aaif-goose/goose/issues/4610)).
  Two calls more than a second apart must submit identical system prompts when
  nothing relevant changed.

- [ ] Support successful responses without a usage event
  ([#899](https://github.com/aaif-goose/goose/issues/899),
  [#1159](https://github.com/aaif-goose/goose/issues/1159)).
  Reply normally, omit usage, then complete another user turn.

- [ ] Support an HTTP 200 response with no choices or messages
  ([#6470](https://github.com/aaif-goose/goose/issues/6470)).

- [ ] Support an empty-body HTTP 500
  ([#5528](https://github.com/aaif-goose/goose/issues/5528)).
  The persisted error should contain the status and a useful missing-body
  explanation, and the next turn should recover.

- [ ] Support reasoning chunks and mixed text/tool completions
  ([#9675](https://github.com/aaif-goose/goose/issues/9675),
  [#7425](https://github.com/aaif-goose/goose/issues/7425)).

## Multiple tool calls

Add one API response form that emits multiple tool calls with configurable IDs.
Use it for all of these rather than adding issue-specific response helpers.

- [ ] Pair every parallel request with exactly one response
  ([#1367](https://github.com/aaif-goose/goose/issues/1367),
  [#5957](https://github.com/aaif-goose/goose/issues/5957),
  [#5997](https://github.com/aaif-goose/goose/issues/5997)).

- [ ] Calls to one extension execute concurrently
  ([#7201](https://github.com/aaif-goose/goose/issues/7201)).
  Two calculator calls should meet at a barrier instead of deadlocking behind
  one client lock.

- [ ] Reverse-order confirmations do not lose either response
  ([#5558](https://github.com/aaif-goose/goose/issues/5558)).

- [ ] Slow tool responses remain ordered after their requests
  ([#9461](https://github.com/aaif-goose/goose/issues/9461)).
  Cross a timestamp second and assert request time is not after response time.

- [ ] Text followed by malformed tool arguments produces a matching parse-error
  response and a valid next provider call
  ([#7425](https://github.com/aaif-goose/goose/issues/7425)).

- [ ] Repeated malformed calls remain bounded
  ([#7527](https://github.com/aaif-goose/goose/issues/7527)).

## Cancellation and steering

- [ ] Cancelling an in-flight tool persists an interrupted response with the
  matching request ID, then a fresh user turn succeeds
  ([#1541](https://github.com/aaif-goose/goose/issues/1541),
  [#1624](https://github.com/aaif-goose/goose/issues/1624),
  [#2337](https://github.com/aaif-goose/goose/issues/2337),
  [#2947](https://github.com/aaif-goose/goose/issues/2947),
  [#4252](https://github.com/aaif-goose/goose/issues/4252)).
  The existing elicitation tool may be sufficient to hold execution open.

- [ ] Work completed before cancellation remains persisted
  ([#7827](https://github.com/aaif-goose/goose/issues/7827)).
  Complete one tool, block a second, cancel, and verify both the completed
  result and interrupted response.

- [ ] A tool timeout answers its request and does not poison later turns
  ([#1075](https://github.com/aaif-goose/goose/issues/1075)).

- [ ] A queued steer survives cancellation
  ([#9037](https://github.com/aaif-goose/goose/issues/9037),
  [#1600](https://github.com/aaif-goose/goose/issues/1600)).

- [ ] A steer submitted during active inference is applied at the next valid
  boundary
  ([#1513](https://github.com/aaif-goose/goose/issues/1513),
  [#1700](https://github.com/aaif-goose/goose/issues/1700)).
  The dummy API needs a gated response.

- [ ] Two queued steers drain in FIFO order and leave the queue empty
  ([#6579](https://github.com/aaif-goose/goose/issues/6579)).

- [ ] Steering near the context limit survives compaction
  ([#8406](https://github.com/aaif-goose/goose/issues/8406)).

## Compaction

- [ ] `/compact` replaces usage immediately, then another user turn runs
  without automatic recompaction
  ([#6588](https://github.com/aaif-goose/goose/issues/6588),
  [#3538](https://github.com/aaif-goose/goose/issues/3538)).

- [ ] Compact twice, reload, and preserve the complete user-visible transcript
  while summaries and continuation messages remain agent-only
  ([#4529](https://github.com/aaif-goose/goose/issues/4529),
  [#3779](https://github.com/aaif-goose/goose/issues/3779)).

- [ ] A provider error immediately after proactive compaction does not restore
  stale usage or cause another compaction on the next user turn
  ([#5164](https://github.com/aaif-goose/goose/issues/5164)).

- [ ] A failed tool pair remains valid through compaction
  ([#1102](https://github.com/aaif-goose/goose/issues/1102)).

- [ ] A non-context server error containing `too long` does not compact
  ([#3944](https://github.com/aaif-goose/goose/issues/3944)).

- [ ] A genuinely small model rejects both inference and compaction without an
  infinite recovery loop
  ([#5255](https://github.com/aaif-goose/goose/issues/5255)).

- [ ] Large tool responses are externalized before persistence and inference
  ([#6714](https://github.com/aaif-goose/goose/issues/6714),
  [#7027](https://github.com/aaif-goose/goose/issues/7027)).
  Add one real calculator operation that deterministically produces a large
  result. Assert the stored file contains the original and the conversation
  contains only the replacement notice.

- [ ] Large structured tool output keeps its structured result and separate
  truncation notice
  ([#7846](https://github.com/aaif-goose/goose/issues/7846)).

## Tool errors and permissions

- [ ] Empty streamed tool arguments normalize to `{}`, produce a useful tool
  error, recover, and do not poison a later user turn
  ([#1108](https://github.com/aaif-goose/goose/issues/1108),
  [#1068](https://github.com/aaif-goose/goose/issues/1068)).

- [ ] MCP runtime error text reaches the model unchanged
  ([#6189](https://github.com/aaif-goose/goose/issues/6189)).
  Add a genuine calculator `fail` operation.

- [ ] Elicitation decline and cancel both complete the waiting tool and allow
  inference to continue
  ([#9024](https://github.com/aaif-goose/goose/issues/9024)).

- [ ] A wire-level MCP `JsonRpcError` completes the tool request instead of
  timing out
  ([#2884](https://github.com/aaif-goose/goose/issues/2884)).
  This needs a small in-process MCP transport fixture.

- [ ] A denial tells the model not to retry and does not execute the tool
  ([#1971](https://github.com/aaif-goose/goose/issues/1971)).
  Assert the historically important text literally rather than matching the
  same shared constant on both sides of the test.

- [ ] Chat mode advertises no model-callable tools
  ([#3818](https://github.com/aaif-goose/goose/issues/3818)).

- [ ] Disabling a platform extension removes its tools from the next inference
  request
  ([#5068](https://github.com/aaif-goose/goose/issues/5068)).

- [ ] App-only tools are not advertised to inference
  ([#7467](https://github.com/aaif-goose/goose/issues/7467)).

- [ ] Per-tool permission rules affect only the intended tools
  ([#1858](https://github.com/aaif-goose/goose/issues/1858)).

- [ ] Dangerous developer commands still require confirmation in Auto mode
  ([#1780](https://github.com/aaif-goose/goose/issues/1780)).

- [ ] Audience-filtered tool output reaches the model once, excludes user-only
  content, and remains correct in persisted history
  ([#6703](https://github.com/aaif-goose/goose/issues/6703)).

- [ ] A tool with an unusual but valid schema cannot panic token counting
  ([#4879](https://github.com/aaif-goose/goose/issues/4879)).

## Skills, prompts, and extensions

- [ ] Root `AGENTS.md` reaches inference without a developer extension
  ([#5104](https://github.com/aaif-goose/goose/issues/5104),
  [#1800](https://github.com/aaif-goose/goose/issues/1800)).

- [ ] A skill created after the first turn is discovered without rebuilding the
  pipeline
  ([#6382](https://github.com/aaif-goose/goose/issues/6382)).

- [ ] `/skills` lists a newly created project skill without calling inference
  ([#8599](https://github.com/aaif-goose/goose/issues/8599)).

- [ ] Loading a skill resolves supporting scripts relative to the skill
  directory
  ([#9558](https://github.com/aaif-goose/goose/issues/9558)).

- [ ] Enabling an extension persists across pipeline reconstruction
  ([#4295](https://github.com/aaif-goose/goose/issues/4295)).

- [ ] Multi-message MCP prompts persist every alternating message in order
  ([#6506](https://github.com/aaif-goose/goose/issues/6506)).

- [ ] Resource reads require explicit extension ownership and preserve errors
  ([#8988](https://github.com/aaif-goose/goose/issues/8988)).

- [ ] Runtime MCP extensions adopt their server-declared names
  ([#6188](https://github.com/aaif-goose/goose/issues/6188)).

## Recipes, scheduling, and subagents

- [ ] An invalid final-output schema cannot panic or kill the session
  ([#10491](https://github.com/aaif-goose/goose/issues/10491)).

- [ ] A recipe slash command accepts an omitted optional parameter and applies
  its default
  ([#6232](https://github.com/aaif-goose/goose/issues/6232)).

- [ ] Active recipes advertise their generated tools and intended extension
  tools
  ([#3730](https://github.com/aaif-goose/goose/issues/3730),
  [#7353](https://github.com/aaif-goose/goose/issues/7353)).

- [ ] Parent parameters are resolved in subrecipes
  ([#6078](https://github.com/aaif-goose/goose/issues/6078)).

- [ ] Recipe and delegated-task `max_turns` overrides reach the child pipeline
  ([#6198](https://github.com/aaif-goose/goose/issues/6198)).

- [ ] A background scheduled recipe runs in Auto mode and loads its declared
  tools
  ([#3882](https://github.com/aaif-goose/goose/issues/3882),
  [#6023](https://github.com/aaif-goose/goose/issues/6023)).

- [ ] The scheduler tool is advertised only when a scheduler is available
  ([#6405](https://github.com/aaif-goose/goose/issues/6405)).

- [ ] An incomplete scheduled recipe returns a tool error instead of panicking
  ([#7431](https://github.com/aaif-goose/goose/issues/7431)).

- [ ] Scheduled session summaries use persisted message counts
  ([#10016](https://github.com/aaif-goose/goose/issues/10016)).

- [ ] Summon/delegate tools are not advertised outside the modes that support
  them
  ([#5140](https://github.com/aaif-goose/goose/issues/5140)).

## Session reconstruction and isolation

These need a fixture that can build a second pipeline over an existing
`SessionManager` and session ID.

- [ ] Persisted Goose mode controls approval after reconstruction
  ([#7603](https://github.com/aaif-goose/goose/issues/7603)).

- [ ] Provider, model, extension, and recipe state survive reconstruction
  ([#7615](https://github.com/aaif-goose/goose/issues/7615),
  [#5358](https://github.com/aaif-goose/goose/issues/5358)).

- [ ] Cost remains accumulated under the model that produced each usage record
  after a model change
  ([#6141](https://github.com/aaif-goose/goose/issues/6141)).

- [ ] Configured context limits and `GOOSE_MAX_TURNS` reach the production
  pipeline constructor
  ([#7839](https://github.com/aaif-goose/goose/issues/7839),
  [#7609](https://github.com/aaif-goose/goose/issues/7609)).

- [ ] Two sessions sharing an extension manager retain distinct working
  directories
  ([#6909](https://github.com/aaif-goose/goose/issues/6909)).

- [ ] Concurrent elicitations remain correlated by session and tool-call ID
  ([#9870](https://github.com/aaif-goose/goose/issues/9870)).

- [ ] Cache-read and cache-write usage fields survive persistence
  ([#4988](https://github.com/aaif-goose/goose/issues/4988)).

- [ ] Tools receive the correct session ID
  ([#6308](https://github.com/aaif-goose/goose/issues/6308)).

## Observability and provider-view checks

- [ ] Fragmented text streaming persists exactly one assistant message
  ([#5576](https://github.com/aaif-goose/goose/issues/5576)).

- [ ] `trace_output` contains the final assistant text
  ([#8586](https://github.com/aaif-goose/goose/issues/8586)).

- [ ] GPT-4.1 receives its model-specific prompt
  ([#3348](https://github.com/aaif-goose/goose/issues/3348)).

- [ ] Session usage reaches the ACP/goosed terminal event and survives reload
  ([#5604](https://github.com/aaif-goose/goose/issues/5604)).

## Historical failures already covered

Keep these issue links near the tests they justify. No new scenario is needed
unless the implementation boundary changes.

- Endless tool loops and max turns:
  [#1037](https://github.com/aaif-goose/goose/issues/1037),
  [#2657](https://github.com/aaif-goose/goose/issues/2657),
  [#5155](https://github.com/aaif-goose/goose/issues/5155).
- Tool errors and later-turn recovery:
  [#1224](https://github.com/aaif-goose/goose/issues/1224),
  [#3350](https://github.com/aaif-goose/goose/issues/3350),
  [#5213](https://github.com/aaif-goose/goose/issues/5213).
- Malformed tool arguments:
  [#4513](https://github.com/aaif-goose/goose/issues/4513).
- Orphaned historical tool requests:
  [#33](https://github.com/aaif-goose/goose/issues/33),
  [#2951](https://github.com/aaif-goose/goose/issues/2951).
- Valid request/result pairing:
  [#856](https://github.com/aaif-goose/goose/issues/856),
  [#1056](https://github.com/aaif-goose/goose/issues/1056).
- Context recovery and bounded retries:
  [#903](https://github.com/aaif-goose/goose/issues/903),
  [#1096](https://github.com/aaif-goose/goose/issues/1096),
  [#1303](https://github.com/aaif-goose/goose/issues/1303),
  [#2368](https://github.com/aaif-goose/goose/issues/2368),
  [#2827](https://github.com/aaif-goose/goose/issues/2827),
  [#3336](https://github.com/aaif-goose/goose/issues/3336),
  [#4263](https://github.com/aaif-goose/goose/issues/4263).
- Compaction usage replacement:
  [#4635](https://github.com/aaif-goose/goose/issues/4635),
  [#5162](https://github.com/aaif-goose/goose/issues/5162).
- `/clear` history and usage:
  [#2960](https://github.com/aaif-goose/goose/issues/2960),
  [#3138](https://github.com/aaif-goose/goose/issues/3138),
  [#5651](https://github.com/aaif-goose/goose/issues/5651).
- Stream usage and mid-stream errors:
  [#5907](https://github.com/aaif-goose/goose/issues/5907),
  [#8021](https://github.com/aaif-goose/goose/issues/8021),
  [#8859](https://github.com/aaif-goose/goose/issues/8859).
- Elicitation:
  [#6471](https://github.com/aaif-goose/goose/issues/6471),
  [#7841](https://github.com/aaif-goose/goose/issues/7841),
  [#8531](https://github.com/aaif-goose/goose/issues/8531),
  [#9031](https://github.com/aaif-goose/goose/issues/9031).
- Hooks:
  [#9068](https://github.com/aaif-goose/goose/issues/9068),
  [#9277](https://github.com/aaif-goose/goose/issues/9277).
- Skill loading from the session working directory:
  [#7853](https://github.com/aaif-goose/goose/issues/7853).
- Queued steering:
  [#8176](https://github.com/aaif-goose/goose/issues/8176).
- Tool-pair persistence through an injected `SessionManager`:
  [#10624](https://github.com/aaif-goose/goose/issues/10624).

## Suggested order

1. Fix the tool-pair cutoff in `test_pipeline()`.
2. Add empty-response recovery.
3. Add multiple tool-call responses and duplicate-ID coverage.
4. Add cancellation and next-turn recovery.
5. Add `/compact` usage and repeated-compaction reload tests.
6. Add missing-usage and empty-argument recovery.
7. Add tool-pair client synchronization.
8. Implement and test nested `AGENTS.md` discovery.
9. Add the remaining tests that need no new fixture support.
10. Add reconstruction, scheduler, MCP transport, and multi-session fixtures only
    when work reaches those boundaries.
