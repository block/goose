# AGENTS Instructions

goose is an AI agent framework in Rust with CLI and Electron desktop interfaces.

## Setup
```bash
source bin/activate-hermit
cargo build
```

## Commands

### Build
```bash
cargo build                   # debug
cargo build --release         # release  
just release-binary           # release + openapi
```

### Test
```bash
cargo test                   # all tests
cargo test -p goose          # specific crate
cargo test --package goose --test mcp_integration_test
just record-mcp-tests        # record MCP
```

### Lint/Format
```bash
cargo fmt
./scripts/clippy-lint.sh
cargo clippy --fix
```

### UI
```bash
just generate-openapi        # after server changes
just run-ui                  # start desktop
cd ui/desktop && npm test    # test UI
```

## Structure
```
crates/
├── goose             # core logic
├── goose-bench       # benchmarking
├── goose-cli         # CLI entry
├── goose-server      # backend (binary: goosed)
├── goose-mcp         # MCP extensions
├── goose-test        # test utilities
├── mcp-client        # MCP client
├── mcp-core          # MCP shared
└── mcp-server        # MCP server

temporal-service/     # Go scheduler
ui/desktop/           # Electron app
```

## Development Loop
```bash
# 1. source bin/activate-hermit
# 2. Make changes
# 3. cargo fmt
# 4. cargo build
# 5. cargo test -p <crate>
# 6. ./scripts/clippy-lint.sh
# 7. [if server] just generate-openapi
```

## Rules

Test: Prefer tests/ folder, e.g. crates/goose/tests/
Test: When adding features, update goose-self-test.yaml, rebuild, then run `goose run --recipe goose-self-test.yaml` to validate
Error: Use anyhow::Result
Provider: Implement Provider trait see providers/base.rs
MCP: Extensions in crates/goose-mcp/
Server: Changes need just generate-openapi

## Never

Never: Edit ui/desktop/openapi.json manually
Never: Edit Cargo.toml use cargo add
Never: Skip cargo fmt
Never: Merge without ./scripts/clippy-lint.sh
Never: Comment self-evident operations (`// Initialize`, `// Return result`), getters/setters, constructors, or standard Rust idioms

## Entry Points
- CLI: crates/goose-cli/src/main.rs
- Server: crates/goose-server/src/main.rs
- UI: ui/desktop/src/main.ts
- Agent: crates/goose/src/agents/agent.rs
