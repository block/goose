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

## Example Usage

### Creating and tracking tasks

#### goose Prompt

> _goose, create a high-priority task titled "Implement rate limiting" in the backend department, then show me all open tasks._

#### goose Output

:::note CLI

<details>
    <summary>Tool Calls</summary>

    ─── add_task | writbase ──────────────────────────

    title: Implement rate limiting

    departmentSlug: backend

    priority: high


    ─── get_tasks | writbase ──────────────────────────

    status: ["todo", "in_progress", "blocked"]

</details>

Task created successfully:
- **Title:** Implement rate limiting
- **Priority:** high
- **Department:** backend
- **Status:** todo
- **ID:** task-a1b2c3d4

Here are all your open tasks:

| Priority | Task | Status |
|----------|------|--------|
| high | Implement rate limiting | todo |
| medium | Update API documentation | in_progress |
| low | Add integration tests | todo |

:::

### Updating task status

#### goose Prompt

> _goose, mark the "Implement rate limiting" task as in progress and add a note that work has started on the middleware layer._

#### goose Output

:::note CLI

<details>
    <summary>Tool Calls</summary>

    ─── get_tasks | writbase ──────────────────────────

    search: Implement rate limiting

    status: ["todo"]


    ─── update_task | writbase ──────────────────────────

    taskId: task-a1b2c3d4

    status: in_progress

    notes: Work started on the middleware layer

</details>

Done! Updated "Implement rate limiting":
- **Status:** todo → in_progress
- **Note added:** Work started on the middleware layer

:::

## Learn More

- [WritBase GitHub Repository](https://github.com/Writbase/writbase)
- License: Apache-2.0
