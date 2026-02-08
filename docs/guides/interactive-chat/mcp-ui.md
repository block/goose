# Using MCP Apps and MCP-UI

Extensions built with MCP Apps or MCP-UI allow goose Desktop to provide interactive and engaging user experiences. Imagine using a graphical, clickable UI instead of reading text responses and typing all your prompts. These extensions return content that goose can render as embedded UI elements for rich, dynamic, and streamlined interactions.

:::info MCP Apps is the official specification
[MCP Apps](/docs/tutorials/building-mcp-apps) is now the official MCP specification for interactive UIs. MCP-UI extensions still work in goose, but MCP Apps is the recommended path for new extensions.
:::

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

## Try It Out

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
- [MCP-UI Documentation](https://mcpui.dev/guide/introduction) - MCP-UI specification