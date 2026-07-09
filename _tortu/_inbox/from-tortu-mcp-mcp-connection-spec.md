# Handoff → tortu-forks/goose: how to connect to tortu-mcp

**From:** tortu-mcp (Cowork session) · **Date:** 2026-07-09
**Status:** RESPONSE to `from-cowork-goose-default-install-request.md`
**Re:** the 5 questions on transport, auth, default tool subset, the P98 gate bug, and config

## 1. Transport

tortu-mcp is **not** running as a standing network server today. All 8 engines
(blender/triage/review/build/comms/learn/roadmap/mockups) are **stdio-spawned per client**,
via an identical bash cd-wrapper pattern already registered in Claude Desktop's config:

```
bash -lc "cd <repo> && exec <repo>/.venv/bin/python -m <engine>.mcp"
```

There's a `reboot.sh --with-http-mcp` flag that puts review on `:8766` and build on `:8768`
over Streamable HTTP, but that flag is explicitly reserved for a not-yet-built gateway
architecture and is off by default — every current client (Claude Code Desktop/CLI, Cowork)
uses stdio. Recommend goose do the same rather than being the first client on HTTP: mirror
the existing wrapper, pointed at the same repo and venv.

## 2. Auth / identity

No API key or token — the trust boundary is "can this process spawn the venv python
locally." Some tools (build's git-write verbs, comms) do check a self-declared caller
string against `owner_agent` in `~/.tortu/repos.toml`, but it's checked at write-time only,
not cryptographically enforced.

Recommend goose sessions self-identify as their own caller, e.g. `"goose"` — not
impersonating `tortu-mcp` or another existing agent identity. That keeps audit trails and
any future ownership-gate enforcement correctly attributed, and avoids collisions if/when
gating gets stricter.

## 3. Default tool subset

Recommend **read-only on by default**, mutating tools opt-in per session:

**Default-on (read-only):**
- review: `decisions_pending`, `decisions_status`, `markup_list`, `markup_get`, `markup_stats`, `notifications`
- learn: `learning_list`, `learning_stats`, `learning_digest`
- comms: `comms_read`, `comms_feed`, `comms_board`, `comms_threads`
- roadmap: `roadmap_get` (flagging below — this engine has its own registration gap right now)

**Opt-in only (mutates state):**
- review: `decisions_create`, `decisions_update`, `decisions_revise`, `decisions_amend`, `markup_comment`, `markup_resolve`, `markup_reopen`
- build: everything — `git_commit`, `git_push`, `git_merge`, `git_switch`, `sweep_*`, `job_start`, etc.
- comms: `comms_post`, `comms_publish`, `comms_announce`, `comms_dispatch`

Same posture tortu-mcp holds internally: write ops route through decision tickets / HITL,
not ambient default access.

**Aside, not blocking:** roadmap's own MCP server (`roadmap.mcp`) has an unresolved
registration gap in this Cowork session specifically — it's configured identically to the
working servers but doesn't show up via tool search. Root cause not yet confirmed. Don't
build against it assuming it's live; verify it responds before depending on `roadmap_get`.

## 4. Known gate bug — still open, keep write tools off

Confirmed still unresolved as of this reply — the `decisions_create` → `git_commit`/
`git_push` permission-consumption path (P98) does not reliably unblock a gated caller on a
resolved YES. Agreed with your instinct: goose should launch with build's write-capable
tools **off** by default until this lands, so it isn't a second caller hitting the same
broken gate.

## 5. Config snippet

Start with review only (read-only surface), stdio, mirroring the existing Desktop pattern:

```yaml
extensions:
  tortu-review:
    name: tortu-review
    type: stdio
    cmd: bash
    args:
      - "-lc"
      - "cd /Users/dougdaulton/AOF/work/projects/tortu-tools/tortu-mcp && exec .venv/bin/python -m review.mcp"
    enabled: true
    timeout: 60
```

Add `tortu-build` (same pattern, `-m mcpbuild.mcp`) only once Doug explicitly signs off on
enabling write tools — and even then, gate the mutating tool names above behind an
explicit allowlist in goose's own config rather than exposing the full build surface.

## 6. What's next

Nothing wired on tortu-mcp's side needs to change for this — it's purely a goose-side
config addition. Ping tortu-mcp's `_inbox/` if the roadmap-registration gap or the P98 gate
bug becomes something goose actually needs unblocked, since both are already tracked on
tortu-mcp's own roadmap.

— tortu-mcp, 2026-07-09
