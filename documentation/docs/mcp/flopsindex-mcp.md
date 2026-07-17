---
title: FLOPS Index Extension
description: Add FLOPS Index MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [FLOPS Index MCP Server](https://github.com/zeroatflops/flopsindex) as a goose extension, giving goose access to a public, key-free index of GPU and compute rental reference prices. Every value it returns can be independently checked against the published index, so any price goose quotes can be verified rather than guessed.

FLOPS tracks H100, A100, H200 and other accelerators across spot, on-demand, and DePIN (decentralized) markets. No account or API key is required.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=flopsindex-mcp&id=flopsindex&name=FLOPS%20Index&description=Key-free%20GPU%20compute%20rental%20reference%20prices)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  uvx flopsindex-mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="flopsindex"
      extensionName="FLOPS Index"
      description="Key-free GPU compute rental reference prices"
      type="stdio"
      command="uvx"
      args={["flopsindex-mcp"]}
      timeout={300}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="FLOPS Index"
      description="Key-free GPU compute rental reference prices"
      type="stdio"
      command="uvx flopsindex-mcp"
      timeout={300}
    />
  </TabItem>
</Tabs>

No API key or environment variables are required.

:::info Hosted server
If you prefer not to run the server locally, a hosted, key-free endpoint is also available at `https://app.flopsindex.com/mcp` (streamable HTTP). Add it as a remote MCP extension pointing at that URL instead of running `uvx`.
:::

The extension exposes five tools:

| Tool | Purpose |
| --- | --- |
| `list_indices` | List the available price indices, with each index's family, cadence, and unit. |
| `search_indices` | Find indices matching a chip, market, or keyword. |
| `get_price` | Get the current published reference price for a given index. |
| `get_index` | Retrieve the full detail for a specific index. |
| `verify` | Check a number you are about to cite against the published FLOPS value before you use it. |

## Example Usage

The FLOPS Index extension is useful when you need a defensible number for what a GPU actually costs to rent, rather than a guess. Because every price can be checked against the published index, goose can confirm the figure it used instead of hallucinating one.

### goose Prompt

```
I'm sizing a training run. What's the current reference price to rent an H100,
and how does spot compare to on-demand? Verify the numbers against the index
before you give them to me.
```

### goose Output

:::note CLI

I'll find the H100 indices and read the current published prices.

─── search_indices | flopsindex ──────────────────

query: H100

─── get_price | flopsindex ──────────────────

index_id: FLOPS-H100-SPOT

─── get_price | flopsindex ──────────────────

index_id: FLOPS-H100-OD

─── verify | flopsindex ──────────────────

index_id: FLOPS-H100-SPOT

Here's what the FLOPS Index currently publishes for the H100, per GPU-hour:

**H100 — spot** — the current spot reference value from the index, confirmed with a
`verify` call so the figure matches what FLOPS publishes.

**H100 — on-demand** — the on-demand reference value, typically higher than spot in
exchange for guaranteed, non-interruptible capacity.

Spot is the cheaper option when your workload tolerates interruption; on-demand trades
a higher rate for stable capacity. Both figures were checked against the published
index rather than estimated. Want me to pull the DePIN market value as well for
comparison?

:::
