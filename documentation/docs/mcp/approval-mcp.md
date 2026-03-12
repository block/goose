---
title: Approval Extension
description: Use Goose's built-in Approval extension for explicit go/no-go checkpoints
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

The Approval extension provides a reusable human-in-the-loop primitive for Goose. Instead of encoding approval logic into every recipe or prompt, Goose can call a built-in MCP tool that asks the user for a structured decision and optional reason.

This works in both interfaces:

- **goose Desktop** shows an inline approval form
- **goose CLI** prompts for the same fields in the terminal

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Approval"
    description="Ask for structured approval before continuing a workflow"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to `Add Extension`, then `Built-In Extension`, then select `Approval`.
  </TabItem>
</Tabs>

## What it provides

The extension exposes a single tool:

- `request_approval`: collects
  - `approved`: boolean
  - `reason`: optional string

The tool returns both human-readable text and structured content with:

- `action_summary`
- `status` (`approved`, `rejected`, `declined`, `cancelled`, or `no_response`)
- `approved`
- `reason`

Use it when Goose should inspect or summarize an action first, then wait for an explicit user decision before proceeding.

## Example usage

Ask Goose to use the Approval extension before a risky step:

```text
Review the migration plan, summarize the risks, and use the approval tool before making any changes.
```

The approval prompt can be used for:

- risky shell commands
- release signoff checkpoints
- destructive file operations
- gated multi-step workflows

## Relationship to recipes

Recipes are still useful on top of this extension. A recipe can prepare the action summary, call `request_approval`, and only continue after the user responds.

If you need automated or headless workflows, pair this with recipe `retry.checks` instead of trying to reimplement approval state inside prompt text alone.
