---
title: PairBook Extension
description: Add PairBook MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [PairBook MCP Server](https://github.com/vj88-coder/pairbook-mcp) as a goose extension to answer correlation and diversification questions about 4,700+ US stocks and ETFs, including issuer-sourced ETF holdings overlap, beta, volatility, drawdowns and fund facts. The data comes from the free PairBook API, refreshes every trading day, and needs no API key or account.

:::tip Quick Install

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=pairbook-mcp&id=pairbook-mcp&name=PairBook&description=Correlation%20and%20ETF%20overlap%20for%204%2C700%2B%20US%20stocks%20and%20ETFs)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y pairbook-mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="pairbook-mcp"
    extensionName="PairBook"
    description="Correlation and ETF overlap for 4,700+ US stocks and ETFs"
    command="npx"
    args={["-y", "pairbook-mcp"]}
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="pairbook"
      description="Correlation and ETF overlap for 4,700+ US stocks and ETFs"
      command="npx -y pairbook-mcp"
    />
  </TabItem>
</Tabs>

No environment variables are needed: the server is read-only and talks to the free public PairBook API without any key or account.

## Example Usage

The extension exposes five read-only tools: compare a pair of assets, profile one asset, find diversifiers, fetch weekly return series, and resolve a company name to its ticker. Any of the 11.3 million possible pairs can be compared; popular pairs come precomputed with holdings overlap and the rest are computed on demand.

### goose Prompt

```
I hold QQQ and I am thinking about adding QQQM and SCHD. Are QQQ and QQQM redundant, and does SCHD actually diversify the position? Compare fees and yields too.
```

goose will call the PairBook tools to fetch the correlations, the holdings overlap between the funds, and their expense ratios and dividend yields, then answer with the actual figures, for example that QQQ and QQQM hold essentially the same portfolio while SCHD overlaps only slightly and carries a higher yield.

The data covers US-listed stocks and ETFs only, uses weekly closes, and is not investment advice.
