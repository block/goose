---
title: Build Remote Agent Extension
description: Add gbr-mcp so a phone running Build Remote Agent can spectate a goose session (gbr/1)
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Build Remote Agent](https://grokbuildremote.com/) MCP server (`gbr-mcp`) as a goose extension so a phone can **spectate** (and optionally inject into) this desktop goose session.

Protocol `gbr/1`. Phone is spectator + veto, not orchestrator. Independent product by Linespotting AB. Not affiliated with xAI or SpaceX.

This does **not** replace [Telegram Gateway](/docs/experimental/remote-access/telegram-gateway) or [remote goose serve](/docs/guides/remote-goose-server). Attach only loopback Bot API `http://127.0.0.1:8788` or stdio `gbr-mcp`. Never put mailbox keys in goose config.

## Prerequisites

1. Install and pair `gbr-agent` **v0.6.0+** on this machine (keep `gbr-agent run` going):

```bash
curl -fsSL https://grokbuildremote.com/install.sh | bash
gbr-agent version
gbr-agent pair && gbr-agent run
```

2. Clone the MCP server and install Node deps:

```bash
git clone https://github.com/LinespottingOrg/GrokBuildRemote-Agents.git
cd GrokBuildRemote-Agents/mcp/gbr-mcp && npm install
```

You need [Node.js](https://nodejs.org/) for `node` / `npm`.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  Add a **Command-line Extension** named `gbr` with command:

  ```sh
  node /ABS/PATH/GrokBuildRemote-Agents/mcp/gbr-mcp/bin/gbr-mcp.js
  ```
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  node /ABS/PATH/GrokBuildRemote-Agents/mcp/gbr-mcp/bin/gbr-mcp.js
  ```
  </TabItem>
</Tabs>
:::

## Configuration

Replace `/ABS/PATH` with the clone location on your machine. No API keys. `gbr-agent run` must already be listening on `127.0.0.1:8788`.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="gbr-mcp"
      extensionName="Build Remote Agent"
      description="Phone spectator via gbr-agent (loopback Bot API / gbr-mcp)"
      type="stdio"
      command="node"
      args={["/ABS/PATH/GrokBuildRemote-Agents/mcp/gbr-mcp/bin/gbr-mcp.js"]}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="gbr"
      description="Phone spectator via gbr-agent (loopback Bot API / gbr-mcp)"
      type="stdio"
      command="node /ABS/PATH/GrokBuildRemote-Agents/mcp/gbr-mcp/bin/gbr-mcp.js"
      timeout={300}
    />
  </TabItem>
</Tabs>

## Example Usage

Pair the phone first (`gbr-agent pair`), leave `gbr-agent run` up, then ask goose to check the spectator API.

### goose Prompt

```
Call the gbr-mcp tools (or curl http://127.0.0.1:8788/health) and tell me if Build Remote Agent is running. Do not print any mailbox keys.
```

### goose Output

:::note Desktop

Health on `http://127.0.0.1:8788/health` returns ok. Sessions list is available at `/v1/sessions`. Phone is spectator; I will not treat it as the orchestrator.

:::

## Verify without MCP

You can pair the phone even if you skip this extension:

```bash
curl -sS http://127.0.0.1:8788/health
curl -sS http://127.0.0.1:8788/v1/sessions
```

Agent source: https://github.com/LinespottingOrg/GrokBuildRemote-Agents
