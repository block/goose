---
sidebar_position: 105
title: Using Goose in ACP Clients
sidebar_label: Goose in ACP Clients
---

Client applications that support the [Agent Client Protocol (ACP)](https://agentclientprotocol.com/) can connect natively to Goose. This integration allows you to seamlessly interact with Goose directly from the client.

:::warning Experimental Feature
ACP is an emerging specification that enables clients to communicate with AI agents like Goose. This feature has limited adoption and may evolve as the protocol develops.
:::

## How It Works
After you configure Goose as an agent in the ACP client, you gain access to Goose's core agent functionality, including its extensions and tools. 

The client manages the Goose lifecycle automatically, including:

- **Initialization**: The client runs the `goose acp` command to initialize the connection
- **Communication**: The client communicates with Goose over stdio using JSON-RPC
- **Multiple Sessions**: The client manages multiple concurrent Goose conversations simultaneously

:::info Session Persistence
ACP sessions are not currently persisted between client restarts or accessible from Goose session history.
:::

## Zed Editor Setup

[Zed](https://zed.dev/) is the primary ACP-compatible editor. Here's how to integrate Goose:

### 1. Prerequisites

Ensure you have both Zed and Goose CLI installed:

- **Zed**: Download from [zed.dev](https://zed.dev/)
- **Goose CLI**: Follow the [installation guide](/docs/getting-started/installation)

  - ACP support requires version 1.8.0 or later - check with `goose --version`. 

  - Temporarily run `goose acp` to test that ACP support is working:

    ```
    ~ goose acp
    Goose ACP agent started. Listening on stdio...
    ```

    Press `Ctrl+C` to exit the test.

### 2. Configure Goose as a Custom Agent

Add Goose to your Zed settings:

1. Open Zed
2. Press `Cmd+,` (macOS) or `Ctrl+,` (Linux/Windows) to open settings
3. Add the following configuration:

```json
{
  "agent_servers": {
    "Goose 🪿": {
      "command": "goose",
      "args": ["acp"],
      "env": {}
    }
  },
  // more settings
}
```

You should now be able to interact with Goose directly in Zed. Your ACP sessions use the same extensions that are enabled in your Goose configuration, and your tools (Developer, Computer Controller, etc.) work the same way as in regular Goose sessions.

### 3. Start Using Goose in Zed

1. **Open the Agent Panel**: Click the sparkles agent icon in Zed's status bar
2. **Create New Thread**: Click the `+` button to show thread options
3. **Select Goose**: Choose `New Goose 🪿 Thread` to start a new conversation with Goose
4. **Start Chatting**: Interact with Goose directly from the agent panel

### Advanced Configuration

By default, Goose will use the provider and model defined in your [configuration file](/docs/guides/config-files). You can override this for specific ACP configurations using the `GOOSE_PROVIDER` and `GOOSE_MODEL` environment variables.

The following Zed settings example configures two Goose agent instances. This is useful for:
- Comparing model performance on the same task
- Using cost-effective models for simple tasks and powerful models for complex ones

```json
{
  "agent_servers": {
    "Goose 🪿": {
      "command": "goose",
      "args": ["acp"],
      "env": {}
    },
    "Goose (GPT-4o)": {
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
