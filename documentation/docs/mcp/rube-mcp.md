---
title: Rube Extension
description: Add Rube as a Goose Extension to connect with 500+ apps
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import { PanelLeft } from 'lucide-react';

This tutorial covers how to add [Rube](https://rube.app) as a Goose extension to enable seamless integration with 500+ apps including Slack, Gmail, Notion, Google Workspace, Microsoft Office, GitHub, and many more.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="rube"
      extensionName="Rube"
      description="Seamlessly connect across 500+ applications including Slack, Gmail, Notion, Google Workspace, Microsoft Office, GitHub, and many more"
      type="http"
      url="https://rube.app/mcp"
    />
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    Use `goose configure` to add a `Remote Extension (Streaming HTTP)` extension type with:
    
    **Endpoint URL**
    ```
    https://rube.app/mcp
    ```
    **Authentication**: OAuth browser authentication (no manual tokens required)
  </TabItem>
</Tabs>
:::

## What is Rube?

Rube is a platform powered by Composio that provides unified access to 500+ apps and services through a single integration. It enables seamless connectivity across different applications without additional setup, making it perfect for both individual users and teams. Rube provides a consistent interface for:

- **Communication**: Slack, Discord, Microsoft Teams, WhatsApp, Telegram
- **Productivity**: Gmail, Outlook, Google Workspace, Microsoft 365, Notion, Airtable
- **Development**: GitHub, GitLab, Jira, Linear, Figma
- **CRM & Sales**: Salesforce, HubSpot, Pipedrive
- **Finance**: Stripe, QuickBooks, PayPal
- **And 500+ more apps**

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `Extensions`
    3. Click `Add custom extension`
    4. On the `Add custom extension` modal, enter the following:
       - **Extension Name**: Rube
       - **Type**: Streamable HTTP
       - **Endpoint**: `https://rube.app/mcp`
       - **Timeout**: 300 (or adjust as needed)
    5. Click `Add Extension` to save the extension
    6. Navigate to the chat and start using Rube - OAuth authentication will happen automatically when needed

  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    <CLIExtensionInstructions
      name="rube"
      type="http"
      url="https://rube.app/mcp"
      timeout={300}
      infoNote="OAuth authentication will happen automatically in your browser when you first use Rube tools"
    />

  </TabItem>
</Tabs>

## Authentication

Rube uses OAuth browser authentication, which means:
- No manual API keys to manage
- Secure authentication handled automatically
- When you first use a Rube tool, your browser will open to authenticate with the relevant service
- Authentication tokens are securely stored and managed by Rube

## Troubleshooting

- **Extension not connecting**: Ensure you have a stable internet connection and that `https://rube.app` is accessible from your network.
- **Authentication issues**: If OAuth flows aren't working, try clearing your browser cache or using a different browser.
- **Tool timeouts**: Some operations with large datasets may take longer. You can increase the timeout value in the extension settings if needed.
- **Rate limits**: Rube respects the rate limits of individual services. If you hit limits, wait a few minutes before retrying.

## Getting Help

- Visit [rube.app](https://rube.app) for documentation and support for integration-specific questions
- Check the Rube status page for any service interruptions