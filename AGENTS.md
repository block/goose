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
```

### UI
```bash
just generate-openapi        # after server changes
just run-ui                  # start desktop
cd ui/desktop && pnpm test   # test UI
```

### Git
```bash
git commit -s                # required for DCO sign-off
```

## Structure
```
crates/
├── goose              # core logic
├── goose-acp-macros   # ACP proc macros
├── goose-cli          # CLI entry
├── goose-server       # backend (binary: goosed)
├── goose-mcp          # MCP extensions
├── goose-test         # test utilities
└── goose-test-support # test helpers

evals/open-model-gym/  # benchmarking / evals
ui/desktop/            # Electron app
```

## Development Loop
```bash
# 1. source bin/activate-hermit
# 2. Make changes
# 3. cargo fmt
```

### Run these only if the user has asked you to build/test your changes:
```
# 1. cargo build
# 2. cargo test -p <crate>
# 3. cargo clippy --all-targets -- -D warnings
# 4. [if server] just generate-openapi
```

## Rules

- Test: Prefer tests/ folder, e.g. crates/goose/tests/
- Test: When adding features, update goose-self-test.yaml, rebuild, then run `goose run --recipe goose-self-test.yaml` to validate
- Error: Use anyhow::Result
- Provider: Implement Provider trait see providers/base.rs
- MCP: Extensions in crates/goose-mcp/
- Server: Changes need just generate-openapi

## Code Quality

- Comments: Write self-documenting code - prefer clear names over comments
- Comments: Never add comments that restate what code does
- Comments: Only comment for complex algorithms, non-obvious business logic, or "why" not "what"
- Simplicity: Don't make things optional that don't need to be - the compiler will enforce
- Simplicity: Booleans should default to false, not be optional
- Errors: Don't add error context that doesn't add useful information (e.g., `.context("Failed to X")` when error already says it failed)
- Simplicity: Avoid overly defensive code - trust Rust's type system
- Logging: Clean up existing logs, don't add more unless for errors or security events

## Ink / Terminal UI (ui/text)

- Ink renders React to a fixed character grid — not a browser. Content that exceeds a Box's dimensions is NOT clipped; it visually overflows into neighboring cells and breaks the layout.

- Ink-Text: Never use `wrap="wrap"` inside a fixed-height Box — wrapped text can exceed the Box height and bleed into adjacent components. Use `wrap="truncate"` and pre-truncate the string to fit the available character budget (lines × width).
  
- Ink-Layout: When changing card/cell dimensions, always recalculate how much content fits. Account for borders (2 chars), padding, margins, and sibling elements when computing the
remaining space for dynamic text.
  
- Ink-Overflow: Ink has no `overflow: hidden`. The only way to prevent overflow is to ensure content never exceeds the container size — truncate text, limit list items, or cap height.
  
- Ink-FlexGrow: Avoid `flexGrow={1}` on text containers inside fixed-height cards — the text will try to fill available space but Ink won't clip it if it exceeds the boundary.
  
- Ink-HeightBudget: When computing how many rows/items fit vertically, count EVERY line used by headers, footers, margins, borders, and scroll indicators. Under-reserving vertical space (e.g., `height - 8` when chrome actually uses 16 lines) causes Ink to squeeze out margins between items, making borders collapse. Always audit the actual line count.
  
- Ink-TrailingMargin: Don't apply `marginBottom` to the last item in a list — it wastes a line and can push content out of the container. Use conditional margins or container `gap`.

## Never

- Never: Edit ui/desktop/openapi.json manually
- Cargo.toml: For human-authored dependency changes, use `cargo add` instead of manually editing dependency entries unless there is a specific reason not to.
- Cargo.toml: Automated dependency bump PRs are exempt; when manual edits are necessary, keep `Cargo.lock` consistent.
- Never: Skip cargo fmt
- Never: Merge without running clippy
- Never: Comment self-evident operations (`// Initialize`, `// Return result`), getters/setters, constructors, or standard Rust idioms

## Entry Points
- CLI: crates/goose-cli/src/main.rs
- Server: crates/goose-server/src/main.rs
- UI: ui/desktop/src/main.ts
- Agent: crates/goose/src/agents/agent.rs

## Architecture

### Agent Reply Loop (`crates/goose/src/agents/agent.rs`)
`Agent::reply()` is the core loop: add user message → auto-compact if near token limit → call `provider.stream(system_prompt, messages, tools)` → parse response for tool calls → execute tools → append results → loop until no more tool calls or max turns reached. Tool calls fall into three categories:
- **Frontend tools** — returned to the UI for execution (file pickers, confirmations)
- **Extension tools** — dispatched via `ExtensionManager` to MCP servers
- **Platform tools** — executed locally (shell commands, with permission checks)

### Provider System (`crates/goose/src/providers/`)
New providers implement two traits:
- `Provider` — core interface: `stream(system, messages, tools) → MessageStream`
- `ProviderDef` — factory: `metadata() → ProviderMetadata`, `from_env() → Box<dyn Provider>`

Register in `providers/provider_registry.rs`. Configuration comes from env vars + `~/.goose/config.yaml` + system keyring (secrets).

### Extension / MCP System (`crates/goose/src/agents/extension*.rs`)
`ExtensionConfig` has three variants: `Stdio` (spawn external MCP subprocess), `Builtin` (built-in tools in `crates/goose-mcp/`), `Sse` (deprecated). `ExtensionManager` owns the MCP client lifecycle, translates MCP tool schemas into Goose tool definitions, and routes `call_tool` invocations.

### ACP Protocol (`crates/goose/src/acp/`)
Agent Client Protocol lets Goose delegate agent work to an external subprocess (e.g., Claude Code, Gemini CLI). `AcpProvider` spawns the subprocess, sends `prompt` requests, streams back response chunks, and handles bidirectional tool-call/result exchanges. Use this when a provider is itself an agentic system rather than a plain LLM.

### Session Persistence
`SessionManager` stores full conversation history in SQLite (`~/.goose/sessions/`). `context_mgmt/` handles auto-compaction: when a conversation approaches the model's context limit it summarizes older turns transparently and emits an `AgentEvent::HistoryReplaced`.

### Key Shared Types (`crates/goose/src/conversation/message.rs`)
`Message` and `MessageContent` are used everywhere — understand `ToolRequest`, `ToolResponse`, `Text`, and `Thinking` variants before touching provider or agent code.
