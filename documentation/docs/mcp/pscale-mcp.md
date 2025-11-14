---
title: PlanetScale Extension
description: Add PScale Extension to goose
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';


This tutorial covers how to add the [PScale MCP Server](https://github.com/planetscale/cli?tab=readme-ov-file#mcp-server-integration) as a goose extension to enable you to inspect your databses and ingest query insights.

## Configuration

:::info
Note that you'll need [PScale](https://github.com/planetscale/cli?tab=readme-ov-file#installation) installed on your system to run this command, as the MCP server is shipped as part of your `pscale` client.
:::


<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="pscale"
    extensionName="PlanetScale"
    description="Invoke the PlanetScale MCP Server to learn more about PlanetScale environments and databases hosted there."
    command="pscale"
    args={["mcp", "server"]}
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="PlanetScale"
      description="Invoke the PlanetScale MCP Server to learn more about PlanetScale environments and databases hosted there."
      command="pscale mcp server"
    />
  </TabItem>
</Tabs>

## Example Usage

### goose Prompt

> _which databases exist in test-staging_

### goose Output

:::note CLI

<details>
    <summary>Tool Calls</summary>
    ─── list_databases | pscale ──────────────────────────
    org: test-staging
</details>


Here are all the databases in the test-staging organization:

1. vhs_glow (MySQL)
2. whale_putunias (MySQL)
3. tangential_rabbits (MySQL)
4. opaque_shadows (MySQL)

All databases in this organization are MySQL databases. You can use any of these database names to explore further details like branches, schemas, or to run queries.
:::