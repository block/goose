---
sidebar_position: 55
title: MCP Elicitation
sidebar_label: MCP Elicitation
description: How extensions can request structured information from you during a task
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

MCP Elicitation allows goose to pause and ask you for specific information when an extension needs it. Instead of guessing or making assumptions, goose presents a form requesting exactly what's needed to continue.

This feature is automatically enabled in goose. When an extension that supports elicitation needs information from you, a form will appear in your session.

:::info
[MCP Elicitation](https://modelcontextprotocol.io/specification/draft/client/elicitation) is a feature in the Model Context Protocol. goose supports form mode requests.
:::

## How MCP Elicitation Works

When an extension needs information, goose pauses and presents a form for you to fill out. You can submit your response or cancel the request.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

    A form appears inline in the chat with:
    - Fields for the requested data
    - Required fields marked with an asterisk (*)
    - Default values you can accept or change
    - A **Submit** button to send your response

    After submitting, you'll see a confirmation message.

  </TabItem>
  <TabItem value="cli" label="goose CLI">

    A prompt appears in your terminal with:
    - A message explaining what information is needed (in cyan)
    - Field names (in yellow) with descriptions
    - Required fields marked with a red asterisk (*)
    - Default values shown in brackets, e.g., `[default]`

    Type your response for each field and press Enter. For yes/no questions, you'll see an interactive toggle.

    To cancel the request, press `Ctrl+C`.

  </TabItem>
</Tabs>

:::info Timeout
Elicitation requests timeout after 5 minutes. If you don't respond in time, the request is cancelled and goose will continue without the information.
:::

## For Extension Developers

Want to add elicitation to your own extensions? See the [MCP Elicitation specification](https://modelcontextprotocol.io/specification/draft/client/elicitation) to learn how MCP servers can request structured input from users.

## Approval Workflow Example

One useful pattern for elicitation is **approval-gated workflows**. Instead of hard-coding approval logic into a recipe, an MCP server can ask goose to collect a structured decision from the user and then continue based on that response.

This works in both interfaces:

- **goose Desktop** renders the approval form inline
- **goose CLI** prompts for the same fields in the terminal

A minimal Rust example now lives in this repository:

`crates/goose-mcp/examples/approval_workflow.rs`

The example exposes a `request_approval` tool and uses elicitation to collect:

- `approved`: boolean
- `reason`: optional string

The core server-side call looks like this:

```rust
context
    .peer
    .elicit_with_timeout::<ApprovalDecision>(prompt, None)
    .await
```

This is a good fit when your workflow should:

1. inspect or summarize an action
2. ask the user to explicitly approve or reject it
3. continue with structured input rather than guessing

If you want to build a complete flow on top of this, see the tutorial extension's `approval-workflows` tutorial and pair elicitation with recipe `retry.checks` when you need checkpointed or headless workflows.
