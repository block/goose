---
sidebar_position: 1
title: Using MCP Apps and MCP-UI
sidebar_label: Using MCP Apps and MCP-UI
description: Learn how goose renders interactive UI components from MCP Apps and MCP-UI extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import { PanelLeft } from 'lucide-react';

# Using MCP Apps and MCP-UI

Extensions built with MCP Apps or MCP-UI allow goose Desktop to provide interactive and engaging user experiences. Instead of reading text responses and typing prompts, you can interact with a graphical and clickable UI.

:::info MCP Apps is the official specification
[MCP Apps](/docs/tutorials/building-mcp-apps) is now the official MCP specification for interactive UIs. MCP-UI extensions still work in goose, but MCP Apps is the recommended path for new extensions.
:::

## MCP Apps

MCP Apps bring interactive interfaces to goose through the official [MCP Apps specification](https://github.com/modelcontextprotocol/ext-apps). Apps can be launched as standalone apps or embedded in your chat session.

### Launching Apps Directly

MCP Apps can be launched in their own windows where you can interact with their UI directly. This is useful for interfaces like dashboards and data visualizers, so you can jump straight to the interface without sending messages to goose.

1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
2. Click `Apps` in the sidebar
3. Browse your available apps. The page displays recently cached MCP Apps and MCP Apps from your enabled extensions.
4. Click `Launch` to launch an app in a new window

The MCP App opens with its own interface where you can click buttons, fill forms, or use other controls. Apps can call tools and read resources through MCP, but cannot communicate with goose.

### Using Apps in Chat

MCP Apps can also render directly in your conversation when goose calls a tool that returns UI. The interactive interface appears inline with the chat, letting you make selections, fill forms, or trigger actions without leaving the conversation flow.

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

## MCP-UI

MCP-UI is an earlier specification for interactive UIs that renders content embedded in your chat. While MCP Apps is now the recommended approach, MCP-UI extensions continue to work in goose.

### Try It Out

See how interactive responses work in goose. For this exercise, we'll add an extension that connects to [MCP-UI Demos](https://mcp-aharvard.netlify.app/) provided by Andrew Harvard.

  <Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
      <GooseDesktopInstaller
        extensionId="richdemo"
        extensionName="Rich Demo"
        description="Demo interactive extension"
        type="http"
        url="https://mcp-aharvard.netlify.app/mcp"
      />
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        <CLIExtensionInstructions
          name="rich_demo"
          description="Demo interactive extension"
          type="http"
          url="https://mcp-aharvard.netlify.app/mcp"
          timeout={300}
        />
    </TabItem>
  </Tabs>

In goose Desktop, ask:

- `Help me select seats for my flight`

Instead of just text, you'll see an interactive response with:
- A visual seat map with available and occupied seats
- Real-time, clickable selection capabilities
- A booking confirmation with flight details

Try out other demos:

- `Plan my next trip based on my mood`
- `What's the weather in Philadelphia?`

## For Extension Developers

Add interactivity to your own extensions:

- [Building MCP Apps](/docs/tutorials/building-mcp-apps) - Step-by-step tutorial (recommended)
- [MCP Apps SDK and Specification](https://modelcontextprotocol.github.io/ext-apps/api/)
- [MCP Apps SDK Guide](https://mcpui.dev/guide/introduction)
