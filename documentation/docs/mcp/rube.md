---
title: Rube
description: Add Rube as a Goose Extension to connect with 500+ apps
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import { PanelLeft } from 'lucide-react';

This tutorial covers how to add [Rube](https://rube.app) as a Goose extension to enable seamless integration with 500+ apps including Slack, Gmail, Notion, Google Workspace, Microsoft Office, GitHub, and many more.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  Use `Add custom extension` in Settings → Extensions to add a `Streamable HTTP` extension type with:
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  Use `goose configure` to add a `Remote Extension (Streaming HTTP)` extension type with:
  </TabItem>
</Tabs>

  **Endpoint URL**
  ```
  https://rube.app/mcp
  ```
  **Authentication**: OAuth browser authentication (no manual tokens required)
:::

## What is Rube?

Rube is a platform powered by Composio that provides unified access to 500+ apps and services through a single integration. It enables seamless connectivity across different applications without additional setup, making it perfect for both individual users and teams. Rube provides a consistent interface for:

- **Communication**: Slack, Discord, Microsoft Teams, WhatsApp, Telegram
- **Productivity**: Gmail, Outlook, Google Workspace, Microsoft 365, Notion, Airtable
- **Development**: GitHub, GitLab, Jira, Linear, Figma
- **CRM & Sales**: Salesforce, HubSpot, Pipedrive
- **Finance**: Stripe, QuickBooks, PayPal
- **And 500+ more apps

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

## Example Usage

### Email Management
```
Send an email to john@example.com with the subject "Project Update" and tell him about our progress on the new feature.
```

### Slack Integration
```
Post a message to the #general channel in Slack about the team meeting tomorrow at 2 PM.
```

### Google Sheets Operations
```
Create a new Google Sheet called "Q1 Sales Data" and add headers for Date, Product, Revenue, and Customer.
```

### GitHub Integration
```
Create a new issue in my project repository titled "Bug: Login form validation" and assign it to the development team.
```

### Multi-App Workflows
```
Get my latest emails from Gmail, summarize them, and post a daily digest to our team Slack channel.
```

### Notion Database Management
```
Add a new entry to my "Project Tasks" Notion database with the task "Implement user authentication" marked as high priority.
```

## Supported Apps

Rube provides access to 500+ applications across categories including:

**Communication & Collaboration**
- Slack, Discord, Microsoft Teams
- Gmail, Outlook, Sendbird
- WhatsApp, Telegram, Superchat

**Productivity & Organization**
- Google Workspace (Docs, Sheets, Drive, Calendar)
- Microsoft 365 (Word, Excel, OneDrive, Outlook)
- Notion, Airtable, Monday.com
- Trello, Asana, ClickUp

**Development & Design**
- GitHub, GitLab, Bitbucket
- Jira, Linear, Figma

**CRM & Sales**
- Salesforce, HubSpot, Pipedrive
- Zendesk, Intercom, Freshworks
- Stripe, Apollo, Attio

**And many more!**

## Benefits of Using Rube

1. **Unified Interface**: One extension to access 500+ apps instead of managing multiple MCP servers
2. **Simplified Authentication**: OAuth flows handled automatically - no manual API key management
3. **Cross-App Workflows**: Easily chain actions across different services
4. **Always Up-to-Date**: New app integrations and API updates handled by the Rube team
5. **Secure**: Enterprise-grade security with OAuth 2.0 and secure token management

## Troubleshooting

**Extension not connecting**: Ensure you have a stable internet connection and that `https://rube.app` is accessible from your network.

**Authentication issues**: If OAuth flows aren't working, try clearing your browser cache or using a different browser.

**Tool timeouts**: Some operations with large datasets may take longer. You can increase the timeout value in the extension settings if needed.

**Rate limits**: Rube respects the rate limits of individual services. If you hit limits, wait a few minutes before retrying.

## Getting Help

- Visit [rube.app](https://rube.app) for documentation and support
- Check the Rube status page for any service interruptions
- Contact Rube support for integration-specific questions