---
title: EasyBits Extension
description: Add EasyBits MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [EasyBits MCP Server](https://github.com/blissito/easybits) as a goose extension to give your agent its own cloud: Firecracker microVM sandboxes where it can run code and host apps on a public URL, reading pages that block bots, file storage on a CDN, SQL databases, generated documents and video — all behind a single endpoint. It also runs goose itself: `goose_spawn` boots a managed goose in a microVM in one call.

:::tip Quick Install

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40easybits.cloud%2Fmcp&arg=--tools&arg=core&id=easybits-mcp&name=EasyBits&description=Give%20your%20agent%20a%20Firecracker%20microVM%3A%20run%20code%2C%20host%20apps%2C%20read%20the%20web%2C%20store%20files%20and%20query%20SQL%20databases&env=EASYBITS_API_KEY%3DEasyBits%20API%20Key)
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
      description="Give your agent a Firecracker microVM: run code, host apps, read the web, store files and query SQL databases"
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
      description="Give your agent a Firecracker microVM: run code, host apps, read the web, store files and query SQL databases"
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

goose already edits files and runs commands — on *your* machine. What it does
not have is somewhere else to do it. EasyBits gives it a Firecracker microVM:
a real Linux box, a few seconds to boot, that survives closing your laptop and
can be handed a public HTTPS URL.

That changes what you can ask for.

:::info
This example uses the sandbox tools, so start the extension with
`--tools core,sandbox` instead of the default `--tools core`.
:::

### goose Prompt

```
Clone github.com/my-org/my-app into a sandbox, install and run the test suite
there, and if it's green start the dev server and give me a public URL I can
send to a designer.
```

### goose Output

:::note Desktop

Done — the box is up and the URL is live.

**1. Box** — `sandbox_create` with the `dev-box` template (size `m`: 2 vCPU,
2 GB RAM), booted in about 12 seconds. Nothing was installed on your machine.

**2. Code** — `sandbox_git_clone` pulled the repo into `/workspace`.

**3. Tests** — `sandbox_exec` ran `npm ci && npm test`. 148 passed, 0 failed,
in 41s. Two warnings about a deprecated peer dependency, nothing blocking.

**4. Serve** — the dev server is running under `sandbox_exec_background`, so
it survives between my turns, and `sandbox_expose_port` put it behind HTTPS:

`https://sb-7f3a91c2-5173.easybits.cloud`

Send that to your designer as-is. Two things worth knowing:

- The box auto-destroys in 5 minutes unless I extend it — say "keep it alive
  for an hour" and I'll call `sandbox_extend`.
- If you want it permanent instead of ephemeral, `launch_app` redoes this as
  an always-on machine with its own domain and a recovery release, in one
  call.

Want me to snapshot it first so you can fork this exact state later?

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

### goose runs on EasyBits, too

The relationship goes both ways. `goose_spawn` — one call, no configuration —
boots goose itself inside a Firecracker microVM as an ACP server (JSON-RPC over
SSE) with managed credentials, and hands you back `{ sandboxId, agentUrl,
healthUrl, agentId, embedToken }`. `goose` is a first-class sandbox template
alongside `python`, `node` and `ubuntu`, and the platform speaks ACP natively
rather than shelling out to a binary.

That makes goose something you can *deploy*, not only something you run: a
goose per customer, a goose behind a web widget, a goose that keeps working
after you close the laptop.

### Beyond the box

The same key reaches the rest of the platform once you widen the toolset:

- **`--tools hosting`** — `launch_app` takes a repo or an archive to a running,
  always-on machine with a public HTTPS URL, a custom domain and a recovery
  release, in one call.
- **`--tools core`** — files on a CDN, SQL databases, documents that export to
  PDF and publish to a URL, forms, brand kits.
- **`--tools web`** — search, unblocked fetching, structured extraction and
  crawling, as described above.
