# Architecture

goose is an AI agent framework. The `goose` crate contains all core logic; `goose-cli` and `goose-server` are thin interface layers that call into it.

## Where Things Live

| To work on... | Look here | What it is |
|---------------|-----------|------------|
| Agent orchestration | `crates/goose/src/agents/agent.rs` | Conversation loop, tool dispatch, event streaming |
| LLM providers | `crates/goose/src/providers/` | Provider trait + 20+ implementations |
| Extension system | `crates/goose/src/agents/extension_manager.rs` | MCP client lifecycle, tool routing |
| Session persistence | `crates/goose/src/session/` | SQLite storage, conversation history |
| Recipes | `crates/goose/src/recipe/` | YAML/JSON task definitions |
| Permission checking | `crates/goose/src/permission/` | Tool approval (Auto/Approve/SmartApprove/Chat) |
| Configuration | `crates/goose/src/config/` | Global settings, secrets, GooseMode |
| Context/token management | `crates/goose/src/context_mgmt/` | Token counting, conversation compaction |
| Hints (.goosehints) | `crates/goose/src/hints/` | Project context loading |
| Scheduling | `crates/goose/src/scheduler.rs` | Cron-based recipe execution |
| Subagents | `crates/goose/src/agents/subagent_tool.rs` | Task delegation to child agents |
| CLI commands | `crates/goose-cli/src/cli.rs` | Clap commands, session builder |
| Server routes | `crates/goose-server/src/routes/` | Axum HTTP API for desktop |
| Built-in MCP tools | `crates/goose-mcp/src/` | developer, memory, tutorial, and more |
| Desktop UI | `ui/desktop/` | Electron + React frontend |

## Core Abstractions

### Agent
The central orchestrator (`agents/agent.rs`). Manages the conversation loop: sends messages to the LLM provider, receives responses, dispatches tool calls to extensions, handles retries, and yields events. The `reply()` method is the main entry point—it returns a stream of `AgentEvent`s (messages, notifications, model changes).

### Provider
The LLM abstraction (`providers/base.rs`). The `Provider` trait requires `complete_with_model()` for generating responses. There are 20+ implementations (OpenAI, Anthropic, Google, Azure, Bedrock, Ollama, etc.). Providers are created via `providers::create()` which uses a registry pattern.

### ExtensionManager
Manages MCP extensions (`agents/extension_manager.rs`). Handles extension lifecycle (add/remove), routes tool calls to the correct MCP client, and aggregates tools/resources/prompts from all extensions. Tools are namespaced as `{extension}__{tool}` (e.g., `developer__shell`).

### Session
Conversation persistence (`session/`). `SessionManager` is a static facade over SQLite storage. Sessions contain message history, token counts, extension state, and metadata. Session IDs follow the format `YYYYMMDD_N`. Types: `User`, `Scheduled`, `SubAgent`, `Hidden`, `Terminal`.

### Recipe
Declarative task definitions (`recipe/`). YAML/JSON files with instructions, extensions, parameters, and optional JSON schema for structured output. Supports sub-recipes for composition via the `subagent` tool. Parameters use MiniJinja templating.

## Extension Types

| Type | Transport | Use Case |
|------|-----------|----------|
| Stdio | Subprocess (stdin/stdout) | Local MCP servers |
| SSE | HTTP Server-Sent Events | Remote MCP servers |
| StreamableHttp | HTTP streaming | Remote MCP servers |
| Builtin | In-process child | Bundled goose-mcp servers |
| Platform | In-process direct | Agent-aware tools (todo, skills, extensionmanager) |
| Frontend | UI callback | Browser-executed tools |
| InlinePython | Python via uvx | Python code execution |

## Key Entry Points

- **Message handling**: `Agent::reply()` in `agents/agent.rs`
- **Tool dispatch**: `Agent::dispatch_tool_call()` in `agents/agent.rs`
- **Provider trait**: `trait Provider` in `providers/base.rs`
- **Extension loading**: `ExtensionManager::add_extension()` in `agents/extension_manager.rs`
- **Permission checking**: `check_tool_permissions()` in `permission/permission_judge.rs`
- **CLI entry**: `main()` in `goose-cli/src/main.rs` → `cli()` in `cli.rs`
- **Server entry**: `main()` in `goose-server/src/main.rs` (binary: `goosed`)
