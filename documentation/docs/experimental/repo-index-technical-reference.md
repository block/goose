---
title: Repository Indexing Technical Reference
sidebar_label: Repo Index Technical Reference
sidebar_position: 6
---

> Deep dive: architecture, configuration variables, data model, ranking, and upgrade guidance for the experimental repository indexing feature.

## Overview
High‑level pipeline: Tree-sitter parses → entities (functions/types) → relationship graph (calls, imports, containment) → weighted PageRank → blended search (lexical + rank) → agent / CLI tools.

## Feature Flags & Stability
The system is experimental; APIs and output schemas may evolve. Runtime exposure is controlled by `ALPHA_FEATURES=true`. The code is compiled in current workspace builds (CLI depends on the feature). Making it fully optional build‑time is a future consideration.

## Architecture
1. Extraction: Tree-sitter per language (Rust, Python, JS/TS, Go, C#, Java, C++, Swift) produces raw entity records (legacy JSONL path retained for compatibility).
2. Normalization: Language‑specific kinds collapsed into a small internal enum (Function/Method, Type/Class/Struct/Enum/Trait, File pseudo entity, Other).
3. Graph Construction: Call edges, containment edges (parent ↔ child), import edges (file → file; unresolved tracked separately).
4. Ranking: PageRank across combined weighted edges.
5. Search: Lexical match tiers (exact > prefix > substring > Levenshtein<=2) blended with normalized rank; optional callers/callees expansion.
6. Tools: Agent tool layer caches built indexes; CLI and (future) UI invoke those tools.

Diagram:
```
+-------------+     +------------------+     +------------------+     +---------------------+
| Source Tree | --> | Tree-sitter Pass | --> | RepoIndexService | --> | Agent Tools / CLI   |
+-------------+     +------------------+     |  Graph + Rank    |     |  Build / Search     |
                                            +------------------+     +---------------------+
```

## Environment Variable Overrides (Ranking)
| Variable | Default | Description |
|----------|---------|-------------|
| `GOOSE_REPO_RANK_CALL_WEIGHT` | 1.0 | Weight of call edges |
| `GOOSE_REPO_RANK_IMPORT_WEIGHT` | 0.5 | Weight of import edges |
| `GOOSE_REPO_RANK_CONTAINMENT_WEIGHT` | 0.2 | Weight for containment (both directions) |
| `GOOSE_REPO_RANK_DAMPING` | 0.85 | PageRank damping factor |
| `GOOSE_REPO_RANK_ITERATIONS` | 20 | Iteration count |

Rules:
- All edge weights must be ≥ 0. If all three weights are 0, defaults are restored to avoid degenerate matrix.
- Damping in [0.0,1.0]. Iterations in [1,200].

## Search Ranking Formula
`final_score = 0.6 * lexical_score + 0.4 * normalized_rank` (subject to future tuning). Exact‑only mode bypasses blending and returns lexical exact matches. Lexical tiers assign a tier score then normalize 0–1.

## Minimal Debug CLI Example (Optional)
```
ALPHA_FEATURES=true GOOSE_REPO_RANK_CALL_WEIGHT=1.2 goose repo query --path . --symbol RepoIndexService --show-score --callers-depth 1
```
Use only for inspecting extraction or ranking; the agent normally triggers and consumes the index automatically.

## Agent Tools
| Tool | Purpose | Key Arguments | Output Highlights |
|------|---------|---------------|-------------------|
| `repo__search` | Fuzzy + ranked search (auto-builds) | `root`, `query`, `limit`, `exact_only`, `min_score`, `show_score`, `callers_depth`, `callees_depth`, `langs[]` | ranked results |
| `repo__stats` | Repo statistics (auto-builds if missing) | `root`, `langs[]` | entity/file counts, unresolved imports, weights |

### Search Tool Output Example
```
{
  "results": [
    {
      "id": 17,
      "name": "RepoIndexService",
      "kind": "class",
      "file": "crates/goose/src/repo_index/service.rs",
      "rank": 0.0123,
      "score": 0.94,
      "callers": [21],
      "callees": [42,55]
    }
  ]
}
```

## Programmatic Build Snippet (Rust) – Debug / Bench Only
```rust
use goose::repo_index::RepoIndexOptions;
use goose::repo_index::service::RepoIndexService;
use std::path::Path;
let opts = RepoIndexOptions::builder()
  .root(Path::new("."))
  .output_null() // in-memory only; prefer this for benchmarks
  .build();
let (_svc, stats) = RepoIndexService::build(opts)?;
println!("{} entities", stats.entities_indexed);
```

## Data Model (Simplified Current)
```
FileRecord { id: u32, path: String, language: &'static str, entities: Vec<u32> }
StoredEntity {
  id: u32,
  file_id: u32,
  kind: EntityKind, // Class | Function | Method | File | Other
  name: String,
  parent: Option<String>,
  signature: String,
  start_line: u32,
  end_line: u32,
  calls: Option<Vec<String>>, // unresolved callee names
  doc: Option<String>,
  rank: f32,
}
```

## Per‑language Extraction Notes
- JavaScript / TypeScript: classes, functions, methods, doc comments, call relationships.
- Python: classes, functions (decorators/docstrings), parent and call relationships.
- Rust: structs, enums, traits, impl functions, docs, calls.
- C++: classes, templates, functions, heuristic call edges.
- Go: types, fields, functions, vars, imports, calls.
- Java / C# / Swift: classes/types, methods/functions, baseline call extraction.

Limitations: variable/field granularity uneven; some languages collect fewer relationship edges; incremental watch mode not yet implemented.

## Caching Strategy (Agent Internal)
Per canonical root path the agent keeps an in-memory index. First `repo__search` / `repo__stats` triggers build; TTL (env `GOOSE_REPO_INDEX_TTL_SECS`, default 600) causes next query after expiry to rebuild. A per-root async mutex prevents duplicate concurrent builds.

## Limitations & Roadmap
- No incremental / watch rebuild yet
- Import resolution heuristic only (multi-module/package edge cases)
- Blend weights (0.6/0.4) fixed for now; env controls only PageRank parameters
- Potential memory optimizations (string interning, arenas) pending
- Future: incremental graph updates, richer entity kinds, additional ranking signals

## Testing (Summary)
Tests validate rank weight overrides, fuzzy ordering, min score filtering, and tool integration (build/search/stats). Additions should extend these suites rather than introduce ad‑hoc test binaries.

## Upgrade Guidance (Tree-sitter Versions)
Currently targets grammar family around `tree-sitter` 0.20.x for multi-language parity. Migrating to 0.23.x prematurely risks native build conflicts. To upgrade:
1. Audit each language grammar crate for compatible versions.
2. Align all grammar versions & their transitive build deps (`cc`, etc.).
3. Update versions & rebuild (optionally with `--features repo-index`).
4. Run extraction + search tests; confirm no AST kind regressions.
5. Defer if even one core language grammar lags significantly.

## Performance Considerations
## Observability
Tracing events (all INFO level unless otherwise noted):

| Event | Description | Fields |
|-------|-------------|--------|
| `repo.index.build` | Index (re)build completed | `root`, `duration_ms`, `files`, `entities`, `trigger` (`query`/`stats`/`background`/`watch`), `background` (bool), `ttl_secs?` |
| `repo.index.search` | Symbol search finished | `root`, `query`, `results`, `limit`, `exact_only`, `callers_depth`, `callees_depth` |
| `repo.index.stats` | Stats retrieval finished | `root`, `files`, `entities` |

Counters exposed via tracing metadata (monotonic):
| Counter | Meaning |
|---------|---------|
| `counter.goose.repo.builds` | Successful builds |
| `counter.goose.repo.search_calls` | Search tool invocations |
| `counter.goose.repo.stats_calls` | Stats tool invocations |

Recommended derived metrics:
- Build latency p50 / p95
- Searches per session before first answer
- Cache reuse ratio = (search_calls - builds) / search_calls
- Time_to_first_build (process start → first build event)

Potential improvements under evaluation:
- Parallel parsing / controlled thread pool
- String interning and arena allocation
- Optional reduced capture mode (skip docs/signatures) for speed
- Incremental rebuild (watch mode) with dependency tracking

## Rationale Recap
Structured, ranked indexing unlocks faster symbol discovery, better agent planning, impact analysis (callers), and extensibility for future relationship types—all while staying local.

## Contributing
Enhancements welcome: new language constructs, improved import resolution, watch mode, alternative ranking signals, performance tuning. Please include tests + docs updates in PRs.

---
*Experimental: Interfaces and JSON output may change. Pin to a commit or gate usage downstream.*
