---
title: SuperBased Observer Extension
description: Add SuperBased Observer MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [SuperBased Observer MCP Server](https://github.com/superbasedapp/observer) as a goose extension. Observer is a local-first observability tool for AI coding agents: it records what your tool calls actually cost — tokens in and out, prompt-cache reads and writes, and outcomes — into a local SQLite database, then exposes that history back to goose over MCP so you can ask about your own past sessions instead of guessing.

Everything runs on your machine. There is no account, no hosted service, and no network egress.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=observer&arg=serve&id=observer&name=SuperBased%20Observer&description=Local-first%20token%2C%20cost%20and%20cache%20observability%20for%20AI%20coding%20agents)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  observer serve
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
Unlike most extensions in this directory, this one does **not** run via `npx` or `uvx` — it needs the `observer` binary on your PATH first. It is a single static binary with no runtime dependencies:

```sh
npm i -g @superbased/observer
# or
pipx install superbased-observer
```
:::

:::note Run the daemon for full data
The MCP server reads a local database that the Observer daemon populates. Start it once with:

```sh
observer start
```

Without the daemon the MCP tools still answer, but only from whatever history has already been captured.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="observer"
      extensionName="SuperBased Observer"
      description="Local-first token, cost and cache observability for AI coding agents"
      type="stdio"
      command="observer"
      args={["serve"]}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="observer"
      description="Local-first token, cost and cache observability for AI coding agents"
      type="stdio"
      command="observer serve"
      timeout={300}
    />
  </TabItem>
</Tabs>

## Example Usage

Observer is most useful once you have a few sessions of history. Because goose extensions *are* MCP servers, and Observer records activity from every AI coding tool you use — not just goose — the history you query here spans your whole toolchain.

No API keys or environment variables are required.

### goose Prompt

> _What did my last goose session cost, and which tool calls were the most expensive?_

### goose Output

:::note CLI

<details>
  <summary>Tool Calls</summary>

  ─── get_session_summary | observer ──────────────────────────

  ─── get_cost_summary | observer ─────────────────────────────

</details>

_Illustrative output — your own figures will differ._

Your last goose session ran 41 tool calls over 18 minutes at an estimated **$0.42**.

The cost was dominated by three calls:

1. **`developer__shell`** (a repo-wide grep) — 84K input tokens. The output was large enough to push the next two turns' context up as well.
2. **`developer__text_editor`** on a 2,100-line file — 31K tokens.
3. **`computercontroller__web_search`** — 12K tokens.

Prompt caching absorbed 63% of your input tokens, so the uncached equivalent would have been roughly **$1.10**.

The single cheapest change: that repo-wide grep was re-run three times with the same arguments. Narrowing it, or reusing the first result, would have saved about $0.09 of the $0.42.

:::

## Notes

- All data stays in a local SQLite database (`~/.observer/observer.db` by default).
- Observer supports 29 AI coding tools, so sessions from goose sit alongside the rest of your toolchain in one normalized schema.
- Dollar figures are estimated list-price totals, not invoiced amounts.
