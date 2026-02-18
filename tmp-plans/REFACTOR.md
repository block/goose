# Light refactor: `local_inference.rs` — extract `stream` sub-paths

## Problem

`local_inference.rs` is 2320 lines. The `stream` method alone is ~580 lines (1442–2020), split into two labelled sections:

- **Emulator path** (lines 1681–1786): tiny/small models that emulate tool calling
- **Native tool-calling path** (lines 1788–2018): medium/large models with native tool support

These are essentially independent code paths selected by an `if use_emulator` branch. The banner comments exist because the function is too long to follow otherwise.

## Proposed change

Extract each path into its own private helper function:

1. **`run_emulator_path(...)`** — takes the loaded model, chat messages, prompt, tools, emulator state, tx sender, etc. Returns nothing (sends results via `tx`). Roughly lines 1681–1786.

2. **`run_native_tool_path(...)`** — takes the loaded model, chat messages, tools json, context/memory params, tx sender, etc. Roughly lines 1788–2018.

The `stream` method keeps the shared setup (model loading, message preparation, context validation) and dispatches to one of the two helpers.

## What this does NOT include

- No module splitting — everything stays in `local_inference.rs` for now
- No changes to the free-standing helper functions (they're already well-factored)
- No logic changes — pure mechanical extraction
- No changes to tests

## Expected outcome

- `stream` shrinks from ~580 lines to ~240 lines (setup + dispatch)
- Each extracted function is ~100–170 lines — readable without section banners
- The `// === ... ===` comments can be removed
- No behaviour change; existing tests continue to pass
