# Repository Search with Tree-sitter Indexing (Experimental)

> Experimental repository indexing: graph + PageRank + blended fuzzy symbol search across multiple languages.

## Overview
The Goose repository indexing stack provides:
- Incremental (build-time) Tree-sitter based parsing across multiple languages.
- In-memory graph model (calls, containment, imports) with weighted PageRank centrality.
- Fuzzy + rank blended symbol search with traversal (callers/callees) expansion.
- Agent tools (`repo__build_index`, `repo__search`, `repo__stats`) for autonomous workflows.
- CLI integration for ad‑hoc exploration.

This enables fast localization of relevant code entities, supports higher-level reasoning (e.g. impact analysis), and improves tool routing by surfacing influential symbols.

## Feature Flags & Stability
All functionality is gated behind the Cargo feature `repo-index` (plus language specific `tree-sitter-*` features). The system is experimental; APIs and output schemas may evolve.

## Architecture at a Glance
1. Tree-sitter extraction streams JSONL entities (legacy path retained).
2. `RepoIndexService::build` loads entities into memory, constructs graphs, runs PageRank.
3. Search layer blends lexical similarity with normalized rank for ordering.
4. Agent tool layer exposes build/search/stats with lightweight caching.
5. CLI & future UI layers consume agent or service APIs.

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

## Agent Tools
| Tool | Purpose | Key Arguments |
|------|---------|---------------|
| `repo__build_index` | Build + cache index | `root`, `langs[]`, `force` |
| `repo__search` | Fuzzy + rank search | `root`, `query`, `limit`, `exact_only`, `min_score`, `show_score`, `callers_depth`, `callees_depth` |
| `repo__stats` | Repo statistics | `root` |

### Build Tool Output
```
{
  "status": "built" | "cached",
  "root": "/abs/path",
  "files_indexed": <u64>,
  "entities_indexed": <u64>,
  "duration_ms": <u64>
}
```

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

## CLI Usage (Examples)
```
# Build & search
GOOSE_REPO_RANK_CALL_WEIGHT=1.2 goose repo build --root . --langs rust,python
GOOSE_REPO_RANK_CALL_WEIGHT=1.2 goose repo query RepoIndexService --show-score --callers-depth 1
```

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

## Caching Strategy
Agent layer holds an in-memory cache keyed by canonical root path. `repo__build_index` returns `status="cached"` when fresh unless `force=true`.

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
