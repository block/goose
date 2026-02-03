# AGENTS Instructions

goose is a **sophisticated enterprise AI agent framework** in Rust with CLI and Electron desktop interfaces, featuring advanced multi-agent orchestration, specialist agents, and enterprise workflow automation.

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
├── goose             # core logic with enterprise multi-agent platform
│   ├── agents/       # Enhanced agent architecture
│   │   ├── agent.rs           # Core Agent with ExecutionMode, planning, critique
│   │   ├── orchestrator.rs    # AgentOrchestrator for multi-agent coordination
│   │   ├── workflow_engine.rs # Enterprise workflow orchestration
│   │   ├── specialists/       # Specialist agent implementations
│   │   │   ├── code_agent.rs     # Code generation specialist
│   │   │   ├── test_agent.rs     # Testing and QA specialist
│   │   │   ├── deploy_agent.rs   # Deployment specialist
│   │   │   ├── docs_agent.rs     # Documentation specialist
│   │   │   └── security_agent.rs # Security analysis specialist
│   │   ├── persistence/       # LangGraph-style checkpointing
│   │   ├── reasoning.rs       # ReAct, CoT, ToT patterns
│   │   ├── reflexion.rs       # Self-improvement via verbal reinforcement
│   │   ├── critic.rs          # Self-critique system
│   │   ├── planner.rs         # Multi-step planning system
│   │   ├── state_graph/       # Self-correcting execution loops
│   │   ├── shell_guard.rs     # Security and approval system
│   │   └── done_gate.rs       # Task completion verification
│   ├── prompts/      # Advanced prompt engineering
│   │   ├── mod.rs             # PromptManager for pattern coordination
│   │   ├── patterns.rs        # 20+ reusable patterns (ReAct, CoT, etc.)
│   │   ├── templates.rs       # Template engine with variable validation
│   │   └── errors.rs          # Error types for prompt operations
│   ├── observability/# Token tracking, cost estimation, tracing
│   ├── policies/     # Rule engine and policy management
│   ├── guardrails/   # Safety constraints and validation
│   └── mcp_gateway/  # MCP protocol gateway
├── goose-bench       # benchmarking
├── goose-cli         # CLI entry with workflow management
├── goose-server      # backend (binary: goosed)
├── goose-mcp         # MCP extensions with security integration
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

## Phase 5 Enterprise Rules

Agent: Implement SpecialistAgent trait see specialists/mod.rs
Orchestrator: Use AgentOrchestrator for multi-agent coordination
Workflow: Create workflow templates in WorkflowEngine
Specialist: Each specialist agent handles specific domain (Code, Test, Deploy, Docs, Security)
Enterprise: Follow enterprise patterns for scalability and maintainability

## Code Quality

Comments: Write self-documenting code - prefer clear names over comments
Comments: Never add comments that restate what code does
Comments: Only comment for complex algorithms, non-obvious business logic, or "why" not "what"
Simplicity: Don't make things optional that don't need to be - the compiler will enforce
Simplicity: Booleans should default to false, not be optional
Errors: Don't add error context that doesn't add useful information (e.g., `.context("Failed to X")` when error already says it failed)
Simplicity: Avoid overly defensive code - trust Rust's type system
Logging: Clean up existing logs, don't add more unless for errors or security events

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
- Orchestrator: crates/goose/src/agents/orchestrator.rs
- WorkflowEngine: crates/goose/src/agents/workflow_engine.rs
- Specialists: crates/goose/src/agents/specialists/mod.rs
- Prompts: crates/goose/src/prompts/mod.rs
- Observability: crates/goose/src/observability/mod.rs
- Policies: crates/goose/src/policies/mod.rs
