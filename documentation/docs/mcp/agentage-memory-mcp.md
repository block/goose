---
title: Agentage Memory Extension
description: Add Agentage Memory MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add [Agentage Memory](https://agentage.io) as a goose extension - a remote MCP server that gives every AI tool one shared markdown memory you own.

Agentage Memory is a **remote MCP server** (Streamable HTTP) hosted at `https://memory.agentage.io/mcp`. It authenticates with OAuth 2.1 + PKCE and Dynamic Client Registration, so there is no API key to manage - you sign in to your agentage account in the browser.

:::info Documentation
See the [Agentage Memory documentation](https://agentage.io/blog/mcp-endpoint-is-live) for details.
:::

## Configuration

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmemory.agentage.io%2Fmcp&id=agentage-memory&name=Agentage%20Memory&description=One%20shared%20markdown%20memory%20for%20every%20AI)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Use `goose configure` to add a `Remote Extension (Streaming HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://memory.agentage.io/mcp
  ```
  </TabItem>
</Tabs>
:::

:::info OAUTH FLOW
An OAuth window will open in your browser. Follow the prompts to authorize access to your agentage account.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="agentage-memory"
      extensionName="Agentage Memory"
      description="One shared markdown memory for every AI"
      type="http"
      url="https://memory.agentage.io/mcp"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="agentage-memory"
      description="One shared markdown memory for every AI"
      type="http"
      url="https://memory.agentage.io/mcp"
      timeout={300}
    />
  </TabItem>
</Tabs>

For all setup and configuration options, see the [Agentage Memory documentation](https://agentage.io/blog/mcp-endpoint-is-live).

## Example Usage

Agentage Memory exposes six tools - `memory__search`, `memory__read`, `memory__write`, `memory__edit`, `memory__list`, and `memory__delete` - so any goose session reads and writes the same markdown memory you use from Claude, Cursor, and ChatGPT.

### goose Prompt

```
Remember that I prefer TypeScript with strict mode for all new projects.
```

### goose Output

```
I'll save that to your memory using the memory__write tool.

Saved to preferences/coding-style.md:
- Language: TypeScript (strict mode) for all new projects

This is now available to every AI tool connected to your Agentage Memory.
```