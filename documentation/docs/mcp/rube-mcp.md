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

## Configuration

These steps configure Rube as a Remote MCP Server. Rube acts as a unified access layer that exposes integrations through the Model Context Protocol (MCP), enabling AI assistants to interact with various applications seamlessly.

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `Extensions`
    3. Click `Add custom extension`
    4. On the `Add custom extension` modal, enter the following:
       - **Extension Name**: Rube
       - **Type**: Streamable HTTP
       - **Endpoint**: `https://rube.app/mcp`
       - **Description**: Connect with 500+ apps like Slack, Gmail, Notion and more
    5. Click `Add Extension` to save the extension
    6. Navigate to the chat

  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    <CLIExtensionInstructions
      name="rube"
      type="http"
      url="https://rube.app/mcp"
      timeout={300}
    />

  </TabItem>
</Tabs>

## Available Integrations

Rube provides access to over 500 applications and services, including:

### **Communication & Collaboration**
- **Slack**: Send messages, manage channels, and automate workflows
- **Microsoft Teams**: Team communication and file sharing
- **Discord**: Community management and messaging
- **Zoom**: Meeting scheduling and management

### **Productivity & Project Management**
- **Notion**: Create pages, manage databases, and organize content
- **Trello**: Board and card management
- **Asana**: Task and project tracking
- **Monday.com**: Work management and team collaboration

### **Email & Calendar**
- **Gmail**: Email management and automation
- **Outlook**: Microsoft email and calendar integration
- **Google Calendar**: Event scheduling and management
- **Calendly**: Meeting scheduling automation

### **File Storage & Cloud Services**
- **Google Drive**: File storage and sharing
- **Dropbox**: Cloud file synchronization
- **OneDrive**: Microsoft cloud storage
- **Box**: Enterprise file sharing

### **Customer Relationship Management**
- **Salesforce**: CRM operations and data management
- **HubSpot**: Marketing and sales automation
- **Pipedrive**: Sales pipeline management
- **Zendesk**: Customer support and ticketing

### **Development & Code Management**
- **GitHub**: Repository management and code collaboration
- **GitLab**: DevOps and CI/CD workflows
- **Jira**: Issue tracking and agile project management
- **Confluence**: Team documentation and knowledge sharing

## Example Usage

After adding Rube as an extension, you can use natural language to interact with connected applications:

### Goose Prompt

```
Send a message to the #general channel in Slack saying "Meeting moved to 3 PM today" and also create a reminder in my Google Calendar for the team standup at 9 AM tomorrow.
```

### Goose Output

:::note Desktop

Rube will handle the authentication flow for each service as needed and execute the requested actions across multiple platforms seamlessly. The exact output will depend on your connected applications and their current state.

:::

## Authentication

When you first use Rube with a specific application:

1. Rube will prompt you to authenticate with the target service
2. Complete the OAuth flow in your browser
3. Future interactions with that service will use the stored credentials
4. You can manage connected applications through Rube's interface

## Benefits

- **Unified Access**: One extension to connect with hundreds of apps
- **Seamless Authentication**: OAuth flows handled automatically
- **Cross-Platform Actions**: Execute workflows spanning multiple applications
- **Natural Language Interface**: Describe what you want to accomplish in plain English
- **Always Up-to-Date**: New integrations added regularly without needing to update Goose

## Getting Started

1. Add the Rube extension using the configuration steps above
2. Start a conversation with Goose
3. Try commands like "Send an email via Gmail" or "Create a task in Notion"
4. Rube will guide you through any required authentication steps
5. Begin automating workflows across your connected applications

:::info
Rube continuously adds new integrations, so the list of available apps is always growing. Check [rube.app](https://rube.app) for the most up-to-date list of supported services.
:::
