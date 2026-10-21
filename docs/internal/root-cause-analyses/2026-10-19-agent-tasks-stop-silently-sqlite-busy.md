# RCA: Agent tasks stop midstream silently — session DB `SQLITE_BUSY` aborts the turn

**Date:** 2026-10-19
**Affected task:** `Flockx creative pipeline architecture` (session `20261015_35`), plus other concurrent sessions
**Severity:** High (silent loss of in-progress turns; user sees the pulsing "active" indicator disappear with no error)
**Classification:** Local harness bug — unrecoverable `SQLITE_BUSY` in prompt-critical session persistence

## Summary

Since the ASI:One Work upgrade, agent tasks stop midstream without an error message.
Investigation of the local CLI logs and session database shows the turns are not
stalling at the model provider (the Oct 14 RCA); they are aborted by SQLite
lock contention on the shared session database. The `sessions.db` has grown to
788 MB with a 107 MB WAL, and the desktop app runs multiple goose processes
against it (`goose acp` plus headless `goose run` schedules). Under concurrent
write load — most visibly `save_compacted_conversation` rewriting a 5,528-message
conversation — other sessions' writes fail with
`error returned from database: (code: 5) database is locked`. That error
propagates out of the agent response stream and is sent to the client as an
ACP `session/prompt` error. The UI then finishes the prompt attempt and clears
the active state, so the pulsing icon simply disappears: no turn, no error
message in the transcript.

## Factual evidence

- `sessions.db` on disk: 788 MB, `sessions.db-wal` 107 MB, 181,387 messages / 420 sessions.
- Session `20261015_35` DB tail: last tool response at 21:12:08 UTC, then nothing
  until Work injected "we crashed, please keep working on your task" plus a
  compaction summary at 21:18 — the app's crash-resume path.
- CLI log `20261019_164822.log`:
  - 21:17:16 `Performing message compaction` on `20261015_35` (rewrite of the whole conversation).
  - Immediately after: `slow statement` warnings up to 4.1 s (session list scans on the 788 MB DB).
  - 21:18:40–21:18:51 — five ACP responses:
    `session/prompt error -32603 "Error in agent response stream: error returned from database: (code: 5) database is locked"`
    (requests 214, 15, 138, 144, plus one more) — these are other concurrent
    sessions whose mid-turn writes hit the lock.
- Same signature on prior days: 2026-10-14 (56 hits), 2026-10-15 (1), 2026-10-18 (4), 2026-10-19 (6).

## Root cause

SQLite returns `SQLITE_BUSY` immediately — without honoring the pool's 30 s
`busy_timeout` — whenever honoring it could deadlock (two concurrent writers
upgrading locks, or a cross-process writer holding the write lock). The pool
(`SessionStorage::create_pool`) sets WAL + a 30 s busy timeout, but that timeout
does not apply to these deadlock-detected `BEGIN IMMEDIATE` upgrades across the
sqlx pool's multiple connections and the concurrent `goose run` processes. The
write methods (`add_message`, `replace_conversation`, `save_compacted_conversation`)
treated the error as fatal; it propagated through the agent response stream and
aborted the turn, which surfaced as a silent stop.

The upgrade made this visible: compaction (`save_compacted_conversation`) and
concurrent scheduled tasks (`goose run`) both grew in usage, and the session DB
crossed the size where lock-held windows overlap with other sessions' mid-turn
writes.

## Fix

Wrap the prompt-critical session writes (`add_message`, `replace_conversation`,
`save_compacted_conversation`) in `with_busy_retry`: on `SQLITE_BUSY`, retry up
to 3 times with exponential backoff (200 ms, 400 ms, 800 ms), logging each
retry. Non-busy errors are passed through unchanged. A transient lock window
from compaction or a concurrent writer now delays the write by well under a
second instead of killing the turn.

Follow-up RCA (2026-10-21, phantom-decline investigation) found two more
prompt-critical writes that could fail on SQLITE_BUSY while their companion
write succeeded, leaving divergent state behind: `update_message_metadata`
(message visibility updates) and `update_tool_request_meta` (the
`goose.executable` flag patch written when a tool call enters approval). If
the meta patch persisted but the approval card message did not (or vice
versa), the resume path reconstructed a tool request flagged
`goose.executable=false` with no pending approval card and synthesized a
"the user declined" response the user never made. Both writes are now wrapped
in `with_busy_retry` as well, so the executable flag and approval state can no
longer diverge on a transient lock: either both writes land or both are
retried. The full phantom-decline analysis lives in the community web app
`docs/internal/asi-work-phantom-decline-rca.md`.

## Regression tests

Added to `crates/goose/src/session/session_manager.rs` tests:

- `busy_error_detects_sqlite_locked` — busy errors are classified busy; other DB errors are not.
- `with_busy_retry_retries_locked_errors_then_succeeds` — a busy error on the first attempt is retried and the operation succeeds.
- `with_busy_retry_gives_up_after_max_retries` — persistent busy fails after exactly `DB_BUSY_RETRIES + 1` attempts.
- `with_busy_retry_passes_through_non_busy_errors` — non-busy errors fail on the first attempt, never retried.

## Follow-ups (not in this change)

- The 788 MB `sessions.db` needs maintenance (archival/VACUUM) — slow 2–4 s session-list scans remain.
- The ACP client should surface `session/prompt` errors in the transcript rather than silently clearing the active state.
