---
title: Actionbook Extension
description: Add Actionbook MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Actionbook MCP Server](https://actionbook.dev) as a goose extension to let goose operate any website through your own logged-in Chrome session — do web research, fill forms, and run repetitive tasks on the sites you already use.

Actionbook ships as a Chrome extension plus a cloud MCP endpoint. Agents call a single MCP tool (`actionbook`) with a CLI-style command string covering browser control and action-library lookup, so goose can drive real pages with verified selectors instead of exploring the DOM from scratch.

:::info Prerequisites
1. Install the [Actionbook Chrome extension](https://actionbook.dev) and sign in.
2. Generate an API key from the [Actionbook dashboard](https://actionbook.dev/dashboard).
:::

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fedge.actionbook.dev%2Fmcp&id=actionbook&name=Actionbook&description=Let%20your%20AI%20agent%20operate%20any%20website%20through%20your%20own%20logged-in%20Chrome%20session&header=Authorization%3DBearer%20YOUR_ACTIONBOOK_API_KEY)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Use `goose configure` to add a `Remote Extension (Streaming HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://edge.actionbook.dev/mcp
  ```
  </TabItem>
</Tabs>

  **Custom Request Header**
  ```
  Authorization: Bearer <YOUR_ACTIONBOOK_API_KEY>
  ```
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="actionbook"
      extensionName="Actionbook"
      description="Let your AI agent operate any website through your own logged-in Chrome session"
      type="http"
      url="https://edge.actionbook.dev/mcp"
      envVars={[
        { name: "Authorization", label: "Bearer YOUR_ACTIONBOOK_API_KEY" }
      ]}
      apiKeyLink="https://actionbook.dev/dashboard"
      apiKeyLinkText="Actionbook API Key"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="actionbook"
      description="Let your AI agent operate any website through your own logged-in Chrome session"
      type="http"
      url="https://edge.actionbook.dev/mcp"
      timeout={300}
      envVars={[
        { key: "Authorization", value: "Bearer ak_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" }
      ]}
      infoNote={
        <>
          Generate your API key from the{" "}
          <a href="https://actionbook.dev/dashboard" target="_blank" rel="noopener noreferrer">
            Actionbook dashboard
          </a>{" "}
          and paste it in as the <code>Bearer</code> token.
        </>
      }
    />
  </TabItem>
</Tabs>

For all setup and configuration options, see the [official Actionbook documentation](https://actionbook.dev).

## Example Usage

Actionbook exposes a single `actionbook` tool that accepts CLI-style commands for browser control (navigate, click, type, read pages) and action-library lookup (pre-verified selectors and step-by-step manuals for popular websites). Because it drives your own logged-in Chrome session, goose can work with sites you're already signed in to — no separate credentials needed.

### goose Prompt

```
Use Actionbook to open Hacker News and tell me the title and a one-sentence summary of the top story right now.
```

### goose Output

```
─── actionbook ──────────────────────────
cmd: browser goto https://news.ycombinator.com/ --tab hn

─── actionbook ──────────────────────────
cmd: code (read the top story from the page)

─── actionbook ──────────────────────────
cmd: browser goto https://blog.cloudflare.com/voidzero-joins-cloudflare/ --tab article

─── actionbook ──────────────────────────
cmd: code (read the article)

Top Hacker News story: "VoidZero Is Joining Cloudflare"

Summary: VoidZero, the company behind Vite, Vitest, Rolldown, Oxc, and Vite+, is
joining Cloudflare, while emphasizing that its tools will remain open source,
vendor-agnostic, and community-driven.
```
