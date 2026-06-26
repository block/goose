---
title: sem Extension
description: Add sem MCP Server as a goose Extension for entity-level code intelligence
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add [sem](https://github.com/Ataraxy-Labs/sem) as a goose extension. sem indexes your codebase at the entity level (functions, classes, methods) and builds a real cross-file call and import graph, so goose can pull precise context and check the blast radius of a change instead of inferring structure from grep.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40ataraxy-labs%2Fsem&arg=mcp&id=sem&name=sem&description=Entity-level%20code%20intelligence%20with%20a%20cross-file%20call%20and%20import%20graph&timeout=300)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @ataraxy-labs/sem mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
You'll need [Node.js](https://nodejs.org/) installed (the command uses `npx`). No API key is required. sem runs locally against the repository you launch goose in.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="sem"
      extensionName="sem"
      description="Entity-level code intelligence with a cross-file call and import graph."
      type="stdio"
      command="npx"
      args={["-y", "@ataraxy-labs/sem", "mcp"]}
      timeout={300}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="sem"
      description="Entity-level code intelligence with a cross-file call and import graph."
      type="stdio"
      command="npx -y @ataraxy-labs/sem mcp"
      timeout={300}
    />
  </TabItem>
</Tabs>

## What You Can Do

sem exposes a small set of tools that give goose a structural view of the codebase rather than a text view.

- **sem_context** packs a target entity plus its real callers and callees into a token budget. Use it instead of opening whole files to understand a function or class, the related code arrives with it.
- **sem_impact** answers "what breaks if I change this" deterministically from the graph, so goose can check the blast radius of an edit before making it.
- **sem_entities** finds code by intent (a ranked structural search) when you do not know the name, or lists the entities in a file or directory.
- **sem_diff**, **sem_blame**, and **sem_log** give entity-level change review, authorship, and history.

### Understand a function with its dependencies

> Use sem_context to show me the `parse_config` function along with everything it calls and everything that calls it.

### Check the blast radius before a change

> I want to change the signature of `resolve_path`. Use sem_impact to list what would break.

### Find the right code by description

> Use sem_entities to find where retry logic is implemented in this repo.

Because sem is deterministic and cross-file, it will not hallucinate call edges or miss callers the way a text search can, which makes it a good complement to goose's built-in developer tools.
