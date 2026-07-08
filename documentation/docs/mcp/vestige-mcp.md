---
title: Vestige Extension
description: Add Vestige MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Vestige MCP Server](https://github.com/samvallad33/vestige) as a goose extension to give goose long-term memory that persists across sessions. Vestige runs locally and stores decisions, preferences, and past fixes on your own machine, so goose can recall project context instead of starting from scratch each time.

:::tip Quick Install
**Command**
```sh
npx -y -p vestige-mcp-server@latest vestige-mcp
```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="goose CLI" default>

**Configure the extension:**

<CLIExtensionInstructions
  name="Vestige"
  description="Local-first long-term memory that persists across sessions"
  command="npx -y -p vestige-mcp-server@latest vestige-mcp"
/>
  </TabItem>
</Tabs>

:::info
Vestige stores its data locally and needs no API key or cloud account. See the [Vestige README](https://github.com/samvallad33/vestige) for configuration options such as the data directory and dashboard.
:::

## Example Usage

Vestige lets goose save what it learns during a session and recall it later, so context carries over between conversations instead of being lost when the session ends.

### goose Prompt

> _goose, remember that this project uses pnpm workspaces and the API server runs on port 3927. Next session, recall what you know about this project before you start._

### goose Output

:::note CLI

<details>
    <summary>Tool Calls</summary>

    ─── smart_ingest | vestige ──────────────────────────

    content: This project uses pnpm workspaces. The API server runs on port 3927.


    ─── search | vestige ──────────────────────────

    query: project setup, build tooling, ports

</details>

Saved. I've stored two facts about this project in Vestige:

- **Package manager**: pnpm workspaces
- **API server port**: 3927

Next session, when you ask me to recall the project, I'll search Vestige first and load these back before I start, so I won't need you to re-explain the setup.

:::
