Client applications that support the [Agent Client Protocol (ACP)](https://agentclientprotocol.com/) can connect natively to goose. This integration allows you to seamlessly interact with goose directly from the client.

:::warning Experimental Feature
ACP is an emerging specification that enables clients to communicate with AI agents like goose. This feature has limited adoption and may evolve as the protocol develops.
:::

## How It Works
After you configure goose as an agent in the ACP client, you gain access to goose's core agent functionality, including its extensions and tools. goose also automatically loads any [configured MCP servers](#using-mcp-servers-from-acp-clients) from your ACP client alongside its own extensions, making their tools available without additional configuration.

The client manages the goose lifecycle automatically, including:

- **Initialization**: The client runs the `goose acp` command to initialize the connection
- **Communication**: The client communicates with goose over stdio using JSON-RPC
- **Multiple Sessions**: The client manages multiple concurrent goose conversations simultaneously
- **Session Isolation**: Each session maintains its own isolated state, including conversation history, agent context, and extension configurations, allowing concurrent sessions to run without interference

:::info Session Persistence
ACP sessions are saved to goose's session history where you can access and manage them using goose. Access to session history in ACP clients might vary.
:::

:::tip Reference Implementation
The [goose for VS Code](/docs/experimental/vs-code-extension) extension uses ACP to communicate with goose. See the [vscode-goose](https://github.com/block/vscode-goose) repository for implementation details.
:::

## Setup in ACP Clients
Any editor or IDE that supports ACP can connect to goose as an agent server. Check the [official ACP clients list](https://agentclientprotocol.com/overview/clients) for available clients with links to their documentation.

### Example: Zed Editor Setup

ACP was originally developed by [Zed](https://zed.dev/). Here's how to configure goose in Zed:

#### 1. Prerequisites

Ensure you have both Zed and goose CLI installed:

- **Zed**: Download from [zed.dev](https://zed.dev/)
- **goose CLI**: Follow the [installation guide](/docs/getting-started/installation)

  - ACP support works best with version 1.16.0 or later - check with `goose --version`.

  - Temporarily run `goose acp` to test that ACP support is working:

    ```
    ~ goose acp
    Goose ACP agent started. Listening on stdio...
    ```

    Press `Ctrl+C` to exit the test.

#### 2. Configure goose as a Custom Agent

Add goose to your Zed settings:

1. Open Zed
2. Press `Cmd+Option+,` (macOS) or `Ctrl+Alt+,` (Linux/Windows) to open the settings file
3. Add the following configuration:

```json
{
  "agent_servers": {
    "goose": {
      "command": "goose",
      "args": ["acp"],
      "env": {}
    }
  },
  // more settings
}
```

You should now be able to interact with goose directly in Zed. Your ACP sessions use the same extensions that are enabled in your goose configuration, and your tools (Developer, Computer Controller, etc.) work the same way as in regular goose sessions.

#### 3. Start Using goose in Zed

1. **Open the Agent Panel**: Click the sparkles agent icon in Zed's status bar
2. **Create New Thread**: Click the `+` button to show thread options
3. **Select goose**: Choose `New goose` to start a new conversation with goose
4. **Start Chatting**: Interact with goose directly from the agent panel

#### Advanced Configuration

##### Overriding Provider and Model

By default, goose will use the provider and model defined in your [configuration file](/docs/guides/config-files). You can override this for specific ACP configurations using the `GOOSE_PROVIDER` and `GOOSE_MODEL` environment variables.

The following Zed settings example configures two goose agent instances. This is useful for:
- Comparing model performance on the same task
- Using cost-effective models for simple tasks and powerful models for complex ones

```json
{
  "agent_servers": {
    "goose": {
      "command": "goose",
      "args": ["acp"],
      "env": {}
    },
    "goose (GPT-4o)": {
      "command": "goose",
      "args": ["acp"],
      "env": {
        "GOOSE_PROVIDER": "openai",
        "GOOSE_MODEL": "gpt-4o"
      }
    }
  },
  // more settings
}
```

## Using MCP Servers from ACP Clients

MCP servers configured in the ACP client's `context_servers` are automatically available to goose. This allows you to use those MCP servers when using both native client features and the goose agent integration.

**Example (Zed):**

```json
{
  "context_servers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "/path/to/allowed/dir"
      ]
    }
  },
  "agent_servers": {
    "goose": {
      "command": "goose",
      "args": ["acp"],
      "env": {}
    }
  },
  // more settings
}
```

To find out what tools are available, just ask goose while it's running in the client.

:::info
All MCP servers in `context_servers` are automatically available to goose, provided that they use stdio (command-based) or HTTP transports. goose doesn't support servers that use the deprecated SSE transport.

If a server in `context_servers` has the same name as a goose extension, goose uses its own [configuration](/docs/guides/config-files).
:::
## Additional Resources

<ContentCardCarousel
  items={[
    {
      type: 'video',
      title: 'Intro to Agent Client Protocol (ACP) | Vibe Code with goose',
      description: 'Watch how ACP lets you seamlessly integrate goose into your code editor to streamline fragmented workflows.',
      thumbnailUrl: 'https://img.youtube.com/vi/Hvu5KDTb6JE/maxresdefault.jpg',
      linkUrl: 'https://www.youtube.com/watch?v=Hvu5KDTb6JE',
      date: '2025-10-16',
      duration: '50:23'
    },
   {
      type: 'blog',
      title: 'Intro to Agent Client Protocol (ACP): The Standard for AI Agent-Editor Integration',
      description: 'Learn how to integrate AI agents like goose directly into your code editor via ACP, eliminating window-switching and vendor lock-in.',
      thumbnailUrl: chooseYourIde,
      linkUrl: '/goose/blog/2025/10/24/intro-to-agent-client-protocol-acp',
      date: '2025-10-24',
      duration: '7 min read'
    }
  ]}
/>