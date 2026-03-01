# Using MCP Apps and MCP-UI

Extensions built with MCP Apps or MCP-UI allow goose Desktop to provide interactive and engaging user experiences. Instead of reading text responses and typing prompts, you can interact with a graphical and clickable UI.

:::info MCP Apps is the official specification
[MCP Apps](/docs/tutorials/building-mcp-apps) is now the official MCP specification for interactive UIs. MCP-UI extensions still work in goose, but MCP Apps is the recommended path for new extensions.
:::

:::warning Experimental Features
The features described in this topic are experimental and in active development. Behavior and support may change in future releases.
:::

## MCP Apps

MCP Apps bring interactive interfaces to goose through the official [MCP Apps specification](https://github.com/modelcontextprotocol/ext-apps). Depending on the extension, apps can be launched in standalone, sandboxed windows or embedded in your chat window.

### Launching Apps in Standalone Windows

Some MCP Apps can be launched in their own windows, allowing you to jump straight to the interface without sending messages to goose.

1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
2. Click `Apps` in the sidebar
3. Browse your available MCP Apps
4. Click `Launch` to launch an app in a new window

:::info Apps Extension
To see the `Apps` page in the sidebar, the [Apps extension](/docs/mcp/apps-mcp) must be enabled from the `Extensions` page. You can also use it to create custom standalone apps.
:::

The `Apps` page displays custom HTML apps you created using the Apps extension, imported HTML apps, and apps from your enabled MCP Apps extensions. The app interface lets you click buttons, fill forms, or use other controls. Apps can call tools and read resources through MCP (if enabled through CORS), but cannot communicate with goose (e.g. via chat).

#### Import an HTML App

Import apps that were created with the Apps extension and shared with you.

1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
2. Click `Apps` in the sidebar
3. Click `Import App`, browse to the app's `.html` file on your file system, and click `Open`

### Using Apps in Chat Windows

Some MCP Apps render directly in your conversation when goose calls a tool that returns UI. The interactive interface appears inline with the chat, letting you make selections, fill forms, or trigger actions without leaving the conversation flow.

If needed, you can just ask goose whether the UI can be loaded in the chat window.

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