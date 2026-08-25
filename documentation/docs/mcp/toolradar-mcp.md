---
title: Toolradar Extension
description: Add Toolradar MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Toolradar MCP Server](https://github.com/Nadeus/toolradar-mcp) as a goose extension. Toolradar gives goose live access to a curated database of 8,600+ software tools: semantic search, budget-aware recommendations, side-by-side comparisons, real alternatives, and pricing verified from vendors' own pricing pages.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Ftoolradar.com%2Fapi%2Fmcp&id=toolradar&name=Toolradar&description=Search%2C%20compare%2C%20and%20get%20verified%20pricing%20for%208%2C600%2B%20software%20tools)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streamable HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://toolradar.com/api/mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration

The remote server works anonymously on a rate-limited free tier, so no API key is needed to get started.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="toolradar"
      extensionName="Toolradar"
      description="Search, compare, and get verified pricing for 8,600+ software tools"
      type="http"
      url="https://toolradar.com/api/mcp"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="toolradar"
      type="http"
      url="https://toolradar.com/api/mcp"
      timeout={300}
    />
  </TabItem>
</Tabs>

For higher rate limits, run the stdio package instead with a free API key from [toolradar.com/dashboard/api-keys](https://toolradar.com/dashboard/api-keys):

```sh
TOOLRADAR_API_KEY=your_key npx -y toolradar-mcp
```

## Example Usage

Toolradar exposes 8 tools: `search_tools`, `recommend_tools`, `get_tool`, `compare_tools`, `get_alternatives`, `get_pricing`, `list_categories`, and `report_issue`. All of them are read-only except `report_issue`, which files a data-correction ticket.

### goose Prompt

```
Compare Notion, Asana and ClickUp for a 10-person team, then show me ClickUp's current pricing.
```

goose calls `compare_tools` for a side-by-side comparison with a computed top pick, then `get_pricing` for the pricing tiers along with the date Toolradar last verified them against the vendor's pricing page.
