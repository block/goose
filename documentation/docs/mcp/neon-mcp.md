---
title: Neon Extension
description: Add Neon MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Neon MCP Server](https://github.com/neondatabase-labs/mcp-server-neon) as a Goose extension to interact with your Neon Postgres databases and manage your projects, branches, and more.

## Configuration

These steps configure the Remote MCP Server. You'll be redirected to neon.com to sign in to your Neon account.

:::tip Remote MCP Server
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
  https://mcp.neon.tech/mcp
  ```
:::

Alternatively, you can also use Neon's Local MCP server, using an API key to connect with a specific organization or your personal account.

Prerequisites:

- Node.js (>= v18.0.0): Ensure Node.js version 18 or higher is installed on your system. You can download it from [nodejs.org](https://nodejs.org/).
- Neon API Key: You will need a Neon API key to authenticate the Neon MCP Server with your Neon account. You can create one from the [Neon Console](https://console.neon.tech) under your Profile settings. Refer to the [Neon documentation on API Keys](https://neon.com/docs/manage/api-keys#creating-api-keys) for detailed instructions.

:::tip Local MCP Server
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="neon"
      extensionName="Neon"
      description="Nanage your Neon Postgres databases, projects, and branches"
      command="npx"
      args={["-y", "@neondatabase/mcp-server-neon", "start", "<YOUR_NEON_API_KEY>"]}
    />
  </TabItem>

  <TabItem value="cli" label="Goose CLI (No Env Vars)">
    <CLIExtensionInstructions
      name="Neon MCP"
      command="npx -y @neondatabase/mcp-server-neon start <YOUR_NEON_API_KEY>"
      timeout={300}
    />
  </TabItem>

</Tabs>
:::

For all setup and configuration options, see the [official Neon MCP Server documentation](https://neon.com/docs/ai/neon-mcp-server).

## Example Usage 1

### Goose Prompt

```
List my Neon projects
```

### Goose Output

```
I'll help you list your Neon projects using the neon__list_projects tool. By default, it will show the first 10 projects.

Here are your Neon projects:

1. laravel-test (ID: mute-mountain-08595850)
- Region: AWS US East 2
- Created: July 23, 2025
- PostgreSQL: v17

2. rls-test (ID: noisy-hill-99207787)
- Region: AWS US East 2
- Created: July 20, 2025
- PostgreSQL: v17

3. neon-auth-test (ID: rapid-pond-81053507)
- Region: Azure East US 2
- Created: July 7, 2025
- PostgreSQL: v17

4. saas-starter-kit-nextjs (ID: misty-poetry-78149700)
- Region: AWS US West 2
- Created: June 23, 2025
- PostgreSQL: v17
- Note: Has logical replication enabled

5. neon-auth-mcp-oauth (ID: gentle-base-21700263)
- Region: AWS US West 2
- Created: June 4, 2025
- PostgreSQL: v17

All projects are running PostgreSQL 17 and have autoscaling configured between 1-2 compute units. Would you like more detailed information about any specific project?
```

## Example Usage 2

### Goose Prompt

```
What tables do I have in my neon-auth-mcp-oauth project?
```

### Goose Output

```
In your neon-auth-mcp-oauth project, there is one table:

users_sync in the neon_auth schema
```

## Example Usage 3

### Goose Prompt

```
Count the rows in the users_sync table.
```

### Goose Output

```
I'll count the rows in the neon_auth.users_sync table using a COUNT query.

Total Records: 1 row
Table Location: Schema neon_auth, Table users_sync
Project: neon-auth-mcp-oauth (Project ID: gentle-base-21700263)
Content: Contains one user record
```