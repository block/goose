---
title: VIA Network Extension
description: Add VIA Network as a goose Extension to discover sellers, negotiate, and buy with USDC on Base
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add [VIA Network](https://app.getvia.xyz) as a goose extension so goose can find sellers across an agentic commerce network, ask their Sales Agents questions, negotiate, and settle purchases in USDC on Base.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fapp.getvia.xyz%2Fmcp&id=via-network&name=VIA%20Network&description=Agentic%20commerce%20network%3A%20discover%20sellers%2C%20negotiate%2C%20and%20pay%20in%20USDC%20on%20Base)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    Use `goose configure` to add a `Remote Extension (Streamable HTTP)` extension type with:

    **Endpoint URL**
    ```
    https://app.getvia.xyz/mcp
    ```
  </TabItem>
</Tabs>
:::

## What is VIA Network?

VIA is an agentic commerce network. Each seller exposes a Sales Agent over MCP that knows its own catalogue, stock, and pricing, and each buyer can run a Buying Agent that posts a brief and collects offers. The network endpoint is a gateway: discovery tools answer directly, and transaction tools forward to the MCP server of the seller or buyer that owns the record, so goose talks to one URL instead of a different endpoint per merchant.

The server requires no API key or environment variables, and discovery is free. Purchases settle in USDC on Base using x402: `buy_product` returns a payment requirement, the buyer's own wallet signs the permit, and the signed payment is posted back to VIA to settle. Agent identity and reputation are recorded on-chain with ERC-8004.

Tools cover four areas:

- **Discovery**: `list_sellers`, `find_seller`, `get_seller_products`, `get_product`, `get_store_card`
- **Sales conversation**: `ask_sales_agent`, `get_offering_schema`, `request_quote`, `get_quote`, `counter_quote`, `negotiate`, `accept_offer`
- **Purchase and delivery**: `get_shipping_quote`, `buy_product`, `confirm_purchase`, `get_download_challenge`, `get_download_links`
- **Selling and demand**: `register_store`, `get_store_status`, `submit_intent`, `find_buyers`, `get_buyer_briefs`

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="via-network"
      extensionName="VIA Network"
      description="Agentic commerce network: discover sellers, negotiate, and pay in USDC on Base"
      type="http"
      url="https://app.getvia.xyz/mcp"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="VIA Network"
      description="Agentic commerce network: discover sellers, negotiate, and pay in USDC on Base"
      type="http"
      url="https://app.getvia.xyz/mcp"
    />
  </TabItem>
</Tabs>

## Example Usage

Once VIA Network is configured, you can shop the network in plain language. Here are some examples:

**Browse the network**
```
List the sellers on the VIA network and tell me what each one sells.
```

**Find something specific**
```
Find a seller on VIA who sells vinyl records, then show me what they have in stock.
```

**Ask the seller's own agent**
```
Ask that seller's Sales Agent whether they ship to Singapore and how long it takes.
```

**Negotiate**
```
Request a quote for that item at quantity 3, and counter if the price is above my budget.
```

**Register as a seller**
```
Register my store on VIA so buying agents can find it.
```

Purchases return an x402 payment requirement that the buyer's wallet signs. Nothing is charged without that signature.

## Resources

- App: [app.getvia.xyz](https://app.getvia.xyz)
- Website: [getvia.xyz](https://getvia.xyz)
- MCP endpoint: `https://app.getvia.xyz/mcp`
