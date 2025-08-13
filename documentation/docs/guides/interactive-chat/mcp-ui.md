---
sidebar_position: 1
title: MCP-UI Extensions
sidebar_label: MCP-UI Extensions
description: Learn how Goose can render graphical and interactive UI components from MCP-UI-enabled extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import { PanelLeft } from 'lucide-react';

Extensions built on MCP-UI allow Goose Desktop to provide interactive and engaging user experiences. Imagine using a graphical, clickable UI instead of reading text responses and typing all your prompts:

<div style={{ width: '100%', maxWidth: '800px', margin: '0 auto' }}>
  <video 
    controls 
    playsInline
    style={{ 
      width: '100%', 
      aspectRatio: '2876/2160',
      borderRadius: '8px'
    }}
  >
    <source src={require('@site/static/videos/plan-trip-demo.mp4').default} type="video/mp4" />
    Your browser does not support the video tag.
  </video>
</div>

<br/>
MCP-UI-enabled extensions return content that Goose can render as embedded UI elements for rich, dynamic, and streamlined interactions.

## Try It Out

See how interactive responses work in Goose. 

### Add Enabled Extension

For this exercise, we'll add an MCP-UI-enabled extension that connects to [MCP-UI Demos](https://mcp-aharvard.netlify.app/) provided by Andrew Harvard.

  <Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
      1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
      2. Click `Extensions`
      3. Click `Add custom extension`
      4. On the `Add custom extension` modal, enter the following:
         - **Extension Name**: Rich Demo
         - **Type**: Streamable HTTP
         - **Endpoint**: `https://mcp-aharvard.netlify.app/mcp`
      5. Click `Add Extension` to save the extension
      6. Navigate to the chat
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        <CLIExtensionInstructions
          name="rich_demo"
          type="http"
          url="https://mcp-aharvard.netlify.app/mcp"
          timeout={300}
        />
    </TabItem>
  </Tabs>

### Interact in Chat

In Goose Desktop, ask:

- `Help me select seats for my flight`

Instead of just text, you'll see an interactive response with:
- A visual seat map with available and occupied seats
- Real-time, clickable selection capabilities
- A booking confirmation with flight details

Ask questions to try out other demos:

- `Plan my next trip based on my mood`
- `What's the weather in Philadelphia?`

Stay tuned as more extensions build with MCP-UI!

## For Extension Developers

Want to add interactivity to your own extensions? MCP-UI extends the Model Context Protocol to allow MCP servers to return content that agents can render as UI components instead of text-only responses. Learn more:
- [MCP-UI: Bringing the Browser into the Agent](/blog/2025/08/11/mcp-ui-post-browser-world)
- [MCP-UI Documentation](https://mcpui.dev/guide/introduction)
