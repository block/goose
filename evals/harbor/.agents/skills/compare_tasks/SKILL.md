---
name: compare_tasks
description: Compare how two harbor benchmark runs performed on a single shared task
---

# Compare two harbor runs on one task

Use when given two harbor run names and a task name, and the goal is to understand
*why* the two runs differ on that task — not just *that* they differ.

## Inputs

- `RUN_A`: harbor run name (e.g. `sonnet46-full`)
- `RUN_B`: harbor run name (e.g. `pi-sonnet46-full`)
- `TASK`: bare task name (e.g. `extract-elf`, not `terminal-bench/extract-elf`)
- `RUNS_DIR`: defaults to `evals/harbor/runs/` relative to the repo root

## Procedure

### 1. Find each run's trial directory for the task

Harbor 0.8 names trial dirs `<task>__<random-suffix>` (e.g.
`extract-elf__bU3GHs4`), **not** `<task>.1`. The suffix is unique per trial,
so don't guess it — discover it from disk:

```bash
TRIAL_A_DIR=$(ls -d "$RUNS_DIR/$RUN_A/${TASK}__"*/ 2>/dev/null | head -1)
TRIAL_B_DIR=$(ls -d "$RUNS_DIR/$RUN_B/${TASK}__"*/ 2>/dev/null | head -1)
```

If either is empty, that run didn't include this task — stop and say so.
(`ls "$RUNS_DIR/$RUN_A/"` shows what's there.)

If you want to confirm the match, every `result.json` carries `task_name`
and `trial_name`:

```bash
jq '{task_name, trial_name}' "$TRIAL_A_DIR/result.json"
```

### 2. Headline facts

Pull these fields from each trial's `result.json`. The actual shape (harbor
0.8 `TrialResult`):

```bash
jq '{
  reward: (.verifier_result.rewards.reward // null),
  rewards_all: .verifier_result.rewards,
  duration_seconds: ((.finished_at | fromdateiso8601) - (.started_at | fromdateiso8601)),
  input_tokens: .agent_result.n_input_tokens,
  cache_tokens: .agent_result.n_cache_tokens,
  output_tokens: .agent_result.n_output_tokens,
  cost_usd: .agent_result.cost_usd,
  error_type: .exception_info.exception_type,
  error_message: (.exception_info.exception_message // "" | split("\n")[0])
}' "$TRIAL_A_DIR/result.json"
```

Derive status from those:

- `pass` if `reward >= 1.0`
- `partial` if `reward > 0` (and < 1)
- `fail` if `reward == 0`
- `timeout` if reward is 0/null **and** `error_type` contains "timeout"
- `error` if reward is 0/null **and** `error_type` is set (non-timeout)
- `no-reward` if neither `verifier_result.rewards` nor `exception_info` is set

Reward wins over errors: harbor can record an `AgentTimeoutError` *after* the
verifier already scored a pass (the agent finished the work then the harness
timed out during teardown, or it timed out after writing the correct answer).
If we got points, count them. See `reporter.trial_status` for the canonical
rule.

Several `agent_result` fields are commonly `null` for older `GooseBinaryAgent`
runs (notably `n_cache_tokens`, `n_output_tokens`, `cost_usd`). Don't treat
that as a failure — just omit those facts from the comparison if missing on
either side. The reporter has fallbacks that read goose's `complete` event
from `agent/goose.txt`; you don't normally need to replicate them here.

### 3. Read the task spec

The task definitions are NOT in the harbor Python package. They are plain
text files on disk, in harbor's dataset cache. Do not run `find /` or
`pip show harbor` — that is the wrong direction.

Find the task directory (works on Linux and macOS):

```bash
TASK_DIR=$(
  ls -d ~/.cache/harbor/datasets/terminal-bench__terminal-bench-2__*/tasks/"$TASK"/ 2>/dev/null \
  || ls -d ~/Library/Caches/harbor/datasets/terminal-bench__terminal-bench-2__*/tasks/"$TASK"/ 2>/dev/null
)
echo "$TASK_DIR"
ls "$TASK_DIR"
```

If both lookups return empty, the dataset hasn't been downloaded yet — bail
out and report that, rather than guessing.

Inside, you care about three files:

- `instruction.md` — exactly what the agent was asked to do
- `tests/test_outputs.py` (or sometimes `run-tests.sh`) — what the verifier
  actually checks, line by line
- `solution/solution.sh` — the reference correct answer

Without all three you can't tell whether a wrong answer was a misread, a
shallow bug, or a verifier surprise. **Quote the assertion that failed**
when you describe a failure — paraphrasing is how wrong conclusions sneak in.

### 4. Read each agent's trajectory

Two sources, prefer the first when present:

- `$TRIAL_DIR/agent/trajectory.json` — harbor's ATIF format, one entry per
  agent step. `jq '.steps[] | {step_id, source, message, tool_calls: [.tool_calls[]?.function_name]}'`
  gives a compact view. Recent goose runs (after the populate_context_post_run
  fix) have this; older `GooseBinaryAgent` runs may not.
- `$TRIAL_DIR/agent/<harness>.txt` — raw stream-json or log. The filename
  matches the harness: `goose.txt`, `pi.txt`, `opencode.txt`,
  `claude-code.txt`. `ls "$TRIAL_DIR/agent/"` to find it.

Skim, don't quote in full. For each agent identify:

- the approach it took (e.g. "wrote a Python script that walks the ELF section
  headers")
- the final artifacts it left in the container (file paths it created or
  modified)
- for losers, the **failure mode** — one of:
  - misread the spec (wrong assumption about input/output)
  - right approach, shallow bug (off-by-one, wrong encoding, wrong base address)
  - ran out of clock (timeout) — note whether it was still making progress or
    had gone in circles
  - diverged into an unproductive thread (e.g. debugging a non-issue)
  - the verifier expected something the spec didn't telegraph

### 5. Read the verifier output

`$TRIAL_DIR/verifier/` typically contains:

- `test-stdout.txt` — the verifier's full stdout (assertion failures, pytest
  output, etc.). This is usually the most diagnostic file.
- `reward.txt` — the scalar reward as a string.
- `ctrf.json` — structured test results in CTRF format, useful if you want
  per-assertion pass/fail without grepping stdout.

```bash
tail -50 "$TRIAL_DIR/verifier/test-stdout.txt"
```

This is often more diagnostic than the agent log — it tells you exactly which
assertion failed and what the agent's output was at that point.

### 6. Produce the comparison

Output markdown with these sections in order:

- **Headline** (1 line): who won, by how much (reward + cost / duration if
  meaningful, omitting fields that are null on either side).
- **What A did** (2-4 sentences): plan, final artifact, verifier outcome.
- **What B did** (2-4 sentences): same shape as A.
- **Why outcomes differ** (2-4 sentences): the actual mechanism. Not "B was
  smarter" but "B's script used `nm -n` so its addresses matched the verifier's
  ground truth, A's script used PIE-relocated virtual addresses which the
  verifier doesn't normalize".
- **Generalizable lesson** (optional, 1-2 sentences): is this a pattern that
  probably affects other tasks, or a one-off accident of this verifier? Skip
  if unclear from one task.

## Tools you'll need

- `ls -d` to discover the `<task>__<suffix>` trial directories
- `jq` for `result.json`
- file reads against `$TRIAL_DIR/agent/` and `$TRIAL_DIR/verifier/`
- file reads against the dataset cache (`~/.cache/harbor/datasets/...`)

No Python imports, no `harbor` package required. Everything you need is on
disk as JSON / text files.
