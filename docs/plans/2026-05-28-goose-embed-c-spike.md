# Plan: goose-embed C-spike

> Generated 2026-05-28 from `/ship --from-brainstorm docs/brainstorms/2026-05-28-goose-embed-rust-sdk.md`.
> Related: [brainstorm](../brainstorms/2026-05-28-goose-embed-rust-sdk.md).

## Goal

Ship a new `crates/goose-embed` crate that exposes an ergonomic builder API for programmatically constructing and driving a goose `Agent` from a plain Rust program. This is the **API-discovery spike** the brainstorm calls for as step 1 — it does NOT carve out `goose-core` (that's the follow-up). The spike validates the public surface against a real working example, so the eventual `goose-core` knows what it must export.

## Affected files

All new except the workspace root `Cargo.toml`:

- `crates/goose-embed/Cargo.toml` — new workspace crate; depends on `goose`, `tokio`, `futures`, `anyhow`, `async-trait`
- `crates/goose-embed/src/lib.rs` — `Goose` handle, `Goose::builder()` entry, prelude re-exporting `AgentEvent`, `Message`, `ExtensionConfig`, `Recipe`, skill types from `goose`
- `crates/goose-embed/src/builder.rs` — `GooseBuilder` with `.provider()`, `.extension()`, `.recipe()`, `.working_dir()`, `.permission_decider()`, `.build()`
- `crates/goose-embed/src/permission.rs` — `PermissionDecider` trait + `AutoApprove` default impl; bridges into goose's `ToolConfirmationRouter` via a background task
- `crates/goose-embed/examples/hello_agent.rs` — minimal end-to-end: provider → prompt → stream `AgentEvent`s to stdout
- `crates/goose-embed/examples/with_recipe.rs` — loads a recipe from disk and runs it (proves Recipe is first-class)
- `crates/goose-embed/README.md` — what it is, what it explicitly is NOT (no dep minimization in v0), and the roadmap to `goose-core`
- `Cargo.toml` (workspace root) — add `crates/goose-embed` to `members`

## Approach

1. Stand up the new crate and a `Goose::builder()` facade. The builder constructs an `AgentConfig` from explicit pieces (provider name + model resolved via `goose::providers::create_with_named_model`, extension list, permission decider, working dir), then calls `Agent::with_config()` — avoiding the `Agent::new()` path that touches `Config::global()`. Where singletons remain (`SessionManager`, `PermissionManager`), v0 uses `.instance()` internally and documents this as a known limitation in the README.
2. Wrap the existing `Agent::reply()` stream in a thin `Goose::reply(prompt) -> impl Stream<Item = AgentEvent>` that handles `SessionConfig` construction and prompt-to-`Message` conversion.
3. Implement `PermissionDecider` as a trait inside `goose-embed`. v0 wires it via a background task that polls/responds to `Agent::handle_confirmation()`. If a wall is hit and a tiny `pub` patch to `crates/goose` is needed, do it in the same PR; otherwise zero edits to `crates/goose`.
4. Two examples: `hello_agent.rs` (provider only, prints streamed text) and `with_recipe.rs` (loads a YAML recipe, runs it).
5. README clearly frames this as a spike: depends on the full `goose` crate, doesn't deliver dep minimization, will be re-emerged from `goose-core` later.

## Edge cases

- **API keys for the example:** don't require any at compile time. Read provider + model from env vars (`GOOSE_EMBED_PROVIDER`, `GOOSE_EMBED_MODEL`) with sensible defaults; if env is missing, the example prints a friendly hint and exits 0 so CI stays green.
- **Session pollution:** `SessionManager::instance()` writes to `~/.local/share/goose/sessions/sessions.db`. v0 uses `SessionType::Hidden` so embed sessions don't show up in user-visible session lists.
- **Stream lifetime:** `Agent::reply()` returns `BoxStream<'_, Result<AgentEvent>>` borrowing `&self`. The `Goose` handle must own the `Agent` so the stream is usable from typical async code.
- **Async trait:** `PermissionDecider` uses `async_trait` (or AFIT if stable in workspace MSRV). Must use the same `async_trait` version `crates/goose` uses to avoid double-include.
- **Tokio runtime:** library is runtime-agnostic in surface, but `goose::Agent` is tokio-based, so embedders must run a tokio runtime. Examples use `#[tokio::main]`.

## Test plan

- `cargo build -p goose-embed` — compiles in workspace
- `cargo build -p goose-embed --examples` — both examples compile
- `cargo clippy -p goose-embed --all-targets -- -D warnings` — clean
- `cargo fmt` — clean
- `cargo test -p goose-embed` — smoke test asserting the builder rejects missing-provider configurations; no live network tests in CI
- **Manual sign-off before push:** run `cargo run -p goose-embed --example hello_agent` against a real provider locally

## Conventions to follow

From [AGENTS.md](../../AGENTS.md):

- DCO sign-off on commits (`git commit -s`)
- Use `cargo add` for new dependencies, not manual `Cargo.toml` edits
- `anyhow::Result` for fallible functions
- Self-documenting code; no comments restating what the code does
- No comments on getters / setters / constructors / standard Rust idioms
- Run `cargo fmt` always; `cargo clippy --all-targets -- -D warnings` before commit
- Tests in `crates/goose-embed/tests/` not `src/`
- Don't make things `Option` that don't need to be; booleans default to `false`

## Open questions / risks

- **Recipe + Skill surface depth:** if the public types from `goose::skills` turn out to be too thin to re-export cleanly, that's a finding to document for the eventual `goose-core` carve-out — not a blocker for this spike.
- **`crates/goose` patches:** the goal is zero blast radius. If a single targeted `pub` exposure is needed, keep it minimal and in the same PR. If multiple patches start accumulating, stop and reassess.
- **CI workflow re-discovery:** adding a new workspace member may trigger CI to start running checks on it. Verify locally first.

## Estimated size

**M** — ~400-600 LOC across 7 new files plus a 1-line workspace `Cargo.toml` edit. Purely additive; no production breakage.

## Out of scope (explicit)

- Creating `goose-core` (Approach B — separate PR series)
- Minimizing dependency weight (embedders still inherit all of `crates/goose`'s deps; acknowledged in README)
- Refactoring `Agent::with_config()` or singleton wiring inside `crates/goose`
- Multi-language bindings (Rust v1 only)
- Exposing the embedded agent as an ACP server (deferred per brainstorm)
