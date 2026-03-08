# Prompt Templates

Handlebars `.md` templates rendered at runtime via `prompt_template.rs`.

## Conventions
- **XML-structured tags** for complex prompts (Anthropic 2025 best practice)
- Template variables use `{{variable_name}}` syntax
- Agent-specific prompts live in subdirectories matching agent names
- System prompts define role + rules; task prompts define specific instructions

## Orchestrator Prompts (`orchestrator/`)
- `system.md` — Meta-coordinator system prompt with `{{agent_catalog}}`
- `routing.md` — Single-intent routing classifier
- `splitting.md` — Compound request splitter (primary LLM classifier)

## Template Variables
- `{{user_message}}` — The user's input message
- `{{agent_catalog}}` — XML-formatted agent/mode catalog from `build_catalog_text()`

## Testing
Prompt rendering is tested in `orchestrator_agent::tests`. When modifying prompts:
1. Ensure template variables match the Rust context struct (e.g., `RoutingPromptContext`)
2. Run `cargo test -p goose --lib orchestrator_agent`
3. Verify JSON output format matches `parse_splitting_response()` expectations
