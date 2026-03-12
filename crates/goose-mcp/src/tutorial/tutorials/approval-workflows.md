Build an approval-gated MCP workflow using elicitation so goose can ask the user for explicit confirmation in Desktop and CLI.

This tutorial shows how to build a small MCP server that pauses for approval before continuing a workflow. The key primitive is **elicitation**: instead of encoding approval logic into a recipe, your MCP server can ask goose to collect structured user input.

## Why this pattern

Approval-gated workflows are useful when an agent should inspect or prepare an action first, then wait for a human decision before proceeding. Typical examples include:

- reviewing a risky command before execution
- confirming a release signoff decision
- requiring explicit go/no-go input before a destructive action

The important design point is that the **approval primitive lives in the MCP server**, while recipes and prompts can sit on top of it.

## Desktop and CLI behavior

- In **goose Desktop**, the user will see an inline form.
- In **goose CLI**, the user will be prompted in the terminal for each field.

That means the same MCP server can support both interfaces without custom UI logic.

## Example implementation

There is a working Rust example in this repository:

`crates/goose-mcp/examples/approval_workflow.rs`

It exposes a `request_approval` tool that:
1. accepts an action summary
2. asks goose for structured user input using elicitation
3. returns an approved/rejected result with an optional reason

## What to look at in the example

1. The request schema:
   - `ApprovalRequest` describes the incoming tool arguments.
   - `ApprovalDecision` describes the data goose should collect from the user.

2. The elicitation call:

```rust
context
    .peer
    .elicit_with_timeout::<ApprovalDecision>(prompt, None)
    .await
```

3. The result handling:
   - accepted responses continue with structured data
   - declined/cancelled responses are handled explicitly
   - the tool returns structured output so recipes or agents can branch on approval state without scraping text

## How to run it

From the repository root:

```bash
source bin/activate-hermit
cargo run -p goose-mcp --example approval_workflow
```

Then configure it as a stdio MCP server in goose and ask goose to use the `request_approval` tool.

## How this fits with recipes

Recipes are still useful on top of this primitive. A recipe can:
- prepare the action summary
- call the MCP tool that requests approval
- continue only after the user responds

If you need headless or automated flows, pair this pattern with recipe success criteria / retry checks rather than baking all workflow control into the prompt itself.
