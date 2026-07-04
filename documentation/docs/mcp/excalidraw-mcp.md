---
title: Excalidraw Extension
description: Add Excalidraw MCP App as a gosling Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GoslingDesktopInstaller from '@site/src/components/GoslingDesktopInstaller';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/REc5IvWvI5s" />

This tutorial covers how to add the [Excalidraw MCP App](https://github.com/excalidraw/excalidraw-mcp) as a gosling extension that enables gosling to draw hand sketched Excalidraw diagrams in real time.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="gosling Desktop" default>
   [Launch the installer](gosling://extension?cmd=http&id=excalidraw&name=Excalidraw&url=https%3A%2F%2Fexcalidraw-mcp-app.vercel.app%2Fmcp&description=Excalidraw%20MCP%20App%20for%20AI-powered%20diagramming)
  </TabItem>
  <TabItem value="cli" label="gosling CLI">
  Add a `Remote Extension (Streaming HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://excalidraw-mcp-app.vercel.app/mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration
These steps configure the Remote MCP Server. For other deployment options, see the [Excalidraw MCP App documentation](https://github.com/excalidraw/excalidraw-mcp).

<Tabs groupId="interface">
  <TabItem value="ui" label="gosling Desktop" default>
    <GoslingDesktopInstaller
      extensionId="Excalidraw"
      extensionName="Excalidraw MCP App"
      description="Excalidraw MCP App for diagramming"
      type="http"
      url="https://excalidraw-mcp-app.vercel.app/mcp"
      envVars={[]}
    />

  </TabItem>
  <TabItem value="cli" label="gosling CLI">
    <CLIExtensionInstructions
      name="Excalidraw"
      description="Excalidraw MCP App for diagramming"
      type="http"
      url="https://excalidraw-mcp-app.vercel.app/mcp"
      timeout={300}
    />

  </TabItem>
</Tabs>

## Example Usage

In this example, we use the Excalidraw MCP App to have Gosling visualize its own automation pipeline in real time. This demonstrates how Gosling can connect to live tools, reason about workflows, and generate structured diagrams element by element.

### gosling Prompt
```
Hey Gosling review my video automation workflow and create a visual of the automation pipeline for me. I want clean lines and clear labels.
```

### gosling Output

```

─── reading /Users/ebonyl/Desktop/plug-and-play-video.yaml ──────────────────────────

─── Create View elements: [ {"type":"cameraUpdate","width":600}, ...] ──────────────────────────

```
![excalidraw image](/img/excalidrawImage.png)

