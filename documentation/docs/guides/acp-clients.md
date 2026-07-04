---
sidebar_position: 105
title: Using gosling in ACP Clients
sidebar_label: gosling in ACP Clients
---

Client applications that support the [Agent Client Protocol (ACP)](https://agentclientprotocol.com/) can connect natively to gosling. This integration allows you to seamlessly interact with gosling directly from the client.

:::warning Experimental Feature
ACP is an emerging specification that enables clients to communicate with AI agents like gosling. This feature has limited adoption and may evolve as the protocol develops.
:::

## How It Works
After you configure gosling as an agent in the ACP client, you gain access to gosling's core agent functionality, including its extensions and tools. gosling also automatically loads any [configured MCP servers](#using-mcp-servers-from-acp-clients) from your ACP client alongside its own extensions, making their tools available without additional configuration.

The client manages the gosling lifecycle automatically, including:

- **Initialization**: The client runs the `gosling acp` command to initialize the connection
- **Communication**: The client communicates with gosling over stdio using JSON-RPC
- **Multiple Sessions**: The client manages multiple concurrent conversations, each with isolated state
- **Model and Mode Switching**: The client can switch models and modes mid-session without restarting
- **File Operations**: The client handles file reads and writes, so gosling sees changes not yet saved to disk and edits show as native diffs
- **Terminal**: The client runs commands in its own terminal, so output appears alongside the conversation

:::info Session Persistence
ACP sessions are saved to gosling's session history where you can access and manage them using gosling. Access to session history in ACP clients might vary.
:::

:::tip Reference Implementation
The [gosling for VS Code](/docs/experimental/vs-code-extension) extension uses ACP to communicate with gosling. See the [vscode-gosling](https://github.com/repo-makeover/vscode-gosling) repository for implementation details.
:::

## Setup in ACP Clients
Any editor or IDE that supports ACP can connect to gosling as an agent server. Check the [official ACP clients list](https://agentclientprotocol.com/overview/clients) for available clients with links to their documentation.

### Example: Zed Editor Setup

ACP was originally developed by [Zed](https://zed.dev/). Zed offers two ways to add gosling, and you can use either one.

#### Option 1: Install from the ACP Registry (recommended)

gosling is published in the [ACP Registry](https://agentclientprotocol.com/registry), and Zed 1.5.0 and later has built-in registry support, so it can download and run gosling for you, with no manual configuration and no pre-installed CLI required.

1. Open Zed
2. Open Agent Settings
3. Click `Add Agent`, then choose `Install from Registry`
4. Select `gosling`

A registry-installed gosling runs the same `gosling acp` server and reads your existing gosling configuration, so your providers, models, and extensions carry over. Zed keeps the installed version up to date for you.

#### Option 2: Configure gosling as a Custom Agent

Use a custom agent if you want to run your own gosling binary (for example, a local development build) or pass environment overrides.

##### Prerequisites

Ensure you have both Zed and gosling CLI installed:

- **Zed**: Download from [zed.dev](https://zed.dev/)
- **gosling CLI**: Follow the [installation guide](/docs/getting-started/installation)

  - Verify gosling is installed: `gosling --version`

  - Temporarily run `gosling acp` to test that ACP support is working:

    ```bash
    gosling acp
    ```

    Press `Ctrl+C` to exit the test.

##### Add gosling to Your Zed Settings

1. Open Zed
2. Open Agent Settings, click `Add Agent`, then choose `Add Custom Agent`. Zed scaffolds an `agent_servers` entry and opens your settings file
3. Edit the entry so it runs gosling:

```json
{
  "agent_servers": {
    "gosling": {
      "type": "custom",
      "command": "gosling",
      "args": ["acp"]
    }
  },
}
```

You should now be able to interact with gosling directly in Zed. Your ACP sessions use the same extensions that are enabled in your gosling configuration, and your tools (Developer, Computer Controller, etc.) work the same way as in regular gosling sessions.

#### Start Using gosling in Zed

After adding gosling with either option above:

1. **Open the Agent Panel**: Click the sparkles agent icon in Zed's status bar
2. **Create New Thread**: Click the `+` button to show thread options
3. **Select gosling**: Choose `New gosling` to start a new conversation with gosling
4. **Start Chatting**: Interact with gosling directly from the agent panel

#### Advanced Configuration

##### Overriding Provider and Model

By default, gosling will use the provider and model defined in your [configuration file](/docs/guides/config-files). You can override this for specific ACP configurations using the `GOSLING_PROVIDER` and `GOSLING_MODEL` environment variables.

The following Zed settings example configures two gosling agent instances. This is useful for:
- Comparing model performance on the same task
- Using cost-effective models for simple tasks and powerful models for complex ones

```json
{
  "agent_servers": {
    "gosling": {
      "type": "custom",
      "command": "gosling",
      "args": ["acp"]
    },
    "gosling (GPT-4o)": {
      "type": "custom",
      "command": "gosling",
      "args": ["acp"],
      "env": {
        "GOSLING_PROVIDER": "openai",
        "GOSLING_MODEL": "gpt-4o"
      }
    }
  },
}
```

## Using MCP Servers from ACP Clients

MCP servers configured in the ACP client's `context_servers` are automatically available to gosling. This allows you to use those MCP servers when using both native client features and the gosling agent integration.

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
    "gosling": {
      "type": "custom",
      "command": "gosling",
      "args": ["acp"]
    }
  },
}
```

To find out what tools are available, just ask gosling while it's running in the client.

:::info
All MCP servers in `context_servers` are automatically available to gosling, provided that they use stdio (command-based) or HTTP transports. gosling doesn't support servers that use the deprecated SSE transport.

If a server in `context_servers` has the same name as a gosling extension, gosling uses its own [configuration](/docs/guides/config-files).
:::

## TUI Client

For terminal-based workflows, gosling provides a TUI (Terminal User Interface) client that communicates with gosling via ACP. This is useful for developers who prefer working entirely in the terminal or need a lightweight alternative to the desktop app.

### Features

- **Full terminal-based chat interface** - Interactive conversation UI rendered directly in your terminal
- **Real-time streaming responses** - See gosling's responses as they're generated
- **Tool call visualization** - View tool executions with status indicators, inputs, and outputs
- **Permission dialogs** - Approve or reject tool permissions inline
- **Keyboard navigation** - Navigate conversation history and scroll through responses
- **Markdown rendering** - Formatted output for code blocks, lists, and other markdown elements
- **Message queuing** - Queue messages while gosling is processing

### Installation

```bash
cd ui/text
npm install
```

### Running the TUI

**Option 1: Auto-launch server (recommended)**

The TUI will automatically start the gosling acp server if you have it installed:

```bash
npm start
```

**Option 2: Connect to a custom server**

For servers that support the draft standard ACP over Streamable HTTP https://github.com/agentclientprotocol/agent-client-protocol/pull/721

```bash
npm start -- --server http://HOST:PORT

# example server
GOSLING_SERVER__SECRET_KEY='a-long-random-secret' cargo run -p gosling-cli --bin gosling -- serve
```

### Server Authentication

Set the `GOSLING_SERVER__SECRET_KEY` environment variable to authenticate the ACP endpoint. `gosling serve` refuses to start without this secret unless you explicitly pass `--dangerously-unauthenticated`:

```bash
GOSLING_SERVER__SECRET_KEY='a-long-random-secret' gosling serve
```

Clients authenticate by sending the token in the `X-Secret-Key` header, or as a `?token=` query parameter for WebSocket connections (the browser WebSocket API can't set custom headers). Requests without a matching token receive `401 Unauthorized`, including WebSocket handshakes.

ACP WebSocket Origin validation allows loopback web origins by default. For `gosling serve`, ACP CORS follows the same policy. If you pass any `--allowed-origin` values, that explicit list replaces the default loopback origins, so include every origin the client needs:

```bash
GOSLING_SERVER__SECRET_KEY='a-long-random-secret' gosling serve \
  --allowed-origin 'http://localhost:5173' \
  --allowed-origin 'app://localhost' \
  --allowed-origin 'https://app.example'
```

For local development only, `gosling serve --dangerously-unauthenticated` starts without a secret and logs a warning. Do not use this mode with shell-capable builtins enabled unless the server is isolated from untrusted browser traffic.

### Single Prompt Mode

Send a single prompt and exit (useful for scripting):

```bash
npm start -- --text "What files are in this directory?"
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Enter` | Send message |
| `↑` / `↓` | Scroll current response |
| `Shift+↑` / `Shift+↓` | Navigate conversation history |
| `Tab` | Expand/collapse tool call details |
| `Ctrl+C` or `Esc` | Exit (or cancel permission dialog) |

### Permission Dialog

When gosling requests permission to use a tool, a dialog appears with these options:

| Key | Action |
|-----|--------|
| `y` | Allow once |
| `a` | Always allow |
| `n` | Reject once |
| `N` | Always reject |
| `↑` / `↓` | Navigate options |
| `Enter` | Confirm selection |
| `Esc` | Cancel |

## Additional Resources

import ContentCardCarousel from '@site/src/components/ContentCardCarousel';

<ContentCardCarousel
  items={[
    {
      type: 'video',
      title: 'Intro to Agent Client Protocol (ACP) | Vibe Code with gosling',
      description: 'Watch how ACP lets you seamlessly integrate gosling into your code editor to streamline fragmented workflows.',
      thumbnailUrl: 'https://img.youtube.com/vi/Hvu5KDTb6JE/maxresdefault.jpg',
      linkUrl: 'https://www.youtube.com/watch?v=Hvu5KDTb6JE',
      date: '2025-10-16',
      duration: '50:23'
    }
  ]}
/>
