---
title: PostEverywhere Extension
description: Add PostEverywhere MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [PostEverywhere MCP Server](https://github.com/posteverywhere/mcp) as a goose extension to schedule and publish social media posts to 11 platforms (Instagram, TikTok, YouTube, LinkedIn, X, Facebook, Threads, Pinterest, Bluesky, Telegram, Discord) with media upload, AI captions, campaigns, and analytics.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40posteverywhere%2Fmcp&id=posteverywhere-mcp&name=PostEverywhere&description=schedule%20and%20publish%20social%20media%20posts%20to%2011%20platforms&env=POSTEVERYWHERE_API_KEY%3DPostEverywhere%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @posteverywhere/mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  POSTEVERYWHERE_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="posteverywhere-mcp"
    extensionName="PostEverywhere"
    description="Schedule and publish social media posts to 11 platforms"
    command="npx"
    args={["-y", "@posteverywhere/mcp"]}
    envVars={[
      { name: "POSTEVERYWHERE_API_KEY", label: "PostEverywhere API Key" }
    ]}
    apiKeyLink="https://app.posteverywhere.ai/settings?tab=developers"
    apiKeyLinkText="PostEverywhere API Key"
  />
  :::info
  Create an API key at [app.posteverywhere.ai](https://app.posteverywhere.ai/settings?tab=developers) under Settings, Developers. See the [authentication docs](https://developers.posteverywhere.ai/authentication) for details.
  :::
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  <CLIExtensionInstructions
    name="posteverywhere"
    command="npx -y @posteverywhere/mcp"
    envVars={[
      { key: "POSTEVERYWHERE_API_KEY", value: "<YOUR_API_KEY>" }
    ]}
  />
  </TabItem>
</Tabs>

## Example Usage

Ask goose to handle your social media directly:

1. "Schedule this announcement to LinkedIn, X, and Threads for tomorrow 9am."
2. "Upload this video and post it to TikTok, YouTube Shorts, and Instagram Reels."
3. "Write a caption for this image and publish it to all my connected accounts."
4. "Show me the analytics summary for last week's posts."

A hosted remote (Streamable HTTP with OAuth) is also available at `https://mcp.posteverywhere.ai/mcp` for clients that prefer remote extensions. Full documentation: [developers.posteverywhere.ai](https://developers.posteverywhere.ai/integrations/mcp).
