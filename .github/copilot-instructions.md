# GitHub Copilot Code Review Instructions

## Review Philosophy
- Only comment when you have HIGH CONFIDENCE (>80%) that an issue exists
- Be concise: one sentence per comment when possible
- Focus on actionable feedback, not observations
- Skip comments on style that clippy/rustfmt will catch

## Priority Areas (Review These)

### Security & Safety
- Unsafe code blocks without justification
- Command injection risks (shell commands, user input)
- Path traversal vulnerabilities
- Credential exposure or hardcoded secrets
- Missing input validation on external data
- Improper error handling that could leak sensitive info

### Correctness Issues
- Logic errors that could cause panics or incorrect behavior
- Race conditions in async code
- Resource leaks (files, connections, memory)
- Off-by-one errors or boundary conditions
- Incorrect error propagation (using `unwrap()` inappropriately)
- Optional types that don't need to be optional
- Booleans that should default to false but are set as optional
- Error context that doesn't add useful information (e.g., `.context("Failed to do X")` when error already says it failed)
- Overly defensive code that adds unnecessary checks
- Unnecessary comments that just restate what the code already shows (remove them)

### Architecture & Patterns
- Code that violates existing patterns in the codebase
- Missing error handling (should use `anyhow::Result`)
- Async/await misuse or blocking operations in async contexts
- Improper trait implementations

## Skip These (Low Value)

- Style issues (rustfmt handles this)
- Clippy warnings (CI catches these)
- Minor naming suggestions unless truly confusing
- Obvious code that doesn't need explanation
- Suggestions to add comments for self-documenting code
- Refactoring suggestions unless there's a clear bug or maintainability issue
- Listing multiple potential issues in one comment (choose the single most critical issue)
- Suggestions to add logging statements - unless for errors or security events (the codebase needs less logging, not more)


## Response Format

When you identify an issue:
1. **State the problem** (1 sentence)
2. **Why it matters** (1 sentence, only if not obvious)
3. **Suggested fix** (code snippet or specific action)

Example:
```
This could panic if the vector is empty. Consider using `.get(0)` or add a length check.
```

## Project-Specific Context

- This is a Rust project using cargo workspaces
- Core crates: `goose` (agent logic), `goose-cli` (CLI), `goose-server` (backend), `goose-mcp` (MCP servers)
- Error handling: Use `anyhow::Result`, not `unwrap()` in production code
- Async runtime: tokio
- All code must pass: `cargo fmt`, `./scripts/clippy-lint.sh`, and tests
- See HOWTOAI.md for AI-assisted code standards
- MCP protocol implementations require extra scrutiny

## When to Stay Silent

If you're uncertain whether something is an issue, don't comment. False positives create noise and reduce trust in the review process.
