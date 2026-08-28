---
title: Vestige Extension
description: Add Vestige MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
This tutorial covers how to add the [Vestige MCP Server](https://github.com/samvallad33/vestige) as a goose extension for local-first memory. Vestige answers **"what caused this?"** via backward-only causal Backfill (shared entities as the join key; similarity is excluded from ranking).

goose already includes a built-in [Memory](/docs/mcp/memory-mcp) extension for storing preferences and project facts. Vestige is an optional local-first alternative focused on causal Backfill. You can use either, or both; this page does not replace the built-in Memory extension.

Proof: no LLM in the memory path, exact FSRS-6 decay, local-first. Proven on local and synthetic traces — not a production claim.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=vestige-mcp-server&id=vestige&name=Vestige&description=Local-first%20Rust%20MCP%20memory%20with%20causal%20Backfill%20(recall%2C%20smart_ingest%2C%20backfill)&timeout=300)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y vestige-mcp-server
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
You will need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`. No API key is required. Data stays on your machine.

For a per-project store, pass `--data-dir` with an absolute directory. There is **no** `--project` flag (unknown args exit 1).
On Ubuntu 22.04 and Debian 12, wait for v2.4.0.

GUI clients should use npx or an absolute path to the vestige-mcp binary.
Resolve it with `which vestige-mcp` (macOS/Linux) or `where vestige-mcp` (Windows).
:::


<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="vestige"
      extensionName="Vestige"
      description="Local-first Rust MCP memory with causal Backfill (recall, smart_ingest, backfill)."
      type="stdio"
      command="npx"
      args={["-y", "vestige-mcp-server"]}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Vestige"
      description="Local-first Rust MCP memory with causal Backfill (recall, smart_ingest, backfill)."
      type="stdio"
      command="npx -y vestige-mcp-server"
      timeout={300}
    />
  </TabItem>
</Tabs>

## Advertised tools

These are the tools this listing tells goose to use:

| Tool | Arguments | Role |
|------|-----------|------|
| `recall` | `query` (mode `lookup` default) | Retrieve |
| `smart_ingest` | `content` | Store |
| `backfill` | `failure_id`, `manual`, `lookback_days`, `promote`, `scan_limit` | Backward causal trail from a later failure or symptom |

Omit `failure_id` to use the most recent failure-like memory. `manual=true` forces a run. `promote=false` is a dry run.

## Example Usage

Store a decision, then ask what caused a later failure.

### goose Prompt

```
Remember that we pin the payments SDK to 4.2 because 4.3 broke webhook signatures. Then, given a later failure about webhook signatures failing in staging, run backfill and tell me what caused this.
```

### goose Output

:::note Desktop

I will store that SDK pin with `smart_ingest`, then run `backfill` from the later webhook-signature failure.

- `smart_ingest`: stored the 4.2 pin and the 4.3 webhook-signature break.
- `backfill`: ranked earlier operational records that share entities with this failure. Similarity is not used for ranking.

The trail points at the SDK pin as a prior cause of the signature failures.

:::

## Data location

Default store (override with `--data-dir`):

- **macOS**: `~/Library/Application Support/com.vestige.core/`
- **Linux**: `~/.local/share/vestige/core/`
- **Windows**: `%APPDATA%/vestige/core/`


## Resources

- Source: https://github.com/samvallad33/vestige
- Package: vestige-mcp-server@2.3.0
