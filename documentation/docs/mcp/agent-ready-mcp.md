---
title: Agent Ready Extension
description: Add Agent Ready MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Agent Ready MCP Server](https://github.com/mlava/agent-ready-mcp) as a goose extension to scan any URL for AI agent-readability against the Vercel Agent Readability Spec, the llmstxt.org standard, and agent-protocol manifests (MCP server cards, A2A, agents.json, agent-permissions.json, UCP, x402, NLWeb) — 60 checks with per-check fix guidance.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=agent-ready-mcp%40latest&id=agent-ready&name=Agent%20Ready&description=Scan%20any%20URL%20for%20AI%20agent-readability%20against%20the%20Vercel%20spec%2C%20llms.txt%2C%20MCP%20cards%2C%20A2A%2C%20and%20agents.json.&env=AGENT_READY_API_KEY%3DAgent%20Ready%20Pro%20API%20key&timeout=300)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y agent-ready-mcp@latest
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  AGENT_READY_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`. You'll also need an Agent Ready **Pro** account — sign up at [agent-ready.dev](https://agent-ready.dev), upgrade to Pro, then issue a key from the [dashboard](https://agent-ready.dev/dashboard/api-keys).
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="agent-ready"
      extensionName="Agent Ready"
      description="Scan any URL for AI agent-readability — Vercel spec, llms.txt, MCP cards, A2A, agents.json. 60 checks with fix guidance."
      type="stdio"
      command="npx"
      args={["-y", "agent-ready-mcp@latest"]}
      timeout={300}
      envVars={[
        { name: "AGENT_READY_API_KEY", label: "Agent Ready Pro API key" }
      ]}
      apiKeyLink="https://agent-ready.dev/dashboard/api-keys"
      apiKeyLinkText="Agent Ready Pro API Key"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Agent Ready"
      description="Scan any URL for AI agent-readability — Vercel spec, llms.txt, MCP cards, A2A, agents.json. 60 checks with fix guidance."
      type="stdio"
      command="npx -y agent-ready-mcp@latest"
      timeout={300}
      envVars={[
        { key: "AGENT_READY_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          Get your API key from{" "}
          <a href="https://agent-ready.dev/dashboard/api-keys" target="_blank" rel="noopener noreferrer">
            agent-ready.dev
          </a> and paste it in.
        </>
      }
    />
  </TabItem>
</Tabs>

## What You Can Do

Agent Ready turns any URL into an actionable agent-readability report and surfaces three workflow prompts your agent can chain end-to-end.

### Score a site

Run the full 60-check scan against any public URL — Vercel Agent Readability Spec, llmstxt.org, AGENTS.md, MCP server cards (SEP-1649), A2A agent cards, agents.json, agent-permissions.json, UCP, x402, and NLWeb. Returns the overall score, per-check status, and a shareable result URL.

**Prompt:**

```
Scan https://example.com for AI agent-readability and tell me the three highest-impact things to fix first.
```

### Interpret an existing scan

If you already have a scan id (from a previous run or a teammate's share URL), `get_scan` returns the full breakdown and the `interpret_scan` prompt walks the agent through prioritising the failing checks.

**Prompt:**

```
Pull scan scan_01HXYZ... and explain in plain English what the failed checks mean and which ones block AI citations.
```

### Generate a remediation plan

The `remediation_plan` prompt synthesises the failing checks into a concrete fix order — copy-pasteable code samples for llms.txt entries, AGENTS.md sections, MCP server cards, and JSON-LD blocks.

**Prompt:**

```
Scan https://my-startup.com, then give me a prioritised remediation plan with code snippets I can paste into our Next.js repo.
```

### Search Agent Ready's own docs

The `ask` tool runs an NLWeb-compatible semantic search over Agent Ready's methodology, glossary, and guides — no API key required for this tool. Useful when your agent needs to look up what a specific check measures or how a spec is defined.

**Prompt:**

```
What does the L8 check measure, and how do I pass it?
```
