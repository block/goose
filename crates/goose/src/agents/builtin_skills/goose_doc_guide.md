---
name: goose-doc-guide
description: Reference goose documentation to create, configure, or explain goose-specific features like recipes, extensions, sessions, and providers. ALWAYS fetch relevant docs before answering - never rely on training data for goose-specific syntax or commands.
---

Use this skill when working with **goose-specific features**:
- Creating or editing recipes
- Configuring extensions or providers
- Explaining how goose features work
- Any goose configuration or setup task

Do NOT use this skill for:
- General coding tasks unrelated to goose
- Running existing recipes (just run them directly)

## Steps

1. Fetch the doc map from `http://localhost:3000/goose/goose-docs-map.md`
2. Search the doc map for pages relevant to the user's topic
3. Use the EXACT paths from the doc map. For example:
   - If doc map shows: `docs/guides/sessions/session-management.md`
   - Fetch: `http://localhost:3000/goose/docs/guides/sessions/session-management.md`
   Do NOT modify or guess paths. You can make multiple fetch calls in parallel
4. Use the fetched documentation to help the user with their task

## Strict Requirements
1. You MUST fetch and verify documentation BEFORE providing any commands or syntax examples
2. If a user asks about feature X, do not explain how to use X until you've found it in the docs
3. Any "How to Use" or example commands MUST come directly from fetched documentation, not from your training data
4. NEVER add "How to Use", "Usage", or example CLI commands to your response unless you have fetched the specific documentation for that command
5. **STOP after completing the task. Do not add usage instructions, CLI examples, or "How to Use" sections unless the user explicitly asked for them.**
