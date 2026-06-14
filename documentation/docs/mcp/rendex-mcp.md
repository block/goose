---
title: Rendex Extension
description: Add Rendex MCP Server as a goose Extension for Screenshots, PDFs, and HTML-to-Image Rendering
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Rendex MCP Server](https://github.com/copperline-labs/rendex-mcp) as a goose extension to capture screenshots, generate PDFs, and render HTML to images from any webpage or raw HTML.

## Configuration

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem default label="goose Desktop" value="ui">
  [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.rendex.dev%2Fmcp&id=rendex&name=Rendex&description=Capture%20screenshots%2C%20generate%20PDFs%2C%20and%20render%20HTML%20to%20images%20via%20AI%20agents&header=Authorization%3DBearer%20YOUR_RENDEX_API_KEY)
  </TabItem>
  <TabItem label="goose CLI" value="cli">
  Add a `Remote Extension (Streaming HTTP)` extension type.
  
  **Endpoint URL:** `https://mcp.rendex.dev/mcp`
  </TabItem>
</Tabs>

**Configuration Requirement**
Please use the `envVars` property for your API key:
```json
{
  "envVars": {
    "AUTHORIZATION": "Bearer YOUR_RENDEX_API_KEY"
  }
}
```
:::