---
title: Bernstein Extension
description: Add Bernstein MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Bernstein MCP Server](https://github.com/sipyourdrink-ltd/bernstein) as a goose extension. Bernstein is a deterministic orchestrator for CLI coding agents. It runs work in parallel inside isolated git worktrees and records every run in a replay journal, so goose can dispatch a batch of tasks and then inspect exactly what each worker did.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=--from&arg=bernstein&arg=bernstein&arg=mcp&id=bernstein&name=Bernstein&description=Deterministic%20orchestration%20for%20CLI%20coding%20agents%20with%20replay%20journal%20and%20verifiable%20audit%20log)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  uvx --from bernstein bernstein mcp
  ```
  </TabItem>
</Tabs>
:::

## What is Bernstein?

Bernstein orchestrates CLI coding agents. It ships adapters for Claude Code, Codex, Gemini CLI, Aider, Cursor and 40+ other agent CLIs, and schedules their work from a deterministic Python scheduler with no model in the coordination loop.

Each task runs in its own git worktree, so parallel workers do not share a mutable checkout. Every run is written to a replay journal with lineage records for the artifacts it produced. An HMAC-chained audit log is available as an opt-in, and `bernstein audit verify` checks it offline without contacting a service.

The MCP server advertises 13 tools at its default tier:

| Tool | Purpose |
|------|---------|
| `bernstein_health` | Liveness check for the MCP server |
| `bernstein_run` | Start an orchestration run from a goal |
| `bernstein_status` | Summary of task counts |
| `bernstein_tasks` | List tasks |
| `bernstein_task_handle` | Return a verifiable handle for a run |
| `bernstein_cost` | Total spend and per-role breakdown |
| `bernstein_claim` | Claim the next eligible task |
| `bernstein_update` | Post an incremental progress update |
| `bernstein_post_artifact` | Attach a journal-anchored artifact |
| `bernstein_stop` | Request a graceful shutdown |
| `bernstein_approve` | Approve a pending or blocked task |
| `bernstein_create_subtask` | Create a subtask under a parent |
| `load_skill` | Load a skill pack body on demand |

Starting the server with `--mcp-tier all` adds five more tools: `bernstein_context`, `bernstein_scenarios`, `bernstein_scenario`, `bernstein_scenario_status`, and `verify_chain`. Starting it with `--mcp-tier core` narrows the set to the five read and dispatch tools, which keeps the context budget small.

The server also exposes a `bernstein://capability` resource, lineage resources, and three prompts: `orchestrate_goal`, `triage_failed_tasks`, and `cost_recap`.

No API key or environment variable is required by the server itself. The agent CLIs that Bernstein drives are configured separately, with their own credentials.

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`. Bernstein requires Python 3.12 or newer.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="bernstein"
      extensionName="Bernstein"
      description="Deterministic orchestration for CLI coding agents with replay journal and verifiable audit log"
      type="stdio"
      command="uvx"
      args={["--from", "bernstein", "bernstein", "mcp"]}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="bernstein"
      description="Deterministic orchestration for CLI coding agents with replay journal and verifiable audit log"
      type="stdio"
      command="uvx --from bernstein bernstein mcp"
      timeout={300}
    />
  </TabItem>
</Tabs>

## Example Usage

Bernstein is useful when a piece of work is large enough to split across several agent runs and you want to be able to reconstruct afterwards what each run did.

### Dispatch a run

#### goose Prompt

```
Use Bernstein to add retry with exponential backoff to the HTTP client
in this repo, and cover it with tests. Assign it to the backend role.
```

goose calls `bernstein_run` with the goal, and Bernstein returns the created task id, title, and status. The work then proceeds in its own git worktree.

### Track progress

#### goose Prompt

```
What is the status of my Bernstein tasks right now?
```

goose calls `bernstein_status` for the task counts, and `bernstein_tasks` when you ask for the individual tasks rather than the summary.

### Review spend

#### goose Prompt

```
How much have the Bernstein runs cost so far, broken down by role?
```

goose calls `bernstein_cost`, which returns the total in USD along with the per-role breakdown.

### Unblock a task

#### goose Prompt

```
Task 14 is blocked waiting on review. Approve it and create a follow-up
subtask for the documentation update.
```

goose calls `bernstein_approve` for the blocked task, then `bernstein_create_subtask` to link the follow-up work to it.
