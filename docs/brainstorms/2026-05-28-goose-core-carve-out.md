# goose-core carve-out

> Brainstorm captured 2026-05-28. Follow-up to [`2026-05-28-goose-embed-rust-sdk.md`](2026-05-28-goose-embed-rust-sdk.md) (Approach B in that doc).

## Clarified Problem Statement

**Goal:** Extract the embeddable primitives of `crates/goose` into a new `goose-core` crate so that embedders (today: `goose-embed`; tomorrow: anyone) can `cargo add goose-core` and get a meaningfully smaller dep tree than depending on `crates/goose`. `crates/goose` becomes the application runtime (sessions, scheduler, oauth, telemetry).

**Constraints:**
- `goose-cli`, `goose-server`, and `goose-embed` must keep working at every step. No big-bang merge.
- DCO sign-off (`git commit -s`), conventional commits, workspace MSRV (1.91.1).
- No public API changes for `goose-cli` / `goose-server` consumers. This is a layering change, not a refactor of behavior.
- The crate carving must actually reduce dep weight for embedders — the test is `cargo tree -p goose-embed` after the work landing significantly smaller than today.

**Pinned design decisions:**
- **Provider policy:** all built-in providers ship in `goose-core`, gated behind the same features they're gated behind today (`aws-providers`, `local-inference`, etc.). Embedders pay only for what they opt into.
- **Sequencing:** two-phase. PR 1 extracts the new traits (`SessionContextProvider`, `OAuthProvider`) inside `crates/goose`. PR 2 (or a series) physically moves modules into `goose-core`. Workspace stays green throughout.
- **`goose-embed` migration:** leave it on `crates/goose` for the entire carve-out series. Final PR re-points it at `goose-core` once the surface is stable.

**Non-goals:**
- No new public APIs. The shape of `Agent::reply`, `Provider`, `Recipe`, etc. stays exactly as today.
- No behavior changes anywhere (no perf tuning, no error-handling rewrites). Pure relayering.
- No multi-language bindings. `goose-core` is Rust; cross-language continues over ACP.
- No refactor of `platform_extensions/` — they stay in `crates/goose` because they consume `Session` state.
- No semver split. `goose-core` tracks the workspace version (same as every other crate today).

**Success criteria:**
- `cargo build -p goose-cli` and `cargo build -p goose-server` continue to pass with no behavior diff.
- `cargo tree -p goose-core` does not include `sqlx`, `tokio-cron-scheduler`, `oauth2`, `axum`, `posthog`, `keyring` in its default-feature build.
- `goose-embed` migrates to depend on `goose-core` (not `goose`) and its `cargo tree` shrinks accordingly. Public API of `goose-embed` is unchanged.
- All existing tests pass at every step of the series.
- `crates/goose` modules retained: `session/`, `scheduler*`, `oauth/`, `posthog`, `otel/`, `dictation/`, `download_manager/`, `doctor`, `platform_extensions/`, `goose_apps/`, `recipe_deeplink/`, `slash_commands/`, plus `Config` (full version).
- `crates/goose-core` gets: `agents/`, `providers/` (trait + all impls, feature-gated as today), `mcp_utils`, `skills/`, `recipe/` (the data model + execution, not file-discovery), `conversation/`, `context_mgmt/`, `permission/`, plus new `traits/` module for the extracted boundaries.

## Approaches Considered

### Approach A: Minimal carve, lift later ⭐ recommended
- **Sketch:** Extract the smallest traits needed to break the three boundary violations identified in the C-spike (`agents/agent.rs:52-53`, `extension_manager.rs:44`, `tool_execution.rs:60`). Move only what's listed under "Success criteria" above; nothing else. `Config` stays as a runtime singleton in `crates/goose`; `goose-core` types accept the values they need as explicit constructor params (Agent builds from an injected `AgentConfig`, providers from `ModelConfig`, etc.). No generalization beyond what's strictly required.
- **Affected files:** [crates/goose/src/agents/agent.rs](../../crates/goose/src/agents/agent.rs) (decoupling), [crates/goose/src/agents/extension_manager.rs](../../crates/goose/src/agents/extension_manager.rs), [crates/goose/src/agents/tool_execution.rs](../../crates/goose/src/agents/tool_execution.rs), [crates/goose/src/scheduler_trait.rs](../../crates/goose/src/scheduler_trait.rs) (already a trait, just moves), new `crates/goose-core/`, import rewrites in [crates/goose-cli](../../crates/goose-cli) and [crates/goose-server](../../crates/goose-server).
- **Tradeoffs:** Smallest review surface; fastest to ship; matches the brainstormed sequencing. Risk: each module move PR has to keep all consumers green, so import-rewrite churn is spread across many PRs. The new traits are designed for "just enough" and may need follow-up generalization later if more boundary problems appear.
- **Effort:** L (multi-PR series, but each PR is M).

### Approach B: Idiomatic split with full trait families
- **Sketch:** Same carve-out target, but design proper trait families upfront: a `Persistence` trait covering session storage (so embedders can swap in-memory / S3 / etc.), an `Auth` trait covering OAuth credential storage, a `Scheduler` trait (already exists, just formalize). Move `Config` into `goose-core` as a slim `GooseCoreConfig`; `crates/goose` extends with `Config { core: GooseCoreConfig, runtime: ... }`. Split `platform_extensions/` per file: leaf ones (`final_output_tool`) move to core, stateful ones (`chatrecall`, `todo`) stay.
- **Affected files:** Everything in Approach A, plus new `crates/goose-core/traits/persistence.rs`, `auth.rs`, the `Config` split, and per-file decisions on `platform_extensions/`.
- **Tradeoffs:** Cleaner long-term contract; embedders get pluggable persistence and auth out of the box. Costs: significantly more design discussion, more code to review, more places where the carve can stall on bikeshedding. Risk: the trait families end up over-engineered for a single embedder (`goose-embed`) and the second consumer reveals they're shaped wrong anyway.
- **Effort:** L+ (probably 2-3x A).

### Approach C: Stateless-only core, `Agent` stays in `goose`
- **Sketch:** Different scope entirely. `goose-core` contains only stateless primitives — `Provider` trait + impls, `Conversation`, `Recipe`, `Skill`, MCP utilities, `context_mgmt`, `permission`. `Agent` (which has all the boundary violations) stays in `crates/goose`. Embedders that want primitives without the loop depend on `goose-core`; embedders that want a full agent depend on `goose` directly (or `goose-embed`).
- **Affected files:** New `crates/goose-core/` with only the leaf modules. No trait extraction needed (because `Agent` isn't moving). No changes to `agents/`. `crates/goose-cli` and `crates/goose-server` keep their `goose` deps as-is.
- **Tradeoffs:** Drastically smaller carve — probably one M-sized PR instead of a series. No risky `Agent` decoupling. But `goose-core` is less useful — you can build agent *components* but not a working loop, so embedders still end up pulling `goose` for anything real. Defeats the C-spike's purpose: `goose-embed` would still need `goose` as a transitive dep.
- **Effort:** M.

## Recommendation

**Approach A.** It matches the pinned sequencing decision (two-phase, traits-first), keeps every PR reviewable, and actually delivers the dep-minimization win the C-spike's brainstorm demanded. Approach B is a tempting future state but the only consumer right now is `goose-embed`, so designing trait families for hypothetical second consumers is premature — let real usage drive that. Approach C is a fork in the road that abandons the original goal: if `goose-core` doesn't include `Agent`, embedders still depend on `goose`, and the C-spike's whole motivation evaporates.

The order matters too: do PR 1 (trait extraction inside `goose`, workspace still green, no new crate) before opening PR 2 (the physical move). PR 1 is the high-risk piece — if the trait shape is wrong, we fix it cheaply before any consumers are touching `goose-core`.

## Sequencing (concrete PR plan)

1. **PR 1 — Trait extraction inside `crates/goose`.** Add `crate::traits::{SessionContextProvider, OAuthProvider}` (and confirm `SchedulerTrait` stays as-is). Refactor `Agent`, `ExtensionManager`, `ToolCallContext` to use the new traits via injection. `crates/goose` provides the concrete impls. No new crate yet. Acceptance: `cargo test --workspace` passes; CLI/server/embed behavior unchanged.
2. **PR 2 — Stand up `crates/goose-core` skeleton.** Move leaf modules with no dependencies on `session`/`oauth`/`scheduler`: `conversation`, `mcp_utils`, `permission`, `context_mgmt`. `crates/goose` re-exports them so downstream imports keep working. Acceptance: `cargo tree -p goose-core` doesn't include `sqlx`, `oauth2`, `axum`.
3. **PR 3 — Move `providers/`, `skills/`, `recipe/`.** Preserve feature flags exactly. Same re-export pattern. Acceptance: feature matrix builds (`--no-default-features`, `--features aws-providers`, `--features local-inference`).
4. **PR 4 — Move `agents/` (the hard one).** Carries the trait extraction from PR 1. `crates/goose` keeps the concrete trait impls (`SessionContextProvider impl` over `SessionManager`, `OAuthProvider impl` over `oauth_flow`). Acceptance: `goose-cli session start` and `goose-server` HTTP routes work end-to-end.
5. **PR 5 — Re-point `goose-embed` at `goose-core`.** Drop the `goose` dep where possible. Acceptance: `cargo tree -p goose-embed` shrinks measurably and `cargo test -p goose-embed` still passes.

Each PR is independently mergeable and rollback-safe. If PR 3 stalls in review, PRs 1-2 are still useful.

## Open questions

- **`Config` singleton.** Stays in `crates/goose` per the recommendation. But `goose-core` types (Agent, providers) need values that today come from `Config::global()` — model overrides, mode, etc. Cleanest fix: every `goose-core` type that needs config takes it as an explicit constructor param. Existing `Agent::new()` keeps the singleton path; new public API uses `Agent::with_config()` exclusively. C-spike already does this — good signal it works.
- **`apply_recipe_components` lives on `Agent`.** Currently a one-liner that calls into `final_output_tool`. Moves with `agents/` in PR 4 — no special handling.
- **`hooks` module.** Used by `Agent::hook_manager` (loaded from CWD on construction). Either stays in `crates/goose` and gets injected via a trait, or moves to `goose-core`. Lean toward moving — it's pure machinery, no heavy deps. Confirm during PR 4.
- **`security/`** (`AdversaryInspector`, `EgressInspector`, `SecurityInspector`). Used by `Agent`. Probably moves with `agents/` since it's tightly coupled.
- **Feature flag naming.** `goose-core` should keep the same feature names as `goose` (`aws-providers`, `local-inference`, etc.) so the migration is transparent for consumers. Documented in PR 2.
- **`ExtensionConfig` definition location.** Lives in `agents/extension.rs` today. Moves to `goose-core` with `agents/`. But `Recipe::extensions: Option<Vec<ExtensionConfig>>` would then live in `goose-core::recipe`, which is fine — recipe and extension are both core concepts.
