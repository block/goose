---
title: Bocha Search Extension
description: Add Bocha Search MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Bocha Search MCP Server](https://github.com/BochaAI/bocha-search-mcp) as a goose extension. Bocha provides web search and AI search tools that return source-linked web results and structured cards for domains such as weather, news, encyclopedia entries, stocks, travel, and more.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  Clone the MCP server locally, then add it as a custom command-line extension in goose Desktop.
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Commands**
  ```sh
  git clone https://github.com/BochaAI/bocha-search-mcp.git
  uv --directory /path/to/bocha-search-mcp run bocha-search-mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  BOCHA_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command.
:::

First, clone the Bocha Search MCP server:

```sh
git clone https://github.com/BochaAI/bocha-search-mcp.git
```

Replace `/path/to/bocha-search-mcp` in the commands below with the absolute path to that local checkout.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  1. Click the sidebar button in the top-left, then click `Extensions`.
  2. Click `Add custom extension`.
  3. Set the type to `Standard IO`.
  4. Set the ID to `bocha-search`.
  5. Set the name to `Bocha Search`.
  6. Set the description to `AI-powered web and vertical search`.
  7. Set the command to:
     ```sh
     uv --directory /path/to/bocha-search-mcp run bocha-search-mcp
     ```
  8. Replace `/path/to/bocha-search-mcp` with the absolute path to your local checkout.
  9. Add an environment variable named `BOCHA_API_KEY` with your Bocha API key from [open.bochaai.com](https://open.bochaai.com).
  10. Click `Add Extension`.
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Bocha Search"
      description="AI-powered web and vertical search"
      type="stdio"
      command="uv --directory /path/to/bocha-search-mcp run bocha-search-mcp"
      timeout={300}
      envVars={[
        { key: "BOCHA_API_KEY", value: "................................" }
      ]}
      commandNote="Replace /path/to/bocha-search-mcp with the absolute path to your local checkout."
      infoNote={
        <>
          Obtain your <a href="https://open.bochaai.com" target="_blank" rel="noopener noreferrer">Bocha API Key</a> and paste it in.
        </>
      }
    />
  </TabItem>
</Tabs>

## Local Development

You can verify the local MCP server before adding it to goose:

```sh
uv --directory /path/to/bocha-search-mcp run bocha-search-mcp
```

You can also test the MCP server with the MCP Inspector:

```sh
npx @modelcontextprotocol/inspector uv --directory /path/to/bocha-search-mcp run bocha-search-mcp
```

## Example Usage

The Bocha Search MCP server provides two tools:

1. **Bocha Web Search**: searches the web and returns titles, URLs, snippets, site names, and publish dates.
2. **Bocha AI Search**: adds structured vertical cards for queries that benefit from richer domain data, such as weather, stocks, calendars, travel, encyclopedia, and news.

Both tools accept:

- `query`: the search query.
- `freshness`: optional time filter such as `noLimit`, `oneYear`, `oneMonth`, `oneWeek`, `oneDay`, or a date range.
- `count`: number of results to return, from 1 to 50.

### goose Prompt

```
Search for the latest news about AI search engines and summarize the most important product updates with source links.
```

### goose Output

```
I'll use Bocha Search to find recent coverage and source-linked results.

Bocha Web Search
query: latest AI search engine product updates
freshness: oneWeek
count: 5

Here are the most relevant updates I found:

1. Several AI search products are adding richer answer pages with source citations and follow-up query suggestions.
2. Search providers are expanding real-time coverage for news, finance, weather, and other high-freshness domains.
3. Enterprise search offerings are focusing on cleaner source attribution and lower-latency retrieval.

Sources:
- Example News Source: https://example.com/ai-search-update
- Example Product Blog: https://example.com/product-search-release
- Example Research Coverage: https://example.com/search-retrieval-analysis
```

:::tip
Use Bocha Web Search for general source-linked web results, and Bocha AI Search when the query may benefit from structured vertical cards such as weather, stocks, travel, or encyclopedia-style data.
:::
