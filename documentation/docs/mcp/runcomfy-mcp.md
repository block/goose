---
title: RunComfy Extension
description: Add RunComfy MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [RunComfy MCP Server](https://github.com/runcomfy-com/runcomfy-mcp) as a goose extension, enabling goose to manage RunComfy Serverless (ComfyUI) GPU deployments and run async image/video inference.

RunComfy MCP is a remote, hosted server at `https://mcp.runcomfy.com/mcp`. goose connects to it natively as a **Streamable HTTP** extension — no local Node.js required.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  Add it from **Settings → Extensions → Add**, or use the goose CLI command.
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Run `goose configure` → **Add Extension** → **Remote Extension (Streamable HTTP)**, URL `https://mcp.runcomfy.com/mcp`, with header `Authorization: Bearer <YOUR_RUNCOMFY_TOKEN>`.
  </TabItem>
</Tabs>
:::

## Configuration

:::info
No local runtime required — RunComfy is a native Streamable HTTP extension. You'll need a RunComfy API token — get one from your [RunComfy Profile](https://www.runcomfy.com/profile) — and send it as an `Authorization: Bearer <YOUR_RUNCOMFY_TOKEN>` request header.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
        extensionId="runcomfy"
        extensionName="RunComfy"
        description="Manage RunComfy Serverless (ComfyUI) GPU deployments and run async image/video inference"
        type="http"
        url="https://mcp.runcomfy.com/mcp"
        timeout={300}
        envVars={[
          { name: "Authorization", label: "Bearer <YOUR_RUNCOMFY_TOKEN>" }
        ]}
        apiKeyLink="https://www.runcomfy.com/profile"
        apiKeyLinkText="RunComfy API token"
        note="Remote Streamable HTTP server — no Node.js required. Provide your RunComfy token as an Authorization header."
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
      <CLIExtensionInstructions
        name="RunComfy"
        description="Manage RunComfy Serverless (ComfyUI) GPU deployments and run async image/video inference"
        type="http"
        url="https://mcp.runcomfy.com/mcp"
        timeout={300}
        envVars={[
          { key: "Authorization", value: "Bearer <YOUR_RUNCOMFY_TOKEN>" }
        ]}
        infoNote="Get your RunComfy API token from https://www.runcomfy.com/profile"
      />
  </TabItem>
</Tabs>

## Available Tools

The RunComfy MCP server exposes 10 tools mapping 1:1 to the [RunComfy Serverless API (ComfyUI)](https://docs.runcomfy.com/serverless):

- **Deployment management**: `list_deployments`, `get_deployment`, `create_deployment`, `update_deployment`, `delete_deployment`
- **Inference**: `submit_request`, `get_request_status`, `get_request_result`, `cancel_request`
- **Advanced**: `call_instance_proxy`

## Example Usage

#### Prompt

```
List my RunComfy deployments, submit an inference request to the first one, and give me the output image URL when it's done.
```

#### goose Output

```
I'll list your RunComfy deployments, then submit a request.

─── list_deployments | runcomfy ──────────────────────────
Found 2 deployment(s).

─── submit_request | runcomfy ──────────────────────────
deployment_id: 6f3a1b...
Submitted request req_abc123.

─── get_request_status | runcomfy ──────────────────────────
Request req_abc123: completed.

Here is your generated image:
https://output.runcomfy.net/.../result.png
```
