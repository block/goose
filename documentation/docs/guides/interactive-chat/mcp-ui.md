---
sidebar_position: 1
title: MCP-UI Extensions
sidebar_label: MCP-UI Extensions
description: Learn how goose can render graphical and interactive UI components from MCP-UI-enabled extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import { PanelLeft } from 'lucide-react';

Extensions built on MCP-UI allow goose Desktop to provide interactive and engaging user experiences. Imagine using a graphical, clickable UI instead of reading text responses and typing all your prompts:

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
MCP-UI-enabled extensions return content that goose can render as embedded UI elements for rich, dynamic, and streamlined interactions.

## Try It Out

See how interactive responses work in goose. 

### Add Enabled Extension

For this exercise, we'll add an MCP-UI-enabled extension that connects to [MCP-UI Demos](https://mcp-aharvard.netlify.app/) provided by Andrew Harvard.

  <Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
      <GooseDesktopInstaller
        extensionId="richdemo"
        extensionName="Rich Demo"
        description="Demo MCP-UI-enabled extension"
        type="http"
        url="https://mcp-aharvard.netlify.app/mcp"
      />
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        <CLIExtensionInstructions
          name="rich_demo"
          description="Demo MCP-UI-enabled extension"
          type="http"
          url="https://mcp-aharvard.netlify.app/mcp"
          timeout={300}
        />
    </TabItem>
  </Tabs>

### Interact in Chat

In goose Desktop, ask:

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
