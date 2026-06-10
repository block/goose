---
title: TempGuru Event Staffing Extension
description: Add TempGuru Event Staffing MCP Server as a goose Extension for W-2 event staffing rates, availability, and compliance across 300+ US/CA markets
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [TempGuru Event Staffing MCP Server](https://github.com/Tempguru-co/tempguru-mcp) as a goose extension to look up W-2 event staffing data across 300+ US and Canadian markets: city coverage, role rate ranges, availability and lead-time guidance, and state-by-state labor compliance rules. Useful when planning conventions, trade shows, festivals, concerts, sporting events, corporate events, or brand activations. Rate ranges are planning estimates, not quotes; an opt-in `request_quote` tool submits a staffing inquiry for human follow-up.

The hosted server requires no API key or account. The five lookup tools are read-only; `request_quote` is the only tool that writes anything (a quote request you compose).

## Configuration

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.tempguru.co%2Fmcp&id=tempguru&name=TempGuru%20Event%20Staffing&description=W-2%20event%20staffing%20data%3A%20coverage%2C%20rates%2C%20lead%20times%2C%20and%20compliance%20for%20300%2B%20US%2FCA%20markets)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streaming HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://mcp.tempguru.co/mcp
  ```
  </TabItem>
</Tabs>

No API key, request header, or environment variable is required.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="tempguru"
      extensionName="TempGuru Event Staffing"
      description="W-2 event staffing data: coverage, rates, lead times, and compliance for 300+ US/CA markets"
      type="http"
      url="https://mcp.tempguru.co/mcp"
    />
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="TempGuru Event Staffing"
      description="W-2 event staffing data: coverage, rates, lead times, and compliance for 300+ US/CA markets"
      type="http"
      url="https://mcp.tempguru.co/mcp"
      timeout={300}
    />
  </TabItem>
</Tabs>

Prefer a local process? The same server ships as an npm CLI — add a `Command-line Extension` with command `npx -y tempguru-mcp` instead.

## Example Usage

Let's use the TempGuru extension to scope staffing for a trade show booth.

### goose Prompt

```
I'm running a booth at a trade show in Boston on September 15. I'll need about
8 brand ambassadors. Does TempGuru cover Boston, what would that roughly run
per hour, and how much lead time do I need?
```

### goose Output

```
I'll check coverage, rates, and lead time for Boston.

[get_cities: state=MA]
✓ 5 Massachusetts markets covered — Boston is a hub-tier market

[check_availability: city=Boston, date=2026-09-15, role=brand-ambassadors, count=8]
✓ Boston, Massachusetts (hub tier)
  Brand Ambassadors: $56–65/hr all-inclusive (workers comp, liability,
  payroll taxes included)
  Event is 97 days out; typical lead time for hub cities is 48 hours —
  comfortably bookable

Boston is covered. Eight brand ambassadors would run roughly $56–65 per hour
per person, all-inclusive W-2 (that's a planning estimate, not a quote). At
97 days out you're far ahead of the typical 48-hour hub-city lead time. Want
me to put together a quote request?
```

## Tools

| Tool | What it does |
|---|---|
| `get_cities` | List covered cities, filterable by state or tier |
| `get_roles` | List staffing roles with descriptions and skill tiers |
| `check_availability` | Lead-time guidance for a city + date (not a reservation) |
| `get_role_pricing` | All-inclusive W-2 hourly rate ranges by role and city |
| `get_compliance_by_state` | Minimum wage, overtime, and classification rules by state |
| `request_quote` | Submit a staffing inquiry for a human-prepared quote (opt-in write) |

## Links

- **Website**: [tempguru.co/ai](https://tempguru.co/ai)
- **GitHub**: [Tempguru-co/tempguru-mcp](https://github.com/Tempguru-co/tempguru-mcp)
- **npm**: [`tempguru-mcp`](https://www.npmjs.com/package/tempguru-mcp)
- **Official MCP Registry**: `co.tempguru/event-staffing`
- **Smithery**: [smithery.ai/server/tempguru/event-staffing](https://smithery.ai/server/tempguru/event-staffing)
