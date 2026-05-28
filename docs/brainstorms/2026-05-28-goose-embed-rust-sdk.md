# Embeddable Rust SDK for goose

> Brainstorm captured 2026-05-28. Source: `/brainstorm what about goose will expose an SDK that allow to write agents?`

## Clarified Problem Statement

**Goal:** Expose goose's core agent capabilities as an embeddable Rust crate so a third-party Rust program can `cargo add` it, plug in a provider + MCP extensions + skills + recipes, and run an agent loop — without shelling out to the `goose` binary or talking ACP over stdio.

**Constraints:**
- Rust-only at v1. TS/Python come later, via ACP if at all.
- Must not break `goose-cli`, `goose-server`, or the existing `goose acp` server mode.
- **Hard constraint — minimize the dependency surface exposed to embedders.** If `cargo add goose-sdk-thing` ends up pulling everything `crates/goose` pulls today (AWS SDK, candle, OAuth, sqlx, OTel, scheduler, dictation, download manager, …), the SDK isn't worth shipping — embedders may as well depend on `goose` directly. Dep minimization is the test of whether the SDK earns its existence.
- Today's `Agent` is coupled to a global `Config` singleton, an on-disk `SessionManager`, and a scheduler. The SDK must let embedders inject or override these (or accept lean defaults).
- `Recipe` and `Skill` are first-class concepts of the SDK — embedders should be able to load, compose, and run them from the public API, not just via file paths read by the CLI.
- `Config` is exposed and configurable from the embedder side — assume the embedder will want to tune at least provider, extensions, and permissioning.

**Non-goals:**
- Multi-language bindings at v1 — ACP already covers cross-language embedding via subprocess.
- A new "agent definition format" — reuse `Recipe` + `skills`.
- Replacing the existing `goose-sdk` crate (that one is an ACP *client* wrapper; this is an *embedding* SDK — different audience, different shape).
- A WASM/plugin runtime.

**Success criteria:**
- A ≤ 50-line `examples/embed.rs` that constructs a provider, registers an MCP extension, loads a recipe + skill, sends a prompt, and streams `AgentEvent`s — compiles against published crates.
- `cargo tree` on the example shows a meaningfully smaller dep graph than depending on `crates/goose` directly. Default features pull no AWS, no candle, no OTel, no scheduler.
- `Recipe` and `Skill` are reachable through `pub` types in the SDK's prelude — not through the runtime crate.
- `Config` is constructible by the embedder; no global singleton required to build an `Agent`.
- `goose-cli` is refactored to consume the same public API it ships (dogfooding — proves the surface is real).
- Public API has a documented semver contract; internals stay free to churn.

## Approaches Considered

### Approach A: Curated facade inside `crates/goose`
- **Sketch:** Add `goose::embed` (or `goose::prelude`) that re-exports a hand-picked stable subset: `Agent`, `AgentConfig`, `ExtensionConfig`, the `Provider` trait, MCP client helpers, `Recipe`, `skills::*`. Mark every other `pub mod` as `#[doc(hidden)]` or move under `pub(crate)`. Tighten cargo features so heavy deps are opt-in.
- **Affected files:** [crates/goose/src/lib.rs](../../crates/goose/src/lib.rs), new `crates/goose/src/embed/mod.rs`, [crates/goose/Cargo.toml](../../crates/goose/Cargo.toml) feature flags.
- **Tradeoffs:** Ships fast, no consumer migration. But embedders still depend on the full `goose` crate — **fails the dep-minimization constraint outright** unless we also aggressively gate every heavy module behind a default-off cargo feature, which is its own large refactor.
- **Effort:** M (but the only way to actually shrink deps from here is to repeat most of Approach B's work).

### Approach B: Carve out `goose-core` ⭐ recommended
- **Sketch:** Extract the embeddable primitives into a new lean crate `goose-core` — `agents::Agent`, `extension_manager`, `mcp_utils`, `providers` (trait + a minimal set of built-ins, the rest behind features), `conversation`, `context_mgmt`, `skills`, `recipe`. `crates/goose` becomes "application runtime" (on-disk sessions, scheduler, recipes-from-disk, telemetry, doctor, OAuth flows, dictation, download manager) and depends on `goose-core`. `goose-cli` and `goose-server` keep working unchanged; their imports shift downward where appropriate.
- **Affected files:** new `crates/goose-core/`, refactor in [crates/goose/src/lib.rs](../../crates/goose/src/lib.rs), import rewrites across [crates/goose-cli/](../../crates/goose-cli/) and [crates/goose-server/](../../crates/goose-server/). `Config` singleton becomes an injected handle in `goose-core` (the singleton can stay in the runtime crate as a thin wrapper).
- **Tradeoffs:** The only approach that actually delivers on the dep-minimization constraint. Real public-API boundary enforced by the crate split, sets long-term direction, semver contract becomes credible. Costs: big refactor; the `Config` and `SessionManager` singletons reach far and need to become injectable; circular-dep risk during the transition; the carve-out forces some interface decisions under time pressure.
- **Effort:** L

### Approach C: New `goose-embed` crate on top of `goose`
- **Sketch:** Pure additive. New crate `goose-embed` depends on `goose` and ships ergonomic builders — `Goose::builder().provider(p).extension(cfg).recipe(r).skill(s).build()` returns a small handle that exposes `reply(prompt) -> Stream<AgentEvent>`. No changes to `crates/goose` internals. The crate hides the `Config`/`SessionManager` plumbing behind sensible defaults (in-memory session store, no scheduler).
- **Affected files:** new `crates/goose-embed/` with `src/lib.rs`, `examples/`, README. Optional thin patches in `crates/goose` only if the in-memory `SessionManager` doesn't already exist.
- **Tradeoffs:** Ships in days, validates the API shape with real users before committing to a refactor, zero risk to existing consumers. **But it fails the dep-minimization constraint** — embedders pull the whole `goose` crate transitively, so the SDK has no weight advantage over just depending on `goose` and writing a thin builder yourself.
- **Effort:** S/M
- **Best use:** as a *prototyping* spike that's explicitly thrown away once Approach B lands. Use it to discover the API shape (which builders, which defaults, which callbacks for permissions), then re-emerge the same surface from `goose-core` in B.

## Recommendation

**Do Approach B.** The new dep-minimization constraint is decisive: A and C both keep embedders bolted to the full `crates/goose` dependency tree, which defeats the point of having an SDK at all. Only the crate carve-out actually makes `cargo add goose-core` lighter than `cargo add goose`.

Sensible sequencing:

1. Spike Approach C in a branch for ~1 week (don't merge) to nail the public API ergonomics with a real example — what's the builder shape, how do permissions get handled headlessly, how do recipes load, etc.
2. Use that API surface as the *contract* for `goose-core`. Carve out per Approach B, lifting the surface as-validated.
3. Throw away the C spike when B lands.
4. Refactor `goose-cli` to consume the public API. If it can't, the API isn't done.

Skip A entirely — it's a refactor of `crates/goose` with no payoff in dep weight.

## Decisions captured from clarifying round

- **Recipes & Skills:** first-class in the SDK. Public types reachable through the prelude, not just file-path loading.
- **Config:** exposed and configurable from the embedder. No global singleton required to construct an `Agent`.
- **ACP server export:** undecided. Leave out of v1 to keep `goose-core` lean; embedders that want to re-expose their composed agent over ACP can depend on a future `goose-acp-server` crate that bridges `goose-core` → ACP. Revisit once a real embedder asks for it.
- **Headless permissions:** not yet decided. The spike (C) should drive the answer — likely a callback trait the embedder implements, or an `async` channel.

## Open questions

- Where exactly does the line fall between `goose-core` and runtime-only modules? First-pass candidates for **runtime** (stay in `crates/goose`): `scheduler*`, `session::*` disk backend, `doctor`, `download_manager`, `dictation`, `oauth`, `posthog`, `otel`, `goose_apps`, `recipe_deeplink`, `slash_commands`. **Core** candidates: `agents`, `conversation`, `context_mgmt`, `providers` trait + minimal built-ins, `mcp_utils`, `skills`, `recipe` (definition + execution, not file discovery), `permission` (trait + types), `tool_inspection`, `tool_monitor`. Needs a per-module dep audit before the carve.
- The `Provider` trait pulls in `reqwest`, OAuth, AWS, candle depending on which providers are enabled. Need to decide: do all built-in providers live in `goose-core`, or only the trait + a single reference impl (e.g. OpenAI-compatible), with each vendor in its own `goose-provider-*` crate?
- How do we keep `goose-cli` and `goose-server` from accidentally bypassing the public API and reaching into `goose-core` internals during the migration? Consider a `#[deny(missing_docs)]` + `pub use` prelude pattern, or feature-gated re-exports.
- Versioning: does `goose-core` track `goose`'s version, or get its own semver track? Independent track is more honest but means coordinating two releases.
