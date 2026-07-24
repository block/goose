---
title: Webhound Extension
description: Add Webhound MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

Webhound runs web investigations whose effort is controlled by a dollar budget. The prompt defines the question; the budget defines how long and how broadly Hound can search, read, follow leads, and verify claims. Hound is Webhound's research harness, built with DeepSeek V4 Pro and GPT-5.4; it is not a model selector. A completed run can return a cited report or structured dataset, along with working documents, claim traces, sources, and an evidence pack.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fapi.webhound.ai%2Fapi%2Fv2%2Fmcp&id=webhound&name=Webhound&description=Run%20budgeted%2C%20inspectable%20web%20investigations%20that%20return%20cited%20reports%2C%20datasets%2C%20and%20evidence%20packs)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Use `goose configure` to add a `Remote Extension (Streamable HTTP)` extension with this endpoint:

  ```text
  https://api.webhound.ai/api/v2/mcp
  ```
  </TabItem>
</Tabs>
:::

:::info OAuth
Goose dynamically registers a public client with Webhound and opens the authorization page in your browser. Paste your own Webhound API key there. Webhound exchanges it for an account-specific token. The catalog entry contains no credential, and connecting it does not give other users access to your account.
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="webhound"
      extensionName="Webhound"
      description="Run budgeted, inspectable web investigations that return cited reports, datasets, and evidence packs"
      type="http"
      url="https://api.webhound.ai/api/v2/mcp"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Webhound"
      description="Run budgeted, inspectable web investigations that return cited reports, datasets, and evidence packs"
      type="http"
      url="https://api.webhound.ai/api/v2/mcp"
      timeout={300}
      infoNote="Goose opens Webhound's OAuth flow. Authorize with your own Webhound account."
    />
  </TabItem>
</Tabs>

## Example Usage

This checks the connection and account defaults without starting a paid research run.

### goose Prompt

```text
Check my Webhound account and explain the default research budget. Do not start research.
```

### goose Output

```text
Webhound is connected. Your default product is a report and your default budget is $5.
A dollar budget controls how much effort Webhound spends on the investigation; $1 buys
about 15 minutes of research. No research session was started.
```

For research runs, keep watching until Webhound reports `done=true`. Partial working notes or `output_ready=true` alone do not mean the investigation is finished.
