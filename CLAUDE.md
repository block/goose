# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Setup
```bash
source bin/activate-hermit    # Hermit environment setup
cargo build                   # Debug build
cargo build --release         # Release build
just release-binary           # Release build + OpenAPI generation
```

### Testing
```bash
cargo test                               # All tests
cargo test -p goose                      # Specific crate tests
cargo test -p <crate-name>               # Any specific crate
cargo test --package goose --test mcp_integration_test  # MCP integration tests
just record-mcp-tests                    # Record MCP test scenarios
just build-test-tools                    # Build test utilities
```

### Linting & Formatting
```bash
cargo fmt                     # Format code
./scripts/clippy-lint.sh      # Run all clippy checks (strict + baseline)
cargo clippy --fix           # Auto-fix clippy issues
cargo clippy --jobs 2 -- -D warnings  # Standard clippy (strict)
```

### UI Development
```bash
just generate-openapi         # Generate OpenAPI schema (required after server changes)
just run-ui                   # Build and run desktop app
just run-ui-only              # Run UI without rebuilding binaries
just debug-ui                 # Run UI in external backend mode
just debug-ui-main-process    # Debug main process (see chrome://inspect)
just run-ui-alpha             # Run with alpha features (Temporal scheduler)
just run-ui-alpha-legacy      # Run with legacy scheduler
just make-ui                  # Build desktop bundle
just lint-ui                  # Lint UI code
cd ui/desktop && npm test     # Run UI tests
cd ui/desktop && npm run generate-api  # Generate frontend API
```

### Server Operations
```bash
just run-server               # Start goose server
just start-temporal           # Start Temporal services
just stop-temporal            # Stop Temporal services
just status-temporal          # Check Temporal status
```

### Release Management
```bash
just prepare-release <version>  # Create release branch and bump versions
just tag                       # Create git tag from Cargo.toml version
just tag-push                  # Tag and push to trigger CI release
just release-notes <old-tag>   # Generate release notes
```

## Architecture

### Core Structure
```
crates/
├── goose/           # Core agent logic, Provider trait implementations
├── goose-cli/       # CLI entry point (main.rs)
├── goose-server/    # Backend server (main.rs, generates OpenAPI)
├── goose-mcp/       # MCP (Model Context Protocol) extensions
├── goose-bench/     # Performance benchmarking tools
├── goose-test/      # Shared testing utilities
├── mcp-client/      # MCP client implementation
├── mcp-core/        # MCP shared components
└── mcp-server/      # MCP server implementation

temporal-service/    # Go-based task scheduler
ui/desktop/         # Electron desktop application
```

### Key Entry Points
- **CLI**: `crates/goose-cli/src/main.rs`
- **Server**: `crates/goose-server/src/main.rs`
- **Desktop UI**: `ui/desktop/src/main.ts`
- **Agent Core**: `crates/goose/src/agents/agent.rs`

### Provider System
Goose uses a Provider trait pattern for LLM integrations. New providers should:
- Implement the Provider trait (see `providers/base.rs`)
- Handle different model configurations for multi-model optimization
- Support both streaming and non-streaming responses

### MCP Integration
Model Context Protocol extensions live in `crates/goose-mcp/`. MCP allows goose to:
- Connect to external tools and services
- Extend capabilities beyond built-in functionality
- Integrate with various development environments

### UI Architecture
The Electron desktop app provides a modern interface for goose interactions:
- Frontend generates API clients from OpenAPI schema
- Backend runs the goose-server binary
- Supports both alpha features and stable releases
- Debug modes available for development

## Development Workflow

1. **Environment Setup**: `source bin/activate-hermit`
2. **Make Changes**: Edit code in appropriate crate
3. **Format**: `cargo fmt` (never skip this)
4. **Build**: `cargo build` or `cargo build --release`
5. **Test**: `cargo test -p <crate-name>`
6. **Lint**: `./scripts/clippy-lint.sh` (required before merge)
7. **Server Changes**: Run `just generate-openapi` to update API schema

## Critical Rules

### Code Quality
- **Always** run `cargo fmt` before committing
- **Always** run `./scripts/clippy-lint.sh` before merging
- Use `anyhow::Result` for error handling
- Prefer `tests/` folder for test files (e.g., `crates/goose/tests/`)

### Dependencies
- Use `cargo add` instead of manually editing Cargo.toml
- Never edit `ui/desktop/openapi.json` manually (auto-generated)

### Server Development
- Server changes require running `just generate-openapi`
- OpenAPI schema drives frontend API generation
- Test API changes with `just run-ui` before committing

### MCP Development
- MCP extensions go in `crates/goose-mcp/`
- Record test scenarios with `just record-mcp-tests`
- Test MCP integrations thoroughly as they affect external tool compatibility

## Multi-Platform Support

Goose supports multiple platforms with specialized build commands:
- **Windows**: `just release-windows`, `just make-ui-windows`
- **Intel Mac**: `just release-intel`, `just make-ui-intel`
- **Linux/Default**: Standard cargo and just commands

Cross-compilation uses Docker for consistent builds across platforms.