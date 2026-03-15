---
title: WritBase Extension
description: Add WritBase MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add [WritBase](https://github.com/Writbase/writbase) as a goose extension to enable MCP-native task management for AI agent fleets.

WritBase provides a persistent task registry with scoped permissions, inter-agent delegation, and full provenance — purpose-built for multi-agent workflows.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2F%3Cproject-ref%3E.supabase.co%2Ffunctions%2Fv1%2Fmcp-server%2Fmcp&id=writbase&name=WritBase&description=MCP-native%20task%20management%20for%20AI%20agent%20fleets&header=Authorization%3DBearer%20wb_YOUR_KEY_ID_YOUR_SECRET)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streaming HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://<project-ref>.supabase.co/functions/v1/mcp-server/mcp
  ```
  </TabItem>
</Tabs>

  **Custom Request Header**
  ```
  Authorization: Bearer wb_<key_id>_<secret>
  ```
:::

## Prerequisites

1. A Supabase project with WritBase deployed
2. An agent key generated via the CLI

To get started:

```bash
npx writbase init
```

This interactive setup will configure your Supabase project and generate your first agent key.

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="writbase"
      extensionName="WritBase"
      description="MCP-native task management for AI agent fleets"
      type="http"
      url="https://<project-ref>.supabase.co/functions/v1/mcp-server/mcp"
      envVars={[
        { name: "Authorization", label: "Bearer wb_<key_id>_<secret>" }
      ]}
      apiKeyLink="https://github.com/Writbase/writbase"
      apiKeyLinkText="WritBase Setup Guide"
    />

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="writbase"
      description="MCP-native task management for AI agent fleets"
      type="http"
      url="https://<project-ref>.supabase.co/functions/v1/mcp-server/mcp"
      timeout={300}
      envVars={[
        { key: "Authorization", value: "Bearer wb_xxxx_xxxxxxxxxxxx" }
      ]}
      infoNote={
        <>
          Run <code>npx writbase init</code> to set up your project and generate an agent key. Replace <code>&lt;project-ref&gt;</code> in the URL with your Supabase project reference, and use your agent key as the <code>Bearer</code> token.
        </>
      }
    />

  </TabItem>
</Tabs>

## MCP Tools

WritBase exposes 11 MCP tools across two permission tiers:

### Worker Tools
| Tool | Description |
|------|-------------|
| `info` | Returns server info and the calling agent's permissions |
| `get_tasks` | Query tasks with filtering, sorting, and cursor pagination |
| `add_task` | Create a new task (requires `can_create` permission) |
| `update_task` | Update an existing task (requires `can_update` or `can_comment` permission) |

### Manager Tools
| Tool | Description |
|------|-------------|
| `manage_agent_keys` | Create, list, rotate, and deactivate agent keys |
| `manage_agent_permissions` | Grant or revoke scoped permissions for agents |
| `get_provenance` | View the full audit trail for any task |
| `manage_projects` | Create and manage projects |
| `manage_departments` | Create and manage departments within projects |
| `subscribe` | Register webhook subscriptions for task events |
| `discover_agents` | List agents and their capabilities in the workspace |

## Example Usage

### goose Prompt
```
Create a new task in the "backend" department titled "Implement rate limiting" with high priority, then list all open tasks assigned to me.
```

### goose Output

```
I'll create the task and then list your open tasks.

1. Creating the task:

─── add_task | writbase ──────────────────────────
title: Implement rate limiting
departmentId: backend-dept-id
priority: high

Task created successfully with ID: task-12345

2. Listing your open tasks:

─── get_tasks | writbase ──────────────────────────
status: ["todo", "in_progress", "blocked"]

Found 3 open tasks:
- [high] Implement rate limiting (todo)
- [medium] Update API documentation (in_progress)
- [low] Add integration tests (todo)
```

## Learn More

- [WritBase GitHub Repository](https://github.com/Writbase/writbase)
- License: Apache-2.0
