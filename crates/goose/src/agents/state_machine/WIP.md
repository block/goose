# State Machine — Work in Progress

This module is the WIP unrolled agent loop, replacing the monolithic
`Agent::reply_internal`. It is gated behind the `GOOSE_STATE_MACHINE`
environment variable.

The thesis: **the conversation is the state.** Operations observe the
current `Session` and return declarative outcomes; the machine applies them.
Persistence, event emission, and orchestration live in the machine driver,
not in operations.

---

## Layout

```
state_machine/
├── mod.rs          # public surface: `reply` + `enabled` flag check
├── machine.rs      # the driver: assemble ops, run loop, apply outcomes
├── operation.rs    # Operation trait, Emitter, TurnOutcome
├── ops_llm.rs      # required inference runner; provider errors -> error messages
├── ops_toolcalling.rs  # execute tool requests, synthesize responses
├── ops_skills.rs   # advertise and load filesystem skills
├── ops_doctor.rs   # diagnose and repair session configuration
├── ops_maxturns.rs # halt after N assistant turns
├── ops_compaction.rs   # proactive + reactive (ContextLengthExceeded) auto-compact
├── ops_exit_on_error.rs # terminal: yield when the tail is an unrecovered error
├── test_helpers.rs # ScriptedProvider, TestExtensionClient, TestHarness
├── tests.rs        # scenario tests
└── WIP.md          # this file
```

---

## Current status

- [x] `GOOSE_STATE_MACHINE=1` flag dispatch from `Agent::reply`
- [x] `Operation` trait with `run(&Session, Emitter) -> OperationResult`
- [x] Required streaming `InferenceRunner`, after all optional operations
- [x] `ToolApprovalOperation` — annotates tool requests with approval decisions before execution
- [x] `ToolExecutionOperation` — bare execute-and-respond (no frontend/chat-mode)
- [x] `SkillOperation` — skill prompt, `load_skill`, `/skills`, and named skill commands
- [x] Cooperative tool handling — execution only claims tools it advertises; the fallback returns unknown calls to inference as tool errors
- [x] `MaxTurnsOperation` — halts the loop after `max_turns` assistant turns this request; the limit message is persisted so the transcript shows why the agent stopped (the old loop only yielded it)
- [x] `CompactionOperation` — proactive auto-compact before an LLM call (returns `ReplaceConversation`)
- [x] Machine driver applies ordered `TurnEffect`s
- [x] Usage recording — the LLM and compaction ops return `RecordUsage` effects. The machine enriches the usage, updates session totals, emits `Usage`/`MessageUsage` events for LLM turns, and attaches the ledger to the assistant message. Compaction usage resets the session total to the summary size so a replace cannot re-trigger on a stale count.
- [x] Stale orphaned tool requests (a crash mid-execution) — approval and execution only consider requests from the current request (at/after the last genuine user prompt); the LLM op strips older unanswered requests from the provider view. The transcript keeps them; nothing re-executes them.
- [x] Hooks — `SessionStart`/`UserPromptSubmit` at machine entry; `PreToolUse`/`PostToolUse` in `ToolExecutionOperation`; `Stop` as `StopHookOperation` (see backlog table)
- [x] Retry / goal / grind / final-output — `RetryOperation` between the LLM op and the stop hook (see backlog table)
- [x] Steering — `SteerOperation` injects mid-run user messages between turns (see backlog table)
- [x] LLM op provider fidelity — routes through `stream_response_from_provider` (toolshim, session-context scoping, thinking-effort default, error enhancement) and injects the moim turn-context block into the provider view
- [x] Unparseable tool calls — answered with the parse error so the model can correct (even in chat mode); the `Err` request stays in history, relying on the formatters' `Err` arms rather than the old loop's placeholder rewrite
- [x] Elicitation — a blocked tool's request flows through the existing action-required stream: the execution op persists it (mid-op, since the op stays blocked until the answer) and emits it; the response arrives via `Agent::reply`'s interception (shared with the old loop, so the state-machine dispatch sits below it) or directly from ACP, unblocking the tool through the `ActionRequiredManager` registry. The design-doc idea of yielding and re-entering doesn't fit here: the blocked tool call is a live future, so the reply stream stays open while the question is out.
- [x] Cancellation plumbed through the machine + `Emitter`
- [x] Observability — the stream runs inside the same `reply_stream` span as the old loop, with `trace_output` recorded from the final assistant text at exit
- [x] Non-blocking `Stop` hook at stream end for exits the blocking op didn't decide (see the Hooks backlog row)
- [x] `SlashCommandOperation` — runs over the persisted tail message and returns ordered effects
- [ ] More operations (see backlog below)
- [x] Errors as conversation state — provider errors become tagged, user-visible / agent-invisible messages (replacing the old fire-and-forget notification)
- [x] `ExitOnErrorOperation` — terminal catch-all; yields when the tail is an unrecovered error
- [x] Reactive compaction on `ProviderError::ContextLengthExceeded` (retry attempts are retained as agent-invisible errors and counted within the current kickoff)
- [ ] `UpdateSession(SessionUpdate)` outcome variant

---

## Shape

### `Operation`

```rust
#[async_trait]
pub trait Operation: Send + Sync {
    fn name(&self) -> &'static str;
    async fn inference_tools(&self, session: &Session) -> Result<Vec<Tool>>;
    async fn prompt_parts(
        &self,
        session: &Session,
        conversation: &Conversation,
    ) -> Result<Vec<(String, String)>>;
    async fn moim_parts(
        &self,
        session: &Session,
        conversation: &Conversation,
    ) -> Result<Vec<String>>;
    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult>;
}

pub trait Inference: Operation {
    async fn infer(
        &self,
        session: &Session,
        conversation: &Conversation,
        input: InferenceInput,
        emit: Emitter,
    ) -> Result<OperationResult>;
}

pub enum OperationResult {
    NotApplicable(Emitter),
    Applied(TurnOutcome),
}
```

Ops take `&Session` (read-only — the conversation IS the state) and an
`Emitter`. The `Emitter` is the op's handle to the machine: it carries a
sender for `AgentEvent`s the client should see in real time, and a
`CancellationToken`. Ops inspect the session and either return
`NotApplicable(Emitter)` without emitting, or stream 0+ events through the
emitter and return `Applied(TurnOutcome)`.

Long-running, streaming ops `select!` on `emit.cancelled()`. On cancel they
commit whatever they fully produced (via the normal `AppendMessages`) and
`break`; the machine's *between-ops* cancel check then ends the loop. Short
ops ignore cancellation entirely — they run to completion and the loop stops
on the next iteration. A cancelled `reply()` ends the stream cleanly (no
`Err`). There is no dedicated `Canceled` outcome: for now the caller doesn't
distinguish a cancelled stop from a normal one, so the between-ops check is
all we need. Add a distinct outcome later *if/when* we want to signal cancel
to the caller.

**Whatever an op commits must be a valid, self-consistent conversation tail.**
This is the op's responsibility, not the machine's and not a downstream repair
pass:

- The LLM op drops the in-flight chunk and commits whole chunks already
  emitted. A chat turn has no tool requests to pair, so this is trivially
  valid.
- The **tool-execution op**, on cancel, must synthesize a cancellation
  `ToolResponse` for every in-flight `ToolRequest` before committing, so the
  committed tail contains no orphaned request. The model then sees that those
  calls were interrupted, rather than the conversation being silently
  repaired (dropped) on the next read.

The old loop instead buffered everything in a local `messages_to_add` and
flushed it only when it believed the turn was consistent — but on the cancel
path it flushed the partial buffer anyway, so the invariant wasn't actually
enforced there; it leaned on `fix_conversation` dropping orphans at read time.
We make the persisted conversation the real state at all times and make each
op responsible for leaving it valid.

**The SessionManager is the single source of truth; the machine keeps no
in-memory copy.** Each loop iteration re-reads the session via
`get_session(id, true)` before selecting an op, and outcomes only *write* to
the SessionManager (`add_message` / `replace_conversation`) — they never patch
a local `Session`. This avoids reintroducing the `messages_to_add` failure
mode in a new shape: a hand-maintained mirror that can drift from disk. It
already would have drifted, because `add_message` assigns a message id when
one is missing, so a pushed-but-not-reloaded message differs from its
persisted form. The reload costs one small indexed DB read per turn —
negligible next to an LLM call — and guarantees every op sees exactly the
persisted state (ids assigned, stored order).

Construction-time dependencies are passed to the component that owns them.
Operations, including inference, may contribute tools, prompt inputs, and
MOIM text when the pipeline reaches its required inference step.
`PromptManager` composes the system prompt from those current inputs. The
pipeline then builds a fresh, immutable `InferenceInput`; it is not shared
between operations or retained across iterations.

### `TurnOutcome`

```rust
pub enum TurnEffect {
    AppendMessage(Message),
    ReplaceConversation(Conversation),
    PatchToolRequestMeta {
        message_id: String,
        tool_call_id: String,
        patch: serde_json::Value,
    },
    SetMessageVisibility {
        message_id: String,
        user_visible: bool,
        agent_visible: bool,
    },
    SetRecipe(Option<Recipe>),
    RecordUsage {
        usage: ProviderUsage,
        is_compaction: bool,
    },
    ResetContextUsage,
    YieldToClient,
}

pub type TurnOutcome = Vec<TurnEffect>;
```

The machine applies all effects in order via `SessionManager`. If it sees
`YieldToClient`, it stops selecting operations after applying the prior effects.
This lets one op perform a small transaction — for example "mark this message
invisible, append that response, then yield" — without relying on a two-pass
op dance. The machine does **not** auto-emit events for appended messages; ops
already streamed what they wanted visible.

### Machine driver

The driver (`machine::reply`) is the only place that:

- persists messages and conversations via `SessionManager`
- mutates `session` (push to conversation, replace, future field updates)
- runs the operation loop
- turns ops' emitted events into the client `AgentEvent` stream
- forwards `HistoryReplaced` on `ReplaceConversation`

Loop termination: either an applied op returns `YieldToClient`, or every
operation and the final inference step return `NotApplicable`.

---

## Two kinds of work

Not everything the machine does is an operation. There are two categories:

1. **Turns** — operations the loop runs *sequentially*: each reads the
   conversation and returns `Applied(TurnOutcome)` when it owns the current
   state. The LLM call, tool execution, compaction, etc. These are tried in
   order every iteration and are the substance of the loop.

2. **Out-of-band side effects** — concurrent, conversation-independent,
   fire-and-forget work triggered at reply *boundaries*. They run alongside
   the loop (`tokio::spawn`), never block it, produce no `TurnOutcome`, and
   their results reach the outside world via a side channel rather than the
   `AgentEvent` stream.

**Session naming** is the canonical out-of-band effect, spawned once at the
start of `reply` (mirroring the old loop): `maybe_update_name` generates a
title via the provider, persists it as a *session field* (not a message), and
publishes a `SessionNameUpdate` on `session_name_update_tx` for the UI. It is
deliberately *not* an operation:

- It must be **concurrent** with the first turn — title generation is itself
  an LLM call, and the whole point is to overlap it with the user's real
  response. An op would block the turn or spawn-inside-an-op (a spawn in a
  costume).
- Its output is **not a `TurnOutcome`** — it touches no message, only a
  session metadata field plus a UI side-channel.
- It is **once-per-reply**, not once-per-turn, so trying it every iteration is
  the wrong model and would need a "did I run" flag — the
  cross-iteration state we are trying to eliminate.

The boundary matters for what comes next: **tool-pair compaction** is a
*hybrid* — spawned concurrently like naming, but unlike naming its result
feeds back into the conversation (marks a request/response pair invisible,
inserts a summary). An out-of-band effect that mutates the very state the loop
reads is the hard case; see Open questions.

---

## Operations to port

Roughly in order of value, with the code in `agents/agent.rs` they replace:

| Operation | Replaces | Notes |
|---|---|---|
| **LLM** | `stream_response_from_provider` + the main `while let Some(next) = stream.next()` arms | **Landed.** Streams the response with the real `tools` list, so the model can emit `ToolRequest`s. Persists the assistant message (requests + thinking/reasoning) as-is. Constructor: `(Arc<dyn Provider>, system_prompt, tools)`. |
| **Tool approval** | `tool_inspection_manager.inspect_tools` + `process_inspection_results_with_permission_inspector` + `handle_approval_tool_requests` | **Landed.** Runs before execution, stores the decision in `ToolRequest.tool_meta`, emits `ActionRequired` and waits on the existing confirmation router when needed. The state lives on the request in the conversation, not in a side map. |
| **Tool execution** | `handle_approved_and_denied_tools` + `combined.next()` `tokio::select!` loop + frontend tool sub-flow | **Landed.** Applies when the last message is an assistant message with approved/denied extension tool requests. Dispatches approved requests through `ExtensionManager`, runs tool hooks, processes large responses, turns denied requests into declined `ToolResponse`s, drains streams forwarding `McpNotification`s, and collects responses into one user message. On cancel: cancels the dispatch token and synthesizes interrupted-tool responses so the committed tail is valid. Elicitation requests from blocked tools are persisted mid-op and emitted; the 100ms tick of the old drain loop was only a cancellation poll, covered here by `select!` on `emit.cancelled()`. Chat mode answers every pending request with the skipped notice instead of executing (and the approval op stands down). A successful `manage_extensions` call persists the extension state; tools are collected again whenever inference is reached. Final output and skills advertise and execute their own tools in their owning operations; scheduling is a normal platform extension. **Dropped deliberately:** frontend tools — nothing registers them anymore; they should be deleted from the old loop too. Unparseable tool calls are answered by the operation that owns the tool, or by the unknown-tool fallback. |
| **Compaction** | `check_if_compaction_needed` block + `ContextLengthExceeded` arm in `reply()` | **Landed (proactive + reactive).** Proactive: cheap synchronous ratio check (`session.total_tokens` vs model context limit, both captured at construction) when the last message is a pending user prompt. The operation also contributes the remaining-context MOIM part from the same threshold; without this operation, inference is not told that compaction exists. Reactive: when the tail is a `ContextLengthExceeded` error message (the LLM op appends one instead of bubbling), hide the error from the agent, compact, and retry. Earlier hidden context errors within the current kickoff provide the retry count, capped by `MAX_CONTEXT_ERROR_COMPACTIONS`. Records the summarization usage with the compaction flag, which resets the session total to the summary size so a replace can't re-trigger on a stale count. |

### Errors as conversation state

An error during a turn is now first-class **conversation** state, not a
state-machine invention: `MessageContent::Error(ErrorContent { kind, message })`
lives in `conversation/message.rs` with a typed `MessageErrorKind`
(`From<&ProviderError>`). `Message::from_provider_error` builds the user-facing
message (user-visible / agent-invisible); `Message::error_kind()` reads the kind
back. It's part of the OpenAPI surface, so the desktop renders it as a real
`error` message content.

The LLM op catches `ProviderError` (at stream creation and mid-stream), discards
any partial turn, and appends that error message instead of unwinding the
stream. Recovery ops dispatch on the kind: compaction reacts to
`ContextLengthExceeded`; everything else falls through to
`ExitOnErrorOperation` (last in the op list), which yields so the user can read
the error and retry with a new message. This replaces the old fire-and-forget
`yield notification; break`.
| **Tool-call pair compaction** | `crate::context_mgmt::maybe_summarize_tool_pairs` background task | **Landed (synchronous first cut, per plan).** An op after regular compaction: when enough agent-visible pairs sit past the cutoff, summarize a batch, mark each pair agent-invisible (`SetMessageVisibility`, transcript keeps them), append the agent-only summaries — they carry the replaced pair's created timestamp, so they sort into position on read. The trailing span of tool activity is protected (the old loop protected the current turn's calls). Revisit backgrounding if the summarization calls visibly delay turns — that's the "first-class background operations" open question. |
| **Elicitation** | `drain_elicitation_messages` + `ActionRequiredManager` calls | **Landed** (in the tool-execution op, not as its own op — see Current status). |
| **Max turns** | `if turns_taken > max_turns` block | Trivial. Counter is per-op or per-machine state (TBD when needed). |
| **Retry / goal / grind / final-output** | `handle_retry_logic` + `goal` / `grind` / `final_output` blocks | **Landed.** `RetryOperation` owns retry, goal, and grind. `RecipeOperation` reads the active recipe from the session, contributes its final-output prompt and tool, executes that tool, and turns the successful request/response pair in the conversation into the final assistant output. Recipe slash commands replace the session recipe through a turn effect. |
| **Subagent sync** | — | **Retired: nothing to port.** Subagents are tool calls through the `summon` platform extension — synchronous ones block, background ones are pulled via `load`/check-progress/cancel tools — so they already work through the execution op; no push channel into the loop exists in the old loop either. (`moim` was never subagent-related: it's the per-turn context block, now injected by the LLM op.) The design doc's vision — a completed background task *waking* the loop — is a future feature the re-entrant design makes cheap: re-enter via `reply()` with a synthetic message, like the ACP approval flow. |
| **Steering** | `drain_pending_steers` at loop top + retry/exit interactions | **Landed** as `SteerOperation` (after slash commands): applies between turns — completed assistant turn or finished tool exchange, never while requests are unanswered — drains the queue, fires `UserPromptSubmit` per steer, appends. Being genuine user-visible user messages, steers reset the max-turns budget (the old loop's cumulative counter did not — deliberate: new user input, new budget). The retry op needs no pending-steers check: the steer op precedes it and consumes the state. |
| **Hooks** | scattered `hook_manager.emit(...)` and `emit_blocking(...)` calls | **Landed.** The "cross-cutting exception" turned out unnecessary — each hook found a home at its own granularity: `SessionStart`/`UserPromptSubmit` fire at machine entry (once per reply, so not ops — a non-mutating op would re-apply forever); `PreToolUse`/`PostToolUse` and the shell/file variants live in `ToolExecutionOperation`, per tool call; `Stop` is an ordinary `StopHookOperation` at the end of the list — it applies when the tail is a completed assistant turn, and a denial just appends the denial-context user message, which re-arms the LLM op. The consecutive-block cap is a per-reply counter on the op (the denial messages are user-role and agent-visible, so a conversation walk can't count them). Exits the blocking op didn't decide — max turns, approval waits, errors, cancellation — still notify Stop hooks via the machine's non-blocking tail emit, mirroring the old loop's `stop_hook_handled_for_exit`. (An earlier note here claimed the old loop consulted the *blocking* hook on the max-turns exit; it doesn't — that consult is the final-output exit, which the retry-op → stop-op sequence covers.) **Divergence:** in the old loop a stop-hook-denial retry didn't count against the max-turns budget; here it does — the denial context is indistinguishable from other machine-generated nudges in the conversation walk. Conservative: denials burn budget faster, never loop longer. |
| **Slash commands** | `execute_command` block in `reply()` | **Landed** as `SlashCommandOperation`. Commands are offered to each operation and then inference; `/status` is owned by inference. The state machine no longer calls `Agent::execute_command`. |
| **Refresh tools after `manage_extensions`** | `tools_updated` block | Either a tail-step of the Tool execution op or a separate op. |

---

## Open questions

- **Emit as the single channel; machine collects (don't return messages).**
  Today an op both emits a message to the client *and* returns it in
  `AppendMessages`, and the machine persists the returned payload. That's two
  places to keep in sync and lets "shown to client" drift from "persisted".
  Cleaner: ops **always emit** messages (LLM deltas, MCP notifications, each
  tool response *as it lands*), never return them; the **machine collects the
  emitted messages and persists them**. One path, machine is the sole
  persister, identical by construction. Bonus: with two tool calls running
  concurrently, the current op holds both results and emits one merged message
  only when *both* finish — emit-as-you-go shows each result the moment it
  lands. Wrinkles to handle:
  - LLM op emits streaming deltas *and* a final coalesced message; the machine
    must persist only the final (coalesce by message id, like
    `Conversation::push` already does) — confirm the `with_id` path covers it.
  - Tool responses become N emits (per-id) instead of one merged user message;
    the machine coalesces by id. Slightly changes the persisted shape — be
    deliberate.
  - `YieldToClient` / `ReplaceConversation` stay as return outcomes (control
    flow / whole-conversation), so `TurnOutcome` keeps those — only
    `AppendMessages` dissolves into "the machine collected what you emitted".
  Not now (touches machine + LLM op + tool op together); current code is
  correct, just batches tool results suboptimally.
- **Where do turn counters live?** Today there are none. When the max-turns
  op lands, it needs to count turns across loop iterations. Options: pass a
  mutable counter into the op constructor (`Arc<AtomicU32>`), or reintroduce
  a thin `TurnState { session, counters }` wrapper. Defer until needed.
- **System prompt rebuild policy** — resolved: the pipeline asks the prompt
  manager to build the prompt whenever inference is reached. Identical inputs
  produce the same prompt, while extension, mode, or project-hint changes are
  visible on the next inference call. The inference runner keeps no prompt or
  tool cache.
- **Persistence granularity.** Per-outcome (write after each append) — same
  as today's behaviour. Fine.
- **First-class background operations.** Right now out-of-band work (session
  naming) is a raw `tokio::spawn` that floats free of the machine — the loop
  has no handle on it, can't cancel it, and can't wait for it. A cleaner
  model: let an op return `TurnOutcome::RunningInBackground(JoinHandle<...>)`.
  The machine keeps these handles in a set, continues the loop immediately,
  and at termination (`YieldToClient` / no-op-applied / cancel) either
  **awaits** them (work that must finish, e.g. flush a summary) or
  **aborts** them (cancel). This brings background work back under the
  machine's lifecycle: cancellation cleanup becomes uniform (the machine
  aborts the set, ops don't each hand-roll `.abort()`), and shutdown is
  deterministic instead of detached-and-hope.
  Open sub-questions before adopting it:
  - **Result feedback.** Naming's result goes to a UI side-channel and never
    re-enters the loop — fine to fire-and-forget. But **tool-pair
    compaction's** result *mutates the conversation* (marks a pair invisible,
    inserts a summary). If it completes mid-loop, does its write land via the
    SessionManager (and get picked up by the next iteration's reload), or does
    the machine need to *join* it at a turn boundary and apply a
    `ReplaceConversation`-like outcome so ordering is deterministic? The
    former keeps the "single source of truth, reload each iteration" model but
    makes the conversation mutate underneath an in-flight turn; the latter is
    ordered but reintroduces a join point.
  - **Await-vs-abort policy.** Per-handle (naming = abort-ok, compaction =
    must-finish) or a flag on the variant.
  - Until this is designed, naming stays a plain spawn and the tool-execution
    op owns its summarization `JoinHandle` (see Cancellation cleanup).
- **Cancellation cleanup.** Resolved for the LLM op (drop in-flight chunk,
  commit whole chunks). The tool-execution op, on cancel, must (1) synthesize
  a cancellation `ToolResponse` for each in-flight `ToolRequest` so its
  `committed` tail is valid, and (2) `.abort()` any background work it owns
  (e.g. the tool-pair summarization `JoinHandle`) before returning. The
  machine doesn't yank a running op — it relies on the op observing
  `emit.cancelled()` and cleaning up itself.

---

## Migration steps remaining

1. Add ops in the order in the backlog table.
2. Fold `reply()` entry-point logic in (elicitation response, slash
   commands, `UserPromptSubmit` hook, pre-turn auto-compact) as
   first-turn-only ops.
3. Tests: scenario tests driven by a scripted provider. Because ops are
   independently constructable and the machine just sequences them, each op
   can be instrumented on its own and in combination with others. Build these
   out once there's enough op surface to be worthwhile — not a differential
   oracle against the old loop. Tests call `state_machine::reply` directly
   (parallel-safe, no env var). A scripted provider returns canned
   responses/tool-requests per call so a scenario can drive LLM → tool
   execution → LLM, etc.
4. Flip the flag default after a release with no regressions.
5. Delete `reply_internal` and friends.
6. Public API for swapping the pipeline (`AgentConfig::operations`,
   dynamic insert/remove).
