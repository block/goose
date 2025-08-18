---
title: Repository Search with Tree-sitter Indexing
sidebar_label: Repository Search (Experimental)
sidebar_position: 5
---

> Experimental: Gives Goose a structured understanding of your codebase so it can answer deeper questions (relationships, key entry points, symbol lookup) across multiple languages.

## What Is This?
An internal capability the LLM agent uses (not something you routinely invoke) that scans the repository and keeps an in‑memory graph of symbols (functions, types, etc.) plus relationships (callers, callees, containment, imports). The agent consults this graph before or during multi‑step reasoning to choose better starting points and jump across code logically.

## Why You Might Care
| If you want to… | Indexing helps by… |
|------------------|--------------------|
| Find where to start in a large unfamiliar repo | Surfacing ranked "central" entry points |
| Jump across a call chain while debugging | Providing callers / callees relationships |
| Ask “Who uses X?” or “Where is Y defined?” | Resolving definitions & references quickly |
| Reduce irrelevant file reads by the agent | Prioritizing important symbols first |
| Work in a polyglot repo | Normalizing entities across languages |

## Common Use Cases
- On‑ramp to a new service: “List key entry points related to auth/session.”
- Debugging a failing request path by traversing callers.
- Estimating impact before a refactor (who calls this function?).
- Quickly locating a type/struct/class whose name you only half remember.
- Improving multi‑step agent plans that need good starting symbols.

## Key Benefits (User View)
| Benefit | What You Get |
|---------|--------------|
| Faster symbol lookup | Jump to definitions or related code quickly |
| Call graph awareness | Ask about callers / callees of important functions |
| Cross‑language support | Mixed repos (e.g. frontend TS + backend Rust + scripts) still index |
| Better agent decisions | Improves tool routing & reduces irrelevant file reads |
| Lightweight & local | Runs locally; no code leaves your machine |

## When Does It Run?
You generally do not run anything manually. The first time the agent needs structural code insight it transparently builds the index (in memory). A time‑to‑live (TTL, default 600s) triggers a silent refresh later. Optional background mode can pre‑warm the index. Manual CLI queries exist only for debugging.

## Quick Start (Agent Focused)
1. Enable experimental features:
```
export ALPHA_FEATURES=true
```
2. Start Goose (CLI, desktop, or web) and simply ask a structural question, e.g.:
  “List key entry points handling scheduling logic” or “Who calls RepoIndexService::build?”
3. The agent will auto‑build (first use) then answer using ranked symbols + call graph data.
4. (Optional) Inspect status:
```
ALPHA_FEATURES=true goose repo status --path .
ALPHA_FEATURES=true goose repo status --path . --json
```
5. (Optional, debugging) Run a direct symbol query outside the agent:
```
ALPHA_FEATURES=true goose repo query --path . --symbol AuthService
```
This bypasses reasoning and just prints raw matches – useful for verifying extraction.

## Desktop UI (Optional Visibility)
If the menu item “Index Repository (Tree-sitter)” is enabled (ALPHA_FEATURES), invoking it forces an immediate build; otherwise the agent will still build lazily on demand. For normal usage you can ignore the menu and just converse.

## Supported Languages (Current)
Rust, Python, JavaScript, TypeScript, Go, C#, Java, C++, Swift.

## What Changes After Indexing?
Before: Goose relies on ad‑hoc fuzzy text scans.  
After: Goose can resolve symbol definitions, surface ranked “central” entities sooner, and traverse callers / callees for richer answers.

## Example Output (Truncated)
```
ALPHA_FEATURES=true goose repo query --path . --symbol RepoIndexService --show-score
{
  "results": [
    {
      "name": "RepoIndexService",
      "kind": "class",
      "file": "crates/goose/src/repo_index/service.rs",
      "rank": 0.0123,
      "score": 0.94
    }
  ]
}
```

## Enabling / Disabling
- Set `ALPHA_FEATURES=true` to expose the feature (CLI + UI menu / background auto index). Unset it to hide.
- Optional env for background behavior: `GOOSE_AUTO_INDEX=0` (disable), `GOOSE_AUTO_INDEX_WATCH=1` (enable watch), `GOOSE_AUTO_INDEX_WRITE_FILE=1` (persist JSONL on background runs).
  - Status meta file: `.goose-repo-index.meta.json` (written after each background/manual run when ALPHA_FEATURES enabled)

## Typical Workflow (Agent Centric)
1. Open a project.
2. Ask a question needing structural understanding.
3. Agent triggers first build (transparent) and caches it.
4. Further symbol questions use the cache; a refresh happens silently after TTL or via watch mode.
5. Optionally inspect with `repo status` if you want confirmation or counts.

## Troubleshooting
| Issue | Fix |
|-------|-----|
| Menu item missing | Ensure `ALPHA_FEATURES=true` was exported before launching the UI |
| CLI says experimental | Re-run with `ALPHA_FEATURES=true` prefixed |
| Output file empty / tiny | Verify there are supported language files; check for glob‑ignored paths |
| Slow first build | Large repos: allow full pass; subsequent builds reuse OS caches |

## Limitations (User Level)
- No automatic watch mode yet—rerun after large refactors.
- Only symbol/function level granularity; variable/field level coverage is minimal in some languages.
- Ranking heuristics are experimental and may change.

## Want the Deep Dive?
Full technical architecture, data model, ranking math, environment overrides, search scoring, and upgrade guidance live in the separate technical reference:

➡️  See: [Repository Indexing Technical Reference](./repo-index-technical-reference)

---
*Experimental: interfaces and JSON output may change. Pin to a commit or feature‑gate usage in downstream tooling.*

<!-- (Advanced content moved to repo-index-technical-reference.md) -->
<!-- END -->
## Feature Flags & Stability
The repository indexing system is experimental; APIs and output schemas may evolve. The Rust implementation currently builds in the `repo-index` code by default (via the CLI crate's dependency enabling the feature). Exposure to end users is controlled at runtime by the environment variable `ALPHA_FEATURES`.

### Enabling the feature (current behavior)

Runtime opt‑in only:

1. Set `ALPHA_FEATURES=true` to expose the experimental CLI `repo` subcommands and the desktop UI menu item "Index Repository (Tree-sitter)".
2. Launch the CLI / UI as usual.

Examples:

Enable for the desktop app during development:
```
export ALPHA_FEATURES=true
cd ui/desktop
npm run start-gui
```

Just recipe (if defined) for UI:
```
just run-ui-alpha   # assumes it exports ALPHA_FEATURES=true for you
```

Without `ALPHA_FEATURES=true` the agent falls back to lighter text heuristics only (no structured symbol graph) and the repo subcommands/menu stay hidden.

### About the Cargo feature
Internally the code is still feature‑gated with `repo-index` (plus language specific `tree-sitter-*` features). Because the CLI crate depends on the core crate with `features = ["repo-index"]`, workspace builds already compile the indexing code—no extra `--features` flag is required right now. If the project later decides to make the build truly optional (e.g. to reduce compile time or binary size) documentation will be updated with the explicit build command.

If you are experimenting locally and want to trim unused grammars, you can manually adjust the dependency features and rebuild (advanced / not required for normal usage).

## Architecture at a Glance
1. Tree-sitter extraction streams JSONL entities (legacy path retained).
2. `RepoIndexService::build` loads entities into memory, constructs graphs, runs PageRank.
3. Search layer blends lexical similarity with normalized rank for ordering.
4. Agent tool layer exposes search/stats (auto-build on demand) with caching.
5. CLI / UI expose only thin debugging/status surfaces; normal users rely purely on agent behavior.

```
+-------------+     +------------------+     +------------------+     +---------------------+
| Source Tree | --> | Tree-sitter Pass | --> | RepoIndexService | --> | Agent Tools / CLI   |
+-------------+     +------------------+     |  Graph + Rank    |     |  Build / Search     |
                                             +------------------+     +---------------------+
```

## Extracted Entities & Relationships
Languages supported (0.20.x grammar family): Rust, Python, JavaScript, TypeScript, Go, C#, Java, C++, Swift.

For each file we record entities (language-specific kinds collapsed to: class/struct/type, function/method, other, file pseudo-entity). Captured fields include:
- `file`, `language`, `kind`, `name`, `parent`, `signature`, `start_line`, `end_line`, `doc`, `calls` (if collected).

### Relationship Graph
Edges stored during build:
- Call edges (function → callee)
- Containment edges (parent ↔ child, bidirectional weight share)
- Import edges (file → imported file) with unresolved imports tracked separately.

### PageRank Weights (Env Overrides)
| Variable | Default | Meaning |
|----------|---------|---------|
| `GOOSE_REPO_RANK_CALL_WEIGHT` | 1.0 | Weight of call edges |
| `GOOSE_REPO_RANK_IMPORT_WEIGHT` | 0.5 | Weight of import edges |
| `GOOSE_REPO_RANK_CONTAINMENT_WEIGHT` | 0.2 | Weight for containment (both directions) |
| `GOOSE_REPO_RANK_DAMPING` | 0.85 | PageRank damping factor |
| `GOOSE_REPO_RANK_ITERATIONS` | 20 | Iteration count |

If all three edge weights are zero defaults are restored to avoid a degenerate matrix.

## Search Ranking
Lexical tiers (exact > prefix > substring > Levenshtein<=2) produce a lexical score (0–1). Final score = `0.6 * lexical + 0.4 * normalized_rank` (subject to future tuning). Exact-only mode bypasses blending.

Filters / flags:
- `--exact-only`
- `--min-score <f32>`
- `--show-score` (surfaced in tool JSON)
- Depth-limited traversal: callers / callees.

## Agent Tools (Internal)
| Tool | Purpose (Agent) | Notes |
|------|-----------------|-------|
| `repo__search` | Retrieve ranked symbol matches + optional callers/callees | Triggers initial build if cache absent / expired |
| `repo__stats` | Lightweight counts / weights (also auto-build) | Mainly for debugging & status surfacing |

### Search Tool Output
```
{
  "results": [
    {
      "id": <u32>,
      "name": "symbol",
      "kind": "function" | "class" | ...,
      "file": "relative/or/abs/path",
      "rank": <f32>,
      "score": <f32|null>,
      "callers": [<u32>]?,
      "callees": [<u32>]?
    }
  ]
}
```

### Stats Tool Output
```
{
  "root": "/abs/path",
  "files": <usize>,
  "entities": <usize>,
  "unresolved_imports_files": <usize>,
  "rank_weights": {
     "call": <f32>,
     "import": <f32>,
     "containment": <f32>,
     "damping": <f32>,
     "iterations": <u32>
  }
}
```

## Debug CLI Example
```
ALPHA_FEATURES=true goose repo query --path . --symbol RepoIndexService --show-score --callers-depth 1
```
Use only to inspect extraction / ranking; the agent already does this internally.

## Programmatic Build
```rust
use goose::repo_index::{RepoIndexOptions};
use std::path::Path;
let opts = RepoIndexOptions::builder()
  .root(Path::new("."))
  .output_file(Path::new("/dev/null"))
  .build();
let (svc, stats) = goose::repo_index::service::RepoIndexService::build(opts)?;
println!("{} entities", stats.entities_indexed);
```

## Caching Strategy (Invisible to User)
Per-root in-memory cache keyed by canonical path. First tool call builds; TTL (env `GOOSE_REPO_INDEX_TTL_SECS`, default 600) triggers rebuild on next access. Set TTL to 0 to pin the initial build.

## Limitations & Roadmap
- No incremental / watch rebuild yet
- Import resolution heuristic; does not fully resolve packages/modules across complex layouts.
- Blended score weights fixed in code (env overrides only for PageRank, not lexical/rank blend proportion yet).
- Potential memory optimizations (string interning, arena allocation) not implemented.

## Testing
Unified tests cover:
- Rank variance & env overrides.
- Fuzzy ordering & minimum score filter.
- Tool integration (build/search/stats) via `repo_tools_tests`.

## Migration Notes
This feature is new; earlier draft documents have been removed after consolidation.

## Implementation Phases
High-level phased rollout of the feature (current status: Steps 1–6 and 8 complete; Step 7 watch mode pending):

1. Service Skeleton: In‑memory `RepoIndexService`, entity storage, inverted index, JSONL export parity.
2. Graph Construction: Call + containment edges, file pseudo‑entities, traversal helpers (BFS callers/callees).
3. Query & CLI: CLI `goose repo query <symbol>` with optional traversal depth.
4. Import Extraction: Per‑language heuristics (python, js/ts, go, rust, cpp includes, java, c_sharp, swift) + unresolved tracking.
5. PageRank: Weighted centrality over combined edges with env overrides.
6. Fuzzy & Ranked Search: Lexical tiers + blended score ordering; min score & exact‑only flags.
7. Incremental / Watch (Planned): Not yet implemented; future partial rebuild + rank refresh.
8. Agent Tool Integration: Expose build/search/stats as agent tools & caching layer.

## Data Model (Current)
Canonical (subject to change):
```
FileRecord {
  id: u32,
  path: String,
  language: &'static str,
  entities: Vec<u32>,
}

StoredEntity {
  id: u32,
  file_id: u32,
  kind: EntityKind, // Class | Function | Method | File | Other
  name: String,
  parent: Option<String>,
  signature: String,
  start_line: u32,
  end_line: u32,
  calls: Option<Vec<String>>, // raw callee names (resolved later)
  doc: Option<String>,
  rank: f32, // filled after PageRank
}
```

## Per‑language Extraction Details
Expanded specifics per language (extraction richness varies):

- JavaScript / TypeScript: classes, functions (top‑level + methods), doc comments, intra‑function call relationships.
- Python: classes, functions (with decorators + docstrings), parent relationships, call relationships.
- Rust: structs, enums, traits, impl fns (names, signatures, doc comments), call relationships.
- C++: classes, templates, functions, call relationships (heuristic; templates recorded as entities).
- Go: types (struct/interface/etc.), fields, functions (with generics where present), variables (top‑level), imports, call relationships.
- Java / C# / Swift: classes/structs/protocols, methods/functions, call relationships (baseline extraction).

Common entity fields: `file`, `language`, `kind`, `name`, `parent`, `signature`, `start_line`, `end_line`, `doc`, `calls` (where available).

Limitations per language mirror general limitations: additional constructs (variables, constants, properties, enum variants) mostly unindexed outside Go.

## Environment Variable Constraints
Existing table lists defaults; implicit constraints retained here for clarity:
- Edge weights (`*_WEIGHT`) must be >= 0. If all three are 0, defaults are restored to avoid a degenerate matrix.
- `GOOSE_REPO_RANK_DAMPING` in [0.0,1.0].
- `GOOSE_REPO_RANK_ITERATIONS` in [1,200].

## Performance & Future Optimization Notes
## Observability (Events & Metrics)
The indexing layer emits structured tracing events you can forward to OTLP / Langfuse / logs. Each event includes stable fields for internal dashboards.

| Event | When | Key Fields |
|-------|------|------------|
| `repo.index.build` | Any index build (lazy query, stats, background, watch) | `root`, `duration_ms`, `files`, `entities`, `trigger` (`query`\|`stats`\|`background`\|`watch`), `ttl_secs?`, `background` (bool) |
| `repo.index.search` | After a symbol search completes | `root`, `query`, `results`, `limit`, `exact_only`, `callers_depth`, `callees_depth` |
| `repo.index.stats` | After stats gathered | `root`, `files`, `entities` |

Monotonic counters (prefixed via tracing metadata):
| Counter | Increment Condition |
|---------|---------------------|
| `counter.goose.repo.builds` | Each successful build (any trigger) |
| `counter.goose.repo.search_calls` | Each search invocation |
| `counter.goose.repo.stats_calls` | Each stats invocation |

Example log line (conceptual):
```
INFO event=repo.index.build counter.goose.repo.builds=1 root="/workspace" trigger="query" duration_ms=842 files=120 entities=1435 ttl_secs=600 Repository index built (query path)
```

Dashboards can chart: build durations (p95), search results count distribution, build frequency per root, time from process start to first build.

Initial considerations (summarized):
- Potential parallel parse (currently sequential/simple concurrency depending on implementation state).
- String interning / arena allocation to reduce memory for large repos.
- Optional reduced mode skipping doc/signature capture for speed.
- Incremental rebuild (watch) to avoid full parse on small edits (future Step 7).

<!-- Progress snapshot removed (implicit in phases and roadmap) -->

## Tree-sitter Version Compatibility
Current grammar family targets `tree-sitter` 0.20.x to maximize multi-language support (javascript, typescript, go, c_sharp, java, python, rust, cpp, swift). Many upstream grammar crates have not fully migrated to 0.23.x at the time of writing; attempting to upgrade prematurely can trigger `cc` crate / native lib conflicts. Track grammar crate releases and only upgrade when a consistent set for all enabled languages is available.

### Upgrade Guidance
1. Check crates.io (or upstream repos) for each language grammar crate version supporting the desired core `tree-sitter`.
2. Ensure all grammar crates + core crate share compatible `cc` / build dependencies.
3. Update versions in one commit; run full build with `--features repo-index`.
4. Re-run extraction tests + a smoke search to ensure no AST kind regressions.
5. If a subset lags behind, prefer deferring upgrade vs. fragmenting support.

## Rationale
An in-memory, graph-enriched repository index unlocks:
- Fast iterative symbol search (avoids repeated JSON streaming passes).
- Higher-level reasoning: influence (PageRank), impact analysis (callers/callees), import surface.
- Improved agent tool routing by surfacing central entities rather than only lexical matches.
- Extensibility: future incremental updates, richer relationship extraction.

## Contributing
Contributions welcome: additional language constructs, improved import resolution, incremental indexing, richer search filters.

---
*Experimental: interfaces and JSON output may change. Pin to a commit or feature-gate usage in downstream tooling.*
