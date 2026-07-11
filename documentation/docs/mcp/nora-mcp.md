---
title: Nora Extension
description: Add the Nora MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Nora MCP Server](https://github.com/solomon2773/nora/tree/master/mcp-server) as a goose extension to inspect and operate a self-hosted Nora agent fleet.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40noraai%2Fmcp-server&id=nora&name=Nora&description=Operate%20a%20self-hosted%20Nora%20agent%20fleet%20from%20goose.&env=NORA_API_URL%3Dhttps%3A%2F%2Fnora.example.com&env=NORA_API_KEY%3Dnora_xxxxxxxx&env=NORA_MCP_ALLOW_DESTRUCTIVE%3Dfalse)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    **Command**
    ```sh
    npx -y @noraai/mcp-server
    ```
  </TabItem>
</Tabs>

**Environment variables**

```
NORA_API_URL=https://nora.example.com
NORA_API_KEY=nora_xxxxxxxx
NORA_MCP_ALLOW_DESTRUCTIVE=false
```

:::

## Configuration

:::info
You'll need [Node.js 20 or later](https://nodejs.org/) to run the server with `npx`. Create a workspace API key in Nora under **Workspace → API Keys**. See the [Nora MCP guide](https://noradocs.solomontsao.com/guides/mcp-server) for required scopes and configuration details.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="nora"
      extensionName="Nora"
      description="Operate a self-hosted Nora agent fleet from goose."
      type="stdio"
      command="npx"
      args={["-y", "@noraai/mcp-server"]}
      envVars={[
        { name: "NORA_API_URL", label: "https://nora.example.com" },
        { name: "NORA_API_KEY", label: "nora_xxxxxxxx" },
        { name: "NORA_MCP_ALLOW_DESTRUCTIVE", label: "false" }
      ]}
      apiKeyLink="https://noradocs.solomontsao.com/guides/mcp-server"
      apiKeyLinkText="Nora API credentials"
    />
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Nora"
      description="Operate a self-hosted Nora agent fleet from goose."
      type="stdio"
      command="npx -y @noraai/mcp-server"
      timeout={300}
      envVars={[
        { key: "NORA_API_URL", value: "https://nora.example.com" },
        { key: "NORA_API_KEY", value: "nora_xxxxxxxx" },
        { key: "NORA_MCP_ALLOW_DESTRUCTIVE", value: "false" }
      ]}
      infoNote={
        <>
          Enter the URL of your Nora deployment and a workspace API key. Keep <code>NORA_MCP_ALLOW_DESTRUCTIVE</code> set to <code>false</code> unless you explicitly want to register tools such as <code>delete_agent</code>. See the <a href="https://noradocs.solomontsao.com/guides/mcp-server" target="_blank" rel="noopener noreferrer">Nora MCP guide</a> for details.
        </>
      }
    />
  </TabItem>
</Tabs>

## Example Usage

After configuration, ask goose for a read-only fleet summary:

### goose Prompt

```
Show me the current Nora fleet status and list any agents that are not running.
```

### goose Output

:::note Desktop

goose calls Nora's `get_fleet_status` and `list_agents` tools, then summarizes the live response from your control plane. Agent names, states, and counts depend on your Nora deployment.

:::

## Safety

The server exposes read tools for fleet status, metrics, monitoring events, and cost data. Lifecycle tools require an API key with `agents:write`. The `delete_agent` tool is registered only when `NORA_MCP_ALLOW_DESTRUCTIVE=true`.

## Resources

- [Nora repository](https://github.com/solomon2773/nora)
- [Nora MCP guide](https://noradocs.solomontsao.com/guides/mcp-server)
- [`@noraai/mcp-server` on npm](https://www.npmjs.com/package/@noraai/mcp-server)
