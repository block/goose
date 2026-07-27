---
title: Publora Extension
description: Add Publora MCP Server as a goose Extension to publish and schedule social media posts
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Publora MCP Server](https://github.com/publora/mcp-server) as a goose extension so goose can publish and schedule social media posts, manage drafts and media, and read LinkedIn analytics.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.publora.com%2Fmcp&id=publora&name=Publora&description=Publish%20and%20schedule%20social%20media%20posts%20across%2010%20networks&header=x-publora-key%3Dsk_YOUR_PUBLORA_API_KEY)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streamable HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://mcp.publora.com/mcp
  ```
  </TabItem>
</Tabs>

  **Custom Request Header**
  ```
  x-publora-key: sk_<YOUR_PUBLORA_API_KEY>
  ```
:::

## What is Publora?

[Publora](https://publora.com) is a social media publishing API and scheduler. Its remote MCP server lets an agent publish or schedule posts to LinkedIn, X, Instagram, Threads, TikTok, YouTube, Facebook, Bluesky, Mastodon and Telegram through one interface, instead of wiring up each network's own API and OAuth flow.

Publora is a commercial service with a free tier. You connect your social accounts once in the Publora dashboard, then create an API key under **Settings > API** and pass it to goose as the `x-publora-key` request header. The server acts on the accounts belonging to that key.

## Configuration

:::info
You need a [Publora](https://publora.com) account with at least one connected social account, and an API key from **Settings > API**.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="publora"
      extensionName="Publora"
      description="Publish and schedule social media posts across 10 networks"
      type="http"
      url="https://mcp.publora.com/mcp"
      envVars={[
        { name: "x-publora-key", label: "sk_YOUR_PUBLORA_API_KEY" }
      ]}
      apiKeyLink="https://publora.com"
      apiKeyLinkText="Publora API key"
    />

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Publora"
      description="Publish and schedule social media posts across 10 networks"
      type="http"
      url="https://mcp.publora.com/mcp"
      envVars={[
        { key: "x-publora-key", value: "sk_YOUR_PUBLORA_API_KEY" }
      ]}
      timeout={300}
      infoNote="Get your API key from the Publora dashboard under Settings > API."
    />
  </TabItem>
</Tabs>

## Available Tools

| Tool | Description |
|------|-------------|
| `list_connections` | List connected social accounts |
| `create_post` | Publish or schedule a post to one or more platforms |
| `list_posts` | List posts with filters (status, platform, dates) |
| `get_post` | Get post details and status |
| `update_post` | Reschedule a post or change its status |
| `delete_post` | Delete a post |
| `get_upload_url` | Get a presigned URL for media upload |
| `linkedin_post_stats` | Post engagement metrics |
| `linkedin_account_stats` | Account-level statistics |
| `linkedin_followers` | Follower count and growth |
| `linkedin_profile_summary` | Combined profile overview |
| `linkedin_create_comment` / `linkedin_delete_comment` | Comment on a LinkedIn post |
| `linkedin_create_reaction` / `linkedin_delete_reaction` | React to a LinkedIn post |

Comments and reactions are LinkedIn-only. The other nine networks support publishing and scheduling.

## Example Usage

Once Publora is configured, you can ask goose to publish on your behalf. Media-only networks such as Instagram, TikTok and YouTube require an image or video.

### Goose Prompt

```
Which social accounts do I have connected? Draft a short post about our new
changelog page and schedule it to LinkedIn and Bluesky for tomorrow at 9am UTC.
```

### Goose Output

```
I'll check your connected accounts first.

─── list_connections ───

You have LinkedIn, X, Bluesky and Telegram connected.

─── create_post ───

Scheduled for tomorrow at 09:00 UTC on LinkedIn and Bluesky:

"Our changelog now lives in one place — every shipped fix, in plain language,
with the date it landed. No more digging through release notes."

The post is in your Publora queue as scheduled. Want me to add an image or
adjust the wording before it goes out?
```
