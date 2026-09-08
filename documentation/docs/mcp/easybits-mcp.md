---
title: EasyBits Extension
description: Add EasyBits MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [EasyBits MCP Server](https://github.com/blissito/easybits) as a goose extension to give your agent its own cloud: reading any page on the web even when it blocks bots, Firecracker microVM sandboxes, file storage on a CDN, SQL databases, generated documents, video, and app hosting — all behind a single endpoint.

:::tip Quick Install

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40easybits.cloud%2Fmcp&arg=--tools&arg=core&id=easybits-mcp&name=EasyBits&description=The%20cloud%20for%20AI%20agents%3A%20sandboxes%2C%20web%2C%20files%2C%20SQL%20databases%2C%20documents%20and%20app%20hosting&env=EASYBITS_API_KEY%3DEasyBits%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @easybits.cloud/mcp --tools core
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  EASYBITS_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="easybits-mcp"
      extensionName="EasyBits"
      description="The cloud for AI agents: sandboxes, web, files, SQL databases, documents and app hosting"
      type="stdio"
      command="npx"
      args={["-y", "@easybits.cloud/mcp", "--tools", "core"]}
      envVars={[
        { name: "EASYBITS_API_KEY", label: "EasyBits API Key" }
      ]}
      apiKeyLink="https://www.easybits.cloud/dash/developer"
      apiKeyLinkText="EASYBITS_API_KEY"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="easybits"
      description="The cloud for AI agents: sandboxes, web, files, SQL databases, documents and app hosting"
      type="stdio"
      command="npx -y @easybits.cloud/mcp --tools core"
      timeout={300}
      envVars={[
        { key: "EASYBITS_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          Get your API key from your{" "}
          <a href="https://www.easybits.cloud/dash/developer" target="_blank" rel="noopener noreferrer">
            EasyBits developer dashboard
          </a> and paste it in.
        </>
      }
    />
  </TabItem>
</Tabs>

### Choosing a toolset

EasyBits exposes 247 tools. Loading all of them at once would flood the model's
context, so the server ships **toolsets** — pass `--tools <group>` and only that
group is registered:

| Group | What you get |
|---|---|
| `core` | The recommended default: files, SQL databases, documents, forms, websites, brand kits, images, and web search |
| `web` | Search, and read pages that block ordinary scrapers — plus structured extraction and crawling |
| `sandbox` | Firecracker microVMs — spawn, run commands, expose ports, snapshot, suspend, destroy |
| `hosting` | Deploy and run an app on an always-on microVM with a public URL |
| `design` | Documents as universal design (letter, social, 16:9 slides), brand kits, images |
| `docs` | Documents only — create, update, deploy, export to PDF/PNG |
| `video` | AI video, recurring characters, voice-over and captions |
| `all` | Everything, if your model can take it |

Groups can be combined: `--tools core,sandbox`.

## Example Usage

A common goose task is "go read what's actually out there, then hand me
something I can send to someone." With EasyBits, goose does the whole loop
without leaving the chat: it searches the live web, reads pages that normally
block scrapers, writes the findings into a document, exports a PDF, and
publishes the same document at a public URL.

### goose Prompt

```
Research the current pricing of the three biggest managed Postgres providers,
then build me a comparison document with our brand kit applied,
export it as a PDF, and publish it so I can share the link.
```

### goose Output

:::note Desktop

I researched the providers and built the document.

**1. Research** — `web_search` and `web_fetch` pulled the current pricing
pages for all three providers, including the two that block plain HTTP clients.

**2. Document** — created *Managed Postgres — Pricing Comparison* with three
pages: an intro, a side-by-side pricing table, and a recommendation. Your
default brand kit was applied, so it uses your palette and typefaces.

**3. Export** — `export_document` produced the PDF — three pages, letter format.

**4. Publish** — `deploy_document` put it online.

Here's everything:

- Live page: `https://www.easybits.cloud/s/managed-postgres-pricing-comparison`
- PDF: the signed download URL is valid for 1 hour — say the word and I'll
  re-issue it, or `create_share_link` will give you a permanent one.

A note from the research: the entry tiers look comparable, but two of the
three bill storage and egress separately, so the cheapest headline price is
the most expensive of the three once you pass ~100 GB of transfer. I put that
in the recommendation page rather than burying it in the table.

Want me to also spin up a sandbox and benchmark them for real?

:::

### Reading the web when the web says no

Search is the easy half. The half that breaks agents is the fetch: the page
that returns a Cloudflare challenge, the listing behind a bot check, the
marketplace that serves one thing to a browser and another to `curl`. EasyBits
routes those requests through unblocking infrastructure, so `web_fetch` comes
back with the real page as clean text instead of an interstitial.

With `--tools web` you get:

| Tool | What it does |
|---|---|
| `web_search` | Live search results, including SERP at volume |
| `web_fetch` | One URL to readable text — works on pages that reject plain HTTP clients |
| `web_extract` / `web_extract_status` | Structured extraction from a page into fields you name; long jobs run async |
| `web_crawl` | Follow a site and bring back many pages in one call |

Because it is the same MCP endpoint, what goose reads it can immediately keep:
pipe a crawl into `db_create` and query it as SQL, or into `create_document`
and hand back a PDF. The reading and the artifact do not live in two different
extensions.

### Beyond documents

The same key reaches the rest of the platform once you widen the toolset:

- **`--tools sandbox`** — goose gets a real Firecracker microVM. It can install
  dependencies, run your test suite, expose a port over HTTPS (and `wss://`),
  snapshot the machine and fork it.
- **`--tools hosting`** — one call takes a repo or an archive to a running,
  always-on app with a public URL.
- **`--tools web`** — search, unblocked fetching, structured extraction and
  crawling, as described above.
