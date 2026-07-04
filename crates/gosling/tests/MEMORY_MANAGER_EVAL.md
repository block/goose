# Long-conversation memory evaluation: gosling Context Manager vs. base goose

This is a comparative evaluation of gosling's fork-specific "memory manager"
feature — internally named the **Context Manager**
(`GOSLING_CONTEXT_MANAGER=off|shadow|on`) — against base goose's only
mechanism for handling conversations that outgrow a model's context window:
LLM-driven compaction (`compact_messages` / `check_if_compaction_needed`,
present unchanged in both repos).

Harness: [`memory_manager_eval.rs`](./memory_manager_eval.rs) in this repo,
and its counterpart of the same name in the `goose` repo (baseline). Both
are `#[ignore]`d by default; run explicitly with:

```
cargo test -p gosling --test memory_manager_eval -- --ignored --nocapture
```

## Why mocked, not live-model

No model API keys are available in the environment this evaluation ran in.
Rather than skip the "million-token" requirement or fake a number, the
harnesses exercise the **real, unmodified production code** —
`ContextManager::build_with_counter`, `classify_blocks`, `FileMemorySource`,
`compact_messages`, `check_if_compaction_needed` — against synthetic
conversations sized by the real tiktoken-backed token counter, with only the
outer LLM call (compaction's summarization step, which only base goose's
mechanism needs) replaced by deterministic mock providers. gosling's
Context Manager needed no provider mocking at all for its core packet-build
path: classification, budget accounting, and file-backed memory recall are
all synchronous and provider-free by design.

Where base goose's recall depends on real summarizer quality (which cannot
be honestly simulated by one mock), the baseline harness reports two bounds
instead of a single fake number:

- **`generic_worst_case`** — a summarizer that always succeeds (within its
  own simulated context budget) but never preserves scenario-specific
  content. Lower bound on recall.
- **`faithful_best_case`** — a summarizer that echoes the needle fact
  verbatim into its summary whenever the fact is present in what it was
  asked to summarize. Upper bound on recall — a maximally competent,
  maximally faithful summarizer still cannot recall a fact it never saw.

Real LLM behavior falls somewhere between these bounds, non-deterministically,
run to run.

## Scenarios

Five long-running-agent scenarios, each with a "needle" fact stated early and
a final question that can only be answered correctly if that fact is still
recoverable:

| Scenario | Needle | Final question |
|---|---|---|
| `codebase_refactor_marathon` | New tables must use UUIDv7 primary keys; don't touch the payments module | What primary key type for a new table, and what to avoid? |
| `incident_response_marathon` | DNS already ruled out at 14:02 UTC; timeout config is 30000ms | Should we re-investigate DNS, and what's the timeout value? |
| `requirements_gathering_thread` | Must support iOS offline mode; SLA is 99.95% uptime | Summarize requirements for the client proposal |
| `research_literature_review` | Exclude pre-2020 papers; transformer-based only | Compile the shortlist per scope |
| `personal_planning_assistant` | Allergic to shellfish; budget cap $5,000 | What restaurant for the final night — does it fit? |

Each was built out to ~2.1M–3.6M real (tiktoken-counted) tokens of synthetic
filler per scenario (large tool-response bodies with genuinely unique
content — an early generator bug that produced near-duplicate content across
messages was caught and fixed; see commit history), then evaluated at three
context-window sizes representative of real models: **128K, 200K, 1M**
tokens. Two additional dimensions were varied to find real failure modes
rather than only the easiest case:

- **Needle position**: `Start` (near the beginning) vs. `Middle` (~50% through the filler).
- **Needle shape**: `IsolatedText` (its own plain-text user message) vs.
  `BuriedInToolOutput` (a sentence embedded inside a large tool-response body
  — the realistic shape of "a config value visible in a build log" or "a
  decision mentioned partway through a long file").

## Results

### gosling Context Manager (`on` mode — the packet actually used as provider input)

| Needle shape | Memory file seeded? | Recall (needle in final provider input) |
|---|---|---|
| IsolatedText, any position, any window (128K/200K/1M) | No | **15/15 (100%)** |
| IsolatedText, any position, any window | Yes | **15/15 (100%)** |
| BuriedInToolOutput | No | **0/1 (0%)** — dedicated test below |
| BuriedInToolOutput | Yes (`FileMemorySource`) | **1/1 (100%)** |

Why isolated-text recall is 100% regardless of window size or position:
`classify_blocks` groups conversation content into same-*slot* runs before
summarizing. A bare user-text message sandwiched between tool-heavy traffic
becomes its own small, isolated `OlderConversationSummary` run — and
`summarize_group`'s 600-character preview covers a 2-message run in full, so
nothing is lost. This is a genuinely useful, likely-unintentional emergent
property of the slot-based design.

Why buried-in-tool-output recall fails: the same 600-character preview is
taken from the *front* of the run's rendered text. A fact sitting in the
middle of a multi-thousand-token tool-response block, itself one of hundreds
in a multi-million-token `SummarizedToolResults` run, has effectively zero
chance of falling within the first 600 characters. Direct test:
`needle_buried_inside_tool_output_is_lost_without_memory_seed` —
`without_memory_seed=false, with_memory_seed=true` (i.e. lost without a
memory seed, recovered with one).

**Token budget does not scale with the model's context window.** `tokens_after`
in the built packet averaged **~1,411 tokens** and was statistically
identical across 128K, 200K, and 1M context limits (min 1,120 / max 1,902 in
all three cases). `RECENT_MESSAGE_WINDOW` (10 messages) and the "collapse
every same-slot Medium run into one summary" behavior are fixed constants,
not parameterized by `context_limit`. A 1M-context model gets handed the same
~1–2K-token packet as a 128K-context model — the extra ~800K tokens of
headroom the larger model could use for richer recent history goes entirely
unused by this mechanism.

**Performance & robustness** (all `#[ignore]`d stress tests pass):
- `build_with_counter` over a ~3.6M-token conversation: **388–1,093 ms**
  (release build), zero panics across 90 sweep runs + dedicated stress runs.
- A single 2M-token tool-output message: builds without panicking.
- A 20,000-line memory file (with malformed lines mixed in): recall stays
  bounded at ≤16 items (`MAX_RECALLED_ITEMS`) and completes in well under 5s.

### goose baseline (`compact_messages`, LLM-based, both repos share this code unchanged)

| Needle shape | `generic_worst_case` | `faithful_best_case` |
|---|---|---|
| IsolatedText (any position/window, 30 runs each) | **0/30 (0%)** | **30/30 (100%)** |
| BuriedInToolOutput (any position/window, 30 runs each) | **0/30 (0%)** | **0/30 (0%)** |

No errors, no failed compactions, one round always sufficed (the mock
compacts the entire conversation in a single LLM call, unlike gosling's
per-message classification).

The structural read: **whether an isolated-text fact survives baseline
compaction is 100% dependent on real summarizer quality, with zero
architectural guarantee either way** (0% to 100% depending entirely on which
mock stands in for the LLM). For facts buried inside tool output, **not even
a hypothetically perfect summarizer can help** — `do_compact`'s progressive
fallback (`removal_percentages = [0, 10, 20, 50, 100]`, stripping tool
responses "middle-out" when the compaction call itself doesn't fit the
summarizer's own context) discards tool-response content wholesale before
the summarizer ever sees it. A fact never presented to the model cannot be
in its summary, regardless of how good that model is.

## Head-to-head

| | Base goose (`compact_messages`) | gosling Context Manager (`on`) |
|---|---|---|
| Isolated-text fact, worst case | 0% (no guarantee) | **100%, deterministic, no LLM call needed** |
| Isolated-text fact, best case | 100% (requires a good summarizer) | 100% |
| Fact buried in tool output, no durable memory | 0% | 0% (same failure mode, different mechanism) |
| Fact buried in tool output, with durable memory | N/A — no equivalent built-in mechanism* | **100%**, via `FileMemorySource` |
| Recall cost as conversation scales | 1 LLM call per compaction (cost + latency + can itself fail on very large input) | Free — deterministic, provider-less, sub-second even at 3.6M tokens |
| Uses extra context window budget (200K→1M) for richer memory | N/A (compacts to threshold regardless) | **No** — fixed ~1.4K-token output regardless of window size |

\* Base goose does have a separate, pre-existing MCP "memory" extension
(`crates/gosling-mcp/src/memory`, `remember_memory`/`retrieve_memories` tools) —
but it requires the agent to proactively decide to call it during the
session; it's unrelated to and untouched by this fork's feature, and exists
identically in gosling too.

## Utility assessment

The Context Manager's actual value is **not** its built-in packet
summarization — that piece has essentially the same fundamental blind spot
as base goose's LLM-based compaction (content buried inside tool output is
gone by default, no guarantee for anything else) and additionally fails to
use extra window budget on larger-context models. Its concrete win is the
`FileMemorySource` seam: deterministic, keyword-ranked recall from a durable,
external, position-invariant store, at zero marginal LLM cost and no
observed scaling penalty even at 20,000 stored facts. That is a real
capability base goose has no equivalent of for automatic, cross-session
fact recall — **but nothing in the current codebase writes to that store**;
recall only helps once something (an operator, or a future agent-side write
path) actually populates `memories.jsonl`. As shipped and off by default,
enabling it today changes nothing for a user who hasn't also started curating
that file by hand.

None of the ~210 combined test runs across both repos panicked, hung, or
produced an unexpected error. The one intentionally-adversarial stress case
(a summarizer context budget too small to even attempt compaction) converges
via the existing progressive-removal fallback or fails with the expected,
descriptive error — never silently or destructively.

## Caveats

- Filler content is synthetic (deterministic, repetitive-but-unique text),
  not real code/logs/prose — real content's token density and repetition
  patterns will differ somewhat from these figures.
- The ~4-chars/token sizing heuristic used to *build* filler to a token
  target is approximate; all reported token counts in the results tables are
  from the real tiktoken-backed counter measuring what was actually built,
  not the sizing heuristic's estimate.
- Recall is checked via literal substring match, matching how a downstream
  model would need the exact fact text present in its input — this doesn't
  simulate whether a real LLM reading the final packet would *notice and use*
  a present fact, only whether it's structurally there to notice.
- `generic_worst_case` / `faithful_best_case` bound real LLM summarization
  quality; they don't reproduce it. Real-world recall for base goose's
  isolated-text case sits somewhere in [0%, 100%] and will vary run to run
  with a live model.
