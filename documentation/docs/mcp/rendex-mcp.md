---
title: Rendex Extension
description: Add Rendex MCP Server as a goose Extension to screenshot, render, extract, and monitor any webpage
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Rendex MCP Server](https://github.com/copperline-labs/rendex-mcp) as a goose extension. Rendex gives goose one rendering platform for the web: capture screenshots and PDFs of any URL or raw HTML, render HTML/Markdown into branded PDF + PNG artifacts with a hosted share link, extract clean reader-mode text from a page, and monitor a URL on a schedule for visual or text changes — useful for archiving UIs, generating invoices and reports, producing OG images, giving goose a reliable "see the web" capability, and watching a page for changes without spinning up a full browser automation stack.

## Configuration

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.rendex.dev%2Fmcp&id=rendex-mcp&name=Rendex&description=Screenshot%2C%20render%2C%20extract%2C%20and%20monitor%20any%20webpage%20via%20AI%20agents&header=Authorization%3DBearer%20YOUR_RENDEX_API_KEY)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streamable HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://mcp.rendex.dev/mcp
  ```
  </TabItem>
</Tabs>

  **Custom Request Header**
  ```
  Authorization: Bearer <YOUR_RENDEX_API_KEY>
  ```
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="rendex-mcp"
      extensionName="Rendex"
      description="Screenshot, render, extract, and monitor any webpage via AI agents"
      type="http"
      url="https://mcp.rendex.dev/mcp"
      envVars={[
        { name: "Authorization", label: "Bearer YOUR_RENDEX_API_KEY" }
      ]}
      apiKeyLink="https://rendex.dev/dashboard/keys"
      apiKeyLinkText="Rendex API key"
    />
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Rendex"
      description="Screenshot, render, extract, and monitor any webpage via AI agents"
      type="http"
      url="https://mcp.rendex.dev/mcp"
      timeout={300}
      envVars={[
        { key: "Authorization", value: "Bearer rdx_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" }
      ]}
      infoNote={
        <>
          Obtain your <a href="https://rendex.dev/dashboard/keys" target="_blank" rel="noopener noreferrer">Rendex API key</a> and paste it in as the <code>Bearer</code> token. Free tier includes 100 renders/month, no credit card required.
        </>
      }
    />
  </TabItem>
</Tabs>

## Available tools

The Rendex extension exposes 13 tools:

| Tool | What it does |
|---|---|
| `rendex_screenshot` | Screenshot or PDF of any URL or raw HTML — full-page, dark mode, custom viewport, CSS/JS injection |
| `rendex_render_link` | Render a URL or HTML to a reusable hosted image URL (og:image / share image) |
| `rendex_extract` | Extract a page to clean reader-mode Markdown, JSON, or HTML |
| `render_artifact` | Turn Markdown or HTML into a branded PDF + PNG with a hosted share page in one call |
| `rendex_account` | Read your plan and this month's usage (read-only, costs no credits) |
| `watch_create`, `watch_test`, `watch_list`, `watch_get`, `watch_run`, `watch_runs`, `watch_update`, `watch_delete` | **Rendex Watch** — monitor a URL on a schedule and get notified on visual or text changes (full CRUD + run/test) |

## Example Usage

Let's use the Rendex extension to generate an invoice PDF from raw HTML and archive a product page.

### goose Prompt

```
1. Render this HTML as an A4 PDF with 1cm margins:

   <!doctype html>
   <html>
     <body style="font-family: system-ui; padding: 40px;">
       <h1>Invoice #0042</h1>
       <p>Bill to: Acme Corp</p>
       <table>
         <tr><td>MCP server listing</td><td>$500.00</td></tr>
         <tr><td>Total</td><td><b>$500.00</b></td></tr>
       </table>
     </body>
   </html>

2. Then take a full-page screenshot of https://news.ycombinator.com in dark mode

3. Return both results so I can save them
```

### goose Output

```
I'll generate the invoice PDF first, then capture the HN page.

[rendex_screenshot: format=pdf, pdfFormat=A4, pdfMargin={top: 1cm, ...}, html=<invoice markup>]
✓ PDF captured successfully (12,344 bytes)
  width: 1280  height: 800  capturedAt: 2026-04-15T12:34:56Z

[rendex_screenshot: url=https://news.ycombinator.com, fullPage=true, darkMode=true]
✓ Screenshot captured (PNG, 847KB)
  width: 1280  height: 4,321  waitStrategy: networkidle2

Both renders completed. You have:
  1. Invoice PDF (A4, 12.3KB) — base64 in result 1
  2. Dark-mode full-page screenshot of Hacker News (PNG, 847KB) — base64 in result 2
```

### Monitoring a page for changes

Rendex Watch lets goose keep an eye on a page and tell you when it changes.

```
Watch https://news.ycombinator.com for visual changes every 3 hours and
alert my webhook at https://example.com/hooks/hn when the front page changes.
```

```
[watch_create: url=https://news.ycombinator.com, diffMode=visual, intervalMinutes=180,
               webhookUrl=https://example.com/hooks/hn]
✓ Watch created (baseline captured now)
  id: wat_7f3a…  status: active  nextRun: 2026-04-15T15:34:56Z
```

## Pricing

Rendex is free to try — no credit card required for the free tier.

| Plan | Renders/Month | Rate limit |
|---|---|---|
| Free | 100 | 3/min |
| Starter | 10,000 | 60/min |
| Pro | 100,000 | 300/min |
| Enterprise | Custom | 1,000/min |

Get an API key at [rendex.dev](https://rendex.dev).

## Links

- **Website**: [rendex.dev](https://rendex.dev)
- **GitHub**: [copperline-labs/rendex-mcp](https://github.com/copperline-labs/rendex-mcp)
- **npm**: [`@copperline/rendex-mcp`](https://www.npmjs.com/package/@copperline/rendex-mcp)
- **Smithery**: [smithery.ai/server/copperline/rendex-mcp](https://smithery.ai/server/copperline/rendex-mcp)
- **Official MCP Registry**: `io.github.copperline-labs/rendex-mcp`
