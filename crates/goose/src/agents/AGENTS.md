# Agents Subsystem

This directory contains the core agent system for Goose.

## Architecture

### Routing Flow
```
User message
  → IntentRouter.route()     — keyword matching (<10ms)
  → OrchestratorAgent.route() — LLM classifier via splitting.md prompt
  → dispatch_compound_dag()  — parallel DAG execution via FuturesUnordered
```

### Key Files
- `agent.rs` — Agent trait, AgentConfig, reply loop
- `orchestrator_agent.rs` — LLM-based routing + compound splitting
- `intent_router.rs` — Keyword-based fast routing fallback
- `dispatch.rs` — Task execution: InProcess, A2A, Composite dispatchers
- `agent_pool.rs` — Agent lifecycle management
- `extension_manager.rs` — MCP extension registration + tool routing
- `extension.rs` — Extension trait and types

### Adding a New Agent
1. Create a new directory under `agents/` (e.g., `agents/my_agent/`)
2. Implement the `Agent` trait from `agent.rs`
3. Register in `crates/goose/src/registry/` with a manifest (modes, tools, keywords)
4. Add prompt templates in `crates/goose/src/prompts/my_agent/`
5. The IntentRouter and OrchestratorAgent will auto-discover it from the registry

### Prompt Templates
- Located in `../prompts/orchestrator/` for orchestrator prompts
- Use XML-structured tags (Anthropic best practice)
- Rendered via Handlebars: `{{user_message}}`, `{{agent_catalog}}`
- Test prompt rendering in orchestrator_agent::tests

### Testing
```bash
cargo test -p goose --lib orchestrator_agent  # 24 tests
cargo test -p goose --lib intent_router       # 13 tests
cargo test -p goose --lib dispatch            # dispatch tests
```

### Design Decisions
- Orchestrator uses **splitting.md** (not routing.md) for LLM classification — supports compound requests
- `dispatch_compound_dag` handles parallel execution with dependency-aware scheduling
- A2A dispatch via `DelegationStrategy::RemoteA2AAgent` for remote agent calls
- Internal modes (`is_internal: true`) are filtered from routing prompts and user-facing catalogs
