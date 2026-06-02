---
title: TypeUI Extension
description: Add TypeUI MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [TypeUI MCP Server](https://www.typeui.sh/docs/guides/goose) as a goose extension to give goose access to curated design systems, UI prompts, and layout variations while building interfaces.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?url=https%3A%2F%2Fmcp.typeui.sh%2Fmcp&type=streamable_http&timeout=300&id=typeui&name=TypeUI&description=Build%20better%20interfaces%20with%20TypeUI%20design%20systems%2C%20UI%20prompts%2C%20and%20layout%20variations.)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Use `goose configure` to add a `Remote Extension (Streamable HTTP)` extension type with:

  **Endpoint URL**
  ```sh
  https://mcp.typeui.sh/mcp
  ```
  </TabItem>
</Tabs>
:::

:::info OAuth Flow
If goose asks you to authorize the connection, sign in with your TypeUI account in the browser window.
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="typeui"
      extensionName="TypeUI"
      description="Build better interfaces with TypeUI design systems, UI prompts, and layout variations"
      type="http"
      url="https://mcp.typeui.sh/mcp"
      timeout={300}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="TypeUI"
      description="Build better interfaces with TypeUI design systems, UI prompts, and layout variations"
      type="http"
      url="https://mcp.typeui.sh/mcp"
      timeout={300}
    />
  </TabItem>
</Tabs>

For more setup details, see the [TypeUI guide for goose](https://www.typeui.sh/docs/guides/goose).

## Example Usage

In this example, goose uses TypeUI to choose a visual direction and retrieve UI prompt context before building a landing page and pricing variations.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   1. Open a new session in goose Desktop
  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. Open a terminal and start a new goose session:

  ```sh
  goose session
  ```

  </TabItem>
</Tabs>

### goose Prompt

```
Use TypeUI to build a landing page with the Bento design skill.
Then give me three pricing section variations so I can compare directions.
```

### goose Output

:::note Desktop

goose will use TypeUI to find the selected design skill, apply its visual guidance, and retrieve relevant landing page and pricing prompt context.

It can then update your project with a styled landing page and generate multiple pricing section directions for you to review before choosing one.

:::
