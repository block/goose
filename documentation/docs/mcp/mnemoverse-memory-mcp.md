---
title: Mnemoverse Memory Extension
description: Add Mnemoverse Memory MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Mnemoverse Memory MCP Server](https://github.com/mnemoverse/mcp-memory-server) as a goose extension to give goose persistent memory that is shared across your AI tools: write a memory in goose, recall it in Claude Code, Cursor, or any other MCP client, and the other way around.

:::tip Quick Install

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40mnemoverse%2Fmcp-memory-server&id=mnemoverse-memory&name=Mnemoverse%20Memory&description=Persistent%20memory%20shared%20across%20AI%20tools&env=MNEMOVERSE_API_KEY%3DMnemoverse%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @mnemoverse/mcp-memory-server
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  MNEMOVERSE_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`. A free API key is available at [console.mnemoverse.com](https://console.mnemoverse.com), no credit card required.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="mnemoverse-memory"
    extensionName="Mnemoverse Memory"
    description="Persistent memory shared across AI tools"
    command="npx"
    args={["-y", "@mnemoverse/mcp-memory-server"]}
    envVars={[
      { name: "MNEMOVERSE_API_KEY", label: "Mnemoverse API Key" }
    ]}
    apiKeyLink="https://console.mnemoverse.com"
    apiKeyLinkText="MNEMOVERSE_API_KEY"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="mnemoverse"
      description="Persistent memory shared across AI tools"
      command="npx -y @mnemoverse/mcp-memory-server"
      envVars={[
        { key: "MNEMOVERSE_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          Get a free <a href="https://console.mnemoverse.com" target="_blank" rel="noopener noreferrer">Mnemoverse API Key</a> and paste it in. The same key gives every MCP client the same memory.
        </>
      }
    />

  </TabItem>
</Tabs>

## Example Usage

Let's use the Mnemoverse Memory extension to save a project decision in goose and rely on it in a later session.

### goose Prompt

```
Remember that this project uses pnpm, not npm. We decided this after a
lockfile conflict, and it applies to every package in this repo.
```

### goose Output

```
I'll save that decision to long-term memory.

─── memory_write | mnemoverse ──────────────────────────
content: Project uses pnpm, not npm. Decided after a lockfile conflict;
applies to all packages in this repo.

Saved. Future sessions, in goose or in any other tool connected to the
same Mnemoverse account, will recall this before running package commands.
```

In a later session, ask:

```
Before you install anything, check what package manager we use here.
```

```
─── memory_read | mnemoverse ──────────────────────────
query: package manager for this repo

Found it: this project uses pnpm, not npm (decided after a lockfile
conflict). I'll use pnpm for the install.
```

The server exposes tools for writing, recalling, listing recent memories, usage stats, and rating which recalled memories helped, which tunes future recall. Memories carry importance scores and associations that strengthen with use. See the [documentation](https://mnemoverse.com/docs/api/mcp-server) for the full tool list.
