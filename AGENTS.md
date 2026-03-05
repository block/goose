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
cargo clippy --all-targets -- -D warnings
./scripts/clippy-lint.sh      # full lint pass
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
├── goose             # core logic (agents, routing, orchestration, providers)
├── goose-bench       # benchmarking
├── goose-cli         # CLI entry point
├── goose-server      # backend REST server (binary: goosed)
├── goose-mcp         # MCP extension servers (developer, etc.)
├── goose-test        # test utilities
├── mcp-client        # MCP client library
├── mcp-core          # MCP shared types
└── mcp-server        # MCP server framework

temporal-service/     # Go scheduler
ui/desktop/           # Electron app (TypeScript)
```

## Architecture

### Agent System
- **Agent trait** (`crates/goose/src/agents/agent.rs`) — core abstraction for all agents
- **OrchestratorAgent** (`agents/orchestrator_agent.rs`) — LLM-based meta-coordinator, routes to specialists
- **IntentRouter** (`agents/intent_router.rs`) — keyword-based fast routing fallback
- **Dispatch** (`agents/dispatch.rs`) — executes sub-tasks via InProcess, A2A, or composite dispatchers
- **AgentPool** (`agents/agent_pool.rs`) — manages agent lifecycle and reuse
- Specialized agents: Developer, QA, PM, Security, Research (each in `agents/` subdir)

### Routing Flow
```
User message → IntentRouter (keywords, <10ms)
            → OrchestratorAgent (LLM classifier, ~1-5s)
            → dispatch_compound_dag (parallel DAG execution)
```

### Extension System
- Extensions are MCP servers (Model Context Protocol)
- Registered in `agents/extension.rs`, managed by `agents/extension_manager.rs`
- Extensions provide tools, resources, and prompts
- genui extension enables UI generation

### Prompt Templates
- Located in `crates/goose/src/prompts/` as `.md` files
- Rendered via `prompt_template.rs` using Handlebars (`{{variable}}`)
- Orchestrator prompts use XML-structured tags (Anthropic best practice)
- Agent-specific prompts in subdirectories (e.g., `prompts/developer/`, `prompts/orchestrator/`)

### A2A (Agent-to-Agent)
- Protocol for cross-agent communication (Google A2A, JSON-RPC 2.0 over HTTP)
- `DelegationStrategy::RemoteA2AAgent` in dispatch.rs
- Agent Cards for discovery, capability negotiation

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
Test: When adding features, update recipes/goose-self-test.yaml, rebuild, then run `goose run --recipe recipes/goose-self-test.yaml` to validate
Error: Use anyhow::Result
Provider: Implement Provider trait — see providers/base.rs
MCP: Extensions in crates/goose-mcp/
Server: Changes need just generate-openapi

## Code Quality

Comments: Write self-documenting code — prefer clear names over comments
Comments: Never add comments that restate what code does
Comments: Only comment for complex algorithms, non-obvious business logic, or "why" not "what"
Simplicity: Don't make things optional that don't need to be — the compiler will enforce
Simplicity: Booleans should default to false, not be optional
Errors: Don't add error context that doesn't add useful information
Simplicity: Avoid overly defensive code — trust Rust's type system
Logging: Clean up existing logs, don't add more unless for errors or security events

## Key Design Decisions

- **Supervisor pattern** for orchestration (OrchestratorAgent selects specialist agents)
- **MCP-first** extension architecture — all tools exposed via MCP protocol
- **A2A** for remote agent delegation — enables distributed multi-agent systems
- **XML-structured prompts** in orchestrator — follows Anthropic 2025 best practices
- **DAG-based parallel dispatch** — `dispatch_compound_dag` uses `FuturesUnordered` for concurrent sub-tasks
- **Feature flag** `GOOSE_ORCHESTRATOR_DISABLED` — falls back to keyword-only routing

## Never

Never: Edit ui/desktop/openapi.json manually
Never: Edit Cargo.toml — use `cargo add`
Never: Skip cargo fmt
Never: Merge without running clippy
Never: Comment self-evident operations, getters/setters, constructors, or standard Rust idioms

## Entry Points
- CLI: crates/goose-cli/src/main.rs
- Server: crates/goose-server/src/main.rs
- UI: ui/desktop/src/main.ts
- Agent: crates/goose/src/agents/agent.rs

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** — Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) — Tests, linters, builds
3. **Update issue status** — Close finished work, update in-progress items
4. **PUSH TO REMOTE** — This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** — Clear stashes, prune remote branches
6. **Verify** — All changes committed AND pushed
7. **Hand off** — Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing — that leaves work stranded locally
- NEVER say "ready to push when you are" — YOU must push
- If push fails, resolve and retry until it succeeds
