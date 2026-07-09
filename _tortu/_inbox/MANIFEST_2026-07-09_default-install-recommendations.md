# Default Install Manifest — Extensions, Recipes & Skills

**Status:** DRAFT — for Doug's review. Nothing here has been installed, enabled, or committed.
**Compiled by:** Cowork, 2026-07-09, from `goose-docs.ai`'s live catalogs (`servers.json`,
`skills-manifest.json`, and the recipe cookbook source in `documentation/src/pages/recipes/data/`).
**Scope:** what to add to the *default* tortu-forks/goose install — i.e. `_tortu/bootstrap.sh` /
`_tortu/config/` territory — not a one-off session enable.

Once Doug picks what to keep, this becomes a real handoff for Claude Code to wire into
`bootstrap.sh` / `config.yaml` / `_tortu/recipes/`.

---

## 1. Extensions

Goose's built-ins (Developer, Extension Manager, Todo, Summon, etc.) are already configured in
`_tortu/config/config.yaml` — not repeated here. This is the third-party/community catalog
(61 entries in `servers.json`), filtered to what's actually relevant to this project: dogfooding +
upstreaming Rust patches to `aaif-goose/goose`.

### Tier 1 — recommend adding now

| Extension | Repo | Why |
|---|---|---|
| **GitHub** | [github/github-mcp-server](https://github.com/github/github-mcp-server) | Native PR/issue ops — the missing piece for the fork→PR workflow in `BUILD_AND_SUBMIT.md`. Goose-endorsed. |
| **Context7** | [upstash/context7](https://github.com/upstash/context7) | Live library docs instead of guessing at Rust crate APIs — directly addresses the "not yet verified against the compiler" pattern seen on the recipe-list patch. Endorsed. |
| **Fetch** | [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers/tree/main/src/fetch) | Official MCP reference server, lightweight web fetching for research. Endorsed. |
| **Playwright** | [microsoft/playwright-mcp](https://github.com/microsoft/playwright-mcp) | Browser automation — useful for `ui/desktop` (Electron) testing and doc-site work. Endorsed, Microsoft-maintained. |
| **Chrome DevTools** | [ChromeDevTools/chrome-devtools-mcp](https://github.com/ChromeDevTools/chrome-devtools-mcp) | Same rationale as Playwright, deeper inspection. Endorsed, official Chrome team. |
| **Knowledge Graph Memory** | [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers/tree/main/src/memory) | Official MCP reference server; more structured than the built-in Memory extension as fork knowledge grows. Endorsed. |

### Tier 2 — worth trying, not essential

| Extension | Repo | Why | Note |
|---|---|---|---|
| **Repomix** | [yamadashy/repomix](https://github.com/yamadashy/repomix) | Repo analysis/organization — the goose monorepo (`crates/`, `ui/desktop`) is large enough to benefit beyond Developer alone. | Not endorsed, single maintainer. |
| **Beads** | [steveyegge/beads](https://github.com/steveyegge/beads) | Git-backed issue tracker for AI agent task management — could dovetail with how patches are already tracked (`_tortu/patches/<name>/`). | Not endorsed. Pairs with the `beads` skill below. |
| **GitMCP** | [idosal/git-mcp](https://github.com/idosal/git-mcp) | Pulls live docs/repo content for any GitHub project into context. | Not endorsed. **Same repo link as "goose Docs" below — check before installing both, may be a duplicate/pre-configured instance.** |
| **JetBrains** | [JetBrains/mcp-jetbrains](https://github.com/JetBrains/mcp-jetbrains) | IDE integration. | Only relevant if you're actually using a JetBrains IDE for this work — otherwise skip. |
| **Container Use** | [container-use.com](https://container-use.com) | Isolated dev environments per task; the fork already has `BUILDING_DOCKER.md`/`Dockerfile`. | Not on GitHub (own site), not verified against this repo's actual container workflow yet. |

### Explicitly skip

Everything finance/e-commerce/IoT-flavored (Alby, Square, Cash App, MBot, Neighborhood), niche
domain tools (I Ching, Scholar Sidekick, NostrBook, Ophis), and DB/data-platform ones (MongoDB, Neon,
Supabase, DataHub, OpenMetadata) unless a specific patch actually touches a database.

---

## 2. Recipes

### Already installed (community, from Block's Goose Subagents Workshop)

Already living in `_tortu/recipes/` — no action needed, listed for completeness:

`code-reviewer` · `test-generator` · `doc-writer` · `codebase-analyzer` · `codebase-locator` ·
`web-researcher` — sourced from
[block/goose-subagents-workshop](https://github.com/block/goose-subagents-workshop).

### Recommend adding, from the official cookpanel

| Recipe | Why |
|---|---|
| **Generate Commit Message** | Matches the DCO-signed commit step already in `BUILD_AND_SUBMIT.md`. |
| **PR Generator** | Auto-drafts PR descriptions from a local diff — could template the attribution-header + before/after PR body format directly. |
| **Test Coverage Optimizer** | `BUILD_AND_SUBMIT.md` already flags "consider adding a test for the new grouping/duplicate-flagging behavior" as a to-do — this formalizes that step. |
| **Lint My Code** | Codifies the `cargo clippy --all-targets -- -D warnings` habit already noted in `.goosehints`. |
| **Generate Change Logs from Git Commits** | Fits the fork's own "merge into fork's `main`" step in `MANIFEST.md`. |

### Strategically interesting — could reshape the patch workflow itself

| Recipe | Why |
|---|---|
| **RPI family** (`rpi-research`, `rpi-plan`, `rpi-iterate`, `rpi-implement`) | A formal Research→Plan→Implement pipeline — essentially what got improvised by hand for `recipe-list-formatting` (`HANDOFF.md` → `PATCH.md` → `BUILD_AND_SUBMIT.md`). Worth trying as a way to make that repeatable. |
| **Ralph Work / Ralph Review** | Single-iteration work + "cross-model review, returns SHIP or REVISE." A built-in quality gate before dogfooding/submitting — matches the validate-before-PR discipline already in place. |

### Worth trying, lower priority

| Recipe | Why |
|---|---|
| **Code Review Mentor** | Compare against the existing `code-reviewer` recipe — may or may not add enough to be worth running both. |
| **Technical Debt Tracker** | Useful now that a long-lived fork is being maintained — helps spot the *next* worthwhile upstream patch. |
| **Add MCP Server** | Meta-recipe for contributing a new MCP server to goose's own docs (`servers.json` entry + tutorial). Not needed today; keep in back pocket if Tortu ever builds one worth upstreaming. |
| **Maverick — Behavioral Adaptation** | Adjusts Goose's pacing/tone/autonomy to your working style. Curiosity-driven, not workflow-critical. |

### Skip

Domain-specific cookbook recipes that don't touch this project: A/B Test Framework Generator,
CSV/data-cleaning recipes, Kafka topic creator, OpenAPI→Locust, DataHub/OpenMetadata, Sunno song
formatter, web accessibility auditor, and the Flutter/JS-React/PHP PR reviewers (language-specific —
the JS one would only matter if patching `ui/desktop` rather than the Rust CLI).

---

## 3. Skills

All sourced from [block/Agent-Skills](https://github.com/block/Agent-Skills), installed via
`npx skills add https://github.com/block/Agent-Skills --skill <id>`. Seven exist total; recommending
four:

| Skill | Why |
|---|---|
| **`code-review`** | Complements the existing `code-reviewer` recipe with a standing checklist rather than an on-demand recipe run. |
| **`testing-strategy`** | Pairs with the recommended Test Coverage Optimizer recipe above. |
| **`beads`** | Same git-backed task-tracking concept as the Beads extension (Tier 2 above) — a skill version if the full extension feels like too much. |
| **`rp-why`** | Analyzes session history for AI-collaboration quality (cognitive depth, tool orchestration, delegation trust). Not workflow-critical, but matches Doug's general interest in refining how agent collaboration works (Framework/BASIS+M territory) — worth trying as a curiosity. |

**Situational, not default:**

| Skill | Why not default |
|---|---|
| `frontend-design` | Only relevant if touching `ui/desktop` (the TypeScript/React Electron app) rather than the Rust CLI. |
| `api-setup` | General-purpose, no specific tie to current work. |
| `goose-blog-post` | Specific to writing blog posts for the `block/goose` project — situational if Doug ever writes publicly about the fork/contribution, not a default. |

**Note on Tortu's own skills:** `_tortu/config/skills/` already has five custom skills
(`kn-nas`, `kn-nrr`, `kn-nsr`, `kn-summary`, `kn-wsr`) — these are separate from the
`block/Agent-Skills` catalog above and untouched by this manifest.

---

## Open questions for Doug

1. GitMCP and "goose Docs" resolve to the same repo (`idosal/git-mcp`) in `servers.json` — worth
   confirming before installing both as if they were distinct.
2. JetBrains extension — only worth it if you're actually using a JetBrains IDE for this work.
3. Any of the "Tier 2" / "worth trying" items you'd rather skip entirely, or promote to Tier 1?
