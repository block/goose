---
title: Honcho Extension
description: Add Honcho MCP Server as a goose Extension for persistent memory and personalization
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Honcho MCP Server](https://docs.honcho.dev/v3/guides/integrations/mcp) as a goose extension to give goose a persistent memory layer. Honcho learns who you are, remembers your preferences, and builds a representation of you across every conversation.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.honcho.dev&id=honcho&name=Honcho&description=Honcho%20persistent%20memory%20and%20personalization&header=Authorization%3DBearer%20YOUR_HONCHO_API_KEY&header=X-Honcho-User-Name%3DYourName)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streaming HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://mcp.honcho.dev
  ```
  </TabItem>
</Tabs>

  **Custom Request Headers**
  ```
  Authorization: Bearer <YOUR_HONCHO_API_KEY>
  X-Honcho-User-Name: <YourName>
  ```
:::

## What is Honcho?

Honcho is an AI memory layer. Connect it once and goose gains persistent, cross-session memory: it remembers your preferences, communication style, and context, then uses Honcho's background reasoning to personalize its responses over time. Honcho is served as a hosted Streamable HTTP MCP server, so there's nothing to install or run locally.

## Configuration

:::info
You'll need an API key from [app.honcho.dev](https://app.honcho.dev) to use the hosted MCP server. It should start with `hch-`.
:::

Honcho requires two headers: `Authorization` (your API key) and `X-Honcho-User-Name` (what goose should call you).

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="honcho"
      extensionName="Honcho"
      description="Honcho persistent memory and personalization"
      type="http"
      url="https://mcp.honcho.dev"
      envVars={[
        { name: "Authorization", label: "Bearer YOUR_HONCHO_API_KEY" },
        { name: "X-Honcho-User-Name", label: "YourName" }
      ]}
      apiKeyLink="https://app.honcho.dev"
      apiKeyLinkText="Honcho API Key"
    />

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="honcho"
      description="Honcho persistent memory and personalization"
      type="http"
      url="https://mcp.honcho.dev"
      timeout={300}
      envVars={[
        { key: "Authorization", value: "Bearer hch-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" },
        { key: "X-Honcho-User-Name", value: "YourName" }
      ]}
      infoNote={
        <>
          Get your API key from <a href="https://app.honcho.dev" target="_blank" rel="noopener noreferrer">app.honcho.dev</a> and paste it in as the <code>Bearer</code> token.
        </>
      }
    />

  </TabItem>
</Tabs>

### Optional Headers

You can customize the assistant name and isolate memory per project by adding extra headers. Both are optional.

| Header | Default | Description |
|--------|---------|-------------|
| `Authorization` | *required* | `Bearer hch-your-key-here` |
| `X-Honcho-User-Name` | *required* | What goose should call you |
| `X-Honcho-Assistant-Name` | `"Assistant"` | Name for the AI peer (e.g. `Goose`) |
| `X-Honcho-Workspace-ID` | `"default"` | Isolate memory per project |

### Teaching goose the Memory Flow

For best results, give goose the recommended memory flow so it knows when to read from and write to Honcho. Save the Honcho [instructions](https://raw.githubusercontent.com/plastic-labs/honcho/refs/heads/main/mcp/instructions.md) into a `.goosehints` file in your goose config directory (`~/.config/goose/` on Linux) or a project root. This is goose's equivalent of Claude Desktop's "Project Instructions". Run `goose info` if you're not sure of your config path.

## Example Usage

Once Honcho is configured, goose sets up a session and pulls what it already knows about you before responding. On the first conversation there won't be much, but Honcho's background reasoning builds a representation of you over a few conversations.

### goose Prompt
```
Hii do u have a memory?
```

### goose Output

```
  ────────────────────────────────────────
  ▸ inspect_workspace | honcho

  ────────────────────────────────────────
  ▸ create_session | honcho
    session_id: session-2026-06-22-chat

  ────────────────────────────────────────
  ▸ get_peer_context | honcho
    peer_id: Assistant
    target_peer_id: user

  ────────────────────────────────────────
  ▸ add_peers_to_session | honcho
    session_id: session-2026-06-22-chat
    peers:
        - peer_id: user
          observe_me: true
          observe_others: true
        - peer_id: Assistant
          observe_me: false
          observe_others: true

Hey Aru! 👋 Yes, I do have a memory! 🧠

I use Honcho — a persistent memory layer — to remember things across
conversations. From what I've got stored, I already know your name is Aru!

Here's what my memory can do:
- Remember preferences — how you like things explained, coding style, etc.
- Learn over time — the more we chat, the better I understand you
- Persist across sessions — even in future conversations, I can recall context about you

Is there something specific you'd like me to remember, or shall we dive into some work? 😊

  ────────────────────────────────────────
  ▸ add_messages_to_session | honcho
    session_id: session-2026-06-22-chat
    messages:
        - peer_id: user
          content: Hii do u have a memory?
        - peer_id: Assistant
          content: Hey Aru! 👋 Yes, I do have a memory! ...
```

After responding, goose records the exchange back to Honcho with `add_messages_to_session` so your representation keeps improving across sessions.

## Verify It Works

After setup, ask goose:

> "What do you know about me?"

On the first conversation there won't be much. Ask again after a couple of conversations and you'll see Honcho's representation start to take shape.

## Troubleshooting

| Problem | Fix |
|---------|-----|
| Tools don't show up | Make sure the extension is enabled and you fully restarted goose after adding it. |
| Authorization errors | Check your API key at [app.honcho.dev](https://app.honcho.dev). It should start with `hch-`. |
| "No personalization insights found" | Normal for new users. Honcho needs a few conversations to build context. |
| Connection timeouts | Check that `https://mcp.honcho.dev` is accessible from your network. |

Need help? Join the Honcho community on [Discord](https://discord.gg/honcho) or open an issue on [GitHub](https://github.com/plastic-labs/honcho/tree/main/mcp).
