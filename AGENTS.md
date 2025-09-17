# Repository Guidelines

Goose is an AI agent framework in Rust with CLI and Electron desktop apps. This guide summarizes how to build, test, and contribute effectively.

## Project Structure & Module Organization
- Rust workspace in `crates/`: `goose` (core), `goose-cli` (CLI), `goose-server` (binary: `goosed`), `goose-mcp`, `goose-bench`, `goose-test`, `mcp-core`, `mcp-client`, `mcp-server`.
- UI: `ui/desktop/` (Electron). Scheduler: `temporal-service/` (Go).
- Tests live per-crate under `tests/` (e.g., `crates/goose/tests/`).
- Entry points: `crates/goose-cli/src/main.rs`, `crates/goose-server/src/main.rs`, `ui/desktop/src/main.ts`, `crates/goose/src/agents/agent.rs`.

## Build, Test, and Development Commands
- Setup: `source bin/activate-hermit`.
- Build: `cargo build` (debug), `cargo build --release`, `just release-binary` (release + OpenAPI).
- Test: `cargo test`, `cargo test -p goose`, `cargo test --package goose --test mcp_integration_test`, `just record-mcp-tests` (record MCP).
- Lint/Format: `cargo fmt`, `./scripts/clippy-lint.sh`, `cargo clippy --fix`.
- UI: `just generate-openapi` after server changes, `just run-ui`, `cd ui/desktop && npm test`.

## Coding Style & Naming Conventions
- Rust formatting is mandatory: run `cargo fmt` before commits.
- Lint with `clippy` and fix warnings; no new warnings in PRs.
- Errors use `anyhow::Result`.
- Modules/files use `snake_case`. Providers implement `Provider` trait (see `crates/goose/src/providers/base.rs`).

## Testing Guidelines
- Prefer integration tests under `crates/<crate>/tests/`; keep unit tests close to code.
- Name tests descriptively; keep tests fast by default.
- Use package filters (`-p <crate>`) to iterate quickly; record MCP behavior with `just record-mcp-tests`.

## Commit & Pull Request Guidelines
- Use Conventional Commits: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`.
- PRs include a clear summary, linked issues, test plan, and screenshots for UI changes.
- Before opening: run `cargo fmt`, `./scripts/clippy-lint.sh`, relevant tests; regenerate OpenAPI if server changed.

## Never
- Do not edit `ui/desktop/openapi.json` manually — run `just generate-openapi`.
- Do not edit `Cargo.toml` by hand — use `cargo add`.
- Do not skip formatting or lint checks.
