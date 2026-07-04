---
sidebar_position: 1
title: Using MCP Apps
sidebar_label: Using MCP Apps
description: Learn how gosling renders interactive UI components from MCP Apps extensions
---

# Using MCP Apps

Extensions built with MCP Apps allow gosling Desktop to provide interactive and engaging user experiences. Instead of reading text responses and typing prompts, you can interact with a graphical and clickable UI.

:::info MCP Apps is the official specification
[MCP Apps](/docs/tutorials/building-mcp-apps) is the official MCP specification for interactive UIs. Use MCP Apps for new interactive extensions.
:::

:::warning Experimental Features
The features described in this topic are experimental and in active development. Behavior and support may change in future releases.
:::

MCP Apps bring interactive interfaces to gosling through the official [MCP Apps specification](https://github.com/modelcontextprotocol/ext-apps). Apps render directly inside your chat window.

### Using Apps in Chat Windows

Some MCP Apps render directly in your conversation when gosling calls a tool that returns UI. The interactive interface appears inline with the chat, letting you make selections, fill forms, or trigger actions without leaving the conversation flow.

If needed, you can just ask gosling whether the UI can be loaded in the chat window.

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

## For Extension Developers

Add interactivity to your own extensions:

- [Building MCP Apps](/docs/tutorials/building-mcp-apps) - Step-by-step tutorial (recommended)
- [MCP Apps SDK and Specification](https://modelcontextprotocol.github.io/ext-apps/api/)
- [MCP Apps SDK Guide](https://mcpui.dev/guide/introduction)
