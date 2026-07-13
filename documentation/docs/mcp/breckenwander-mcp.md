---
title: BreckenWander Travel Extension
description: Add BreckenWander Travel MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [BreckenWander Travel MCP Server](https://github.com/durwardjohnson/breckenwander-travel-connector) as a goose extension to search flights, hotels, and experiences at all-in prices and get a link to complete the booking on breckenwander.com.

BreckenWander Travel is a hosted, **keyless** remote server — no API key or sign-in is required to connect.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.breckenwander.com%2Fmcp&id=breckenwander&name=BreckenWander%20Travel&description=Search%20flights%2C%20hotels%2C%20and%20experiences%20at%20all-in%20prices%20and%20book%20on%20breckenwander.com)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    Use `goose configure` to add a `Remote Extension (Streaming HTTP)` extension type with:

    **Endpoint URL**
    ```
    https://mcp.breckenwander.com/mcp
    ```
  </TabItem>
</Tabs>
:::

## What is BreckenWander?

BreckenWander is an independent travel platform built around all-in pricing: the first price you see includes every mandatory tax and fee. This extension is read-only — it searches and quotes, and every result links back to breckenwander.com to complete the booking. It never books or takes payment.

The extension exposes four tools: `search_flights`, `search_hotels`, `search_experiences`, and `build_trip` (which combines chosen results into one trip link).

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="breckenwander"
      extensionName="BreckenWander Travel"
      description="Search flights, hotels, and experiences at all-in prices and book on breckenwander.com"
      type="http"
      url="https://mcp.breckenwander.com/mcp"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="breckenwander"
      description="Search flights, hotels, and experiences at all-in prices and book on breckenwander.com"
      type="http"
      url="https://mcp.breckenwander.com/mcp"
      timeout={300}
    />
  </TabItem>
</Tabs>

## Example Usage

Once the extension is configured, ask goose for travel options. It calls the BreckenWander tools and returns all-in prices, each with a link to complete the booking on breckenwander.com.

### goose Prompt

```
Find a 4-star hotel in Cancún for August 14–17 for 2 adults, and show the all-in price.
```

### goose Output

:::note Desktop

Here are BreckenWander hotel options for Cancún, Aug 14–17 (2 adults), with all-in totals that include every mandatory tax and fee:

- **Example Beach Resort** — $980 all-in for 3 nights — [Book on breckenwander.com](https://breckenwander.com)
- **Example Lagoon Hotel** — $1,120 all-in for 3 nights — [Book on breckenwander.com](https://breckenwander.com)

Each link opens that exact property on breckenwander.com so you can complete the booking. Prices are in USD and non-binding until checkout.

:::
