---
title: VEED Extension
description: Add VEED AI Video Generator as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [VEED AI Video Generator](https://veedstudio.github.io/veed-fabric-mcp/) as a goose extension that enables goose to generate AI talking-head videos with custom characters and voices.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   [Launch the installer](goose://extension?cmd=http&id=veed&name=VEED%20AI%20Video%20Generator&url=https%3A%2F%2Fwww.veed.io%2Fapi%2Fv1%2Fmcp&description=Generate%20AI%20talking-head%20videos%20with%20custom%20characters%20and%20voices)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streaming HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://www.veed.io/api/v1/mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration
These steps configure the Remote MCP Server. For more details, see the [VEED Fabric MCP documentation](https://veedstudio.github.io/veed-fabric-mcp/).

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="veed"
      extensionName="VEED AI Video Generator"
      description="Generate AI talking-head videos with custom characters and voices"
      type="http"
      url="https://www.veed.io/api/v1/mcp"
      envVars={[]}
    />

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="VEED AI Video Generator"
      description="Generate AI talking-head videos with custom characters and voices"
      type="http"
      url="https://www.veed.io/api/v1/mcp"
      timeout={300}
    />

  </TabItem>
</Tabs>

## Example Usage

In this example, we use the VEED extension to generate an AI talking-head video.

### goose Prompt
```
Create a short video of a professional presenter explaining the benefits of AI-powered video editing. Use a friendly tone and keep it under 30 seconds.
```
