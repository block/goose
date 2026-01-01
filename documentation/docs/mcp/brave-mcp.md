---
title: Brave Search Extension
description: Add Brave Search API as a goose Extension
unlisted: true
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/kD2YA61NTLU" />

Server moved

This tutorial will get you started with the [Brave Search MCP Server](https://www.pulsemcp.com/servers/brave-search) as a goose extension to enable interactive searches for both web and local searches.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40modelcontextprotocol%2Fserver-brave-search&id=brave-search&name=Brave%20Search&description=Brave%20Search%20API&env=BRAVE_API_KEY%3DYour%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @modelcontextprotocol/server-brave-search
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  BRAVE_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="brave-search"
    extensionName="Brave Search"
    description="Brave Search API"
    command="npx"
    args={["-y", "@modelcontextprotocol/server-brave-search"]}
    envVars={[{ name: "BRAVE_API_KEY", label: "Your Brave Search API Key" }]}
    apiKeyLink="https://api-dashboard.search.brave.com/app/keys"
    apiKeyLinkText="Brave Search API Key"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◆  What type of extension would you like to add?
    │  ○ Built-in Extension 
    // highlight-start    
    │  ● Command-line Extension (Run a local command or script)
    // highlight-end    
    │  ○ Remote Extension (SSE) 
    │  ○ Remote Extension (Streaming HTTP) 
    └ 
  ```

  3. Give your extension a name
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    // highlight-start
    ◆  What would you like to call this extension?
    │  brave-search
    // highlight-end
    └ 
  ```

  4. Enter the command
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  brave-search
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y @modelcontextprotocol/server-brave-search
    // highlight-end
    └ 
  ```  

  5. Enter the number of seconds goose should wait for actions to complete before timing out. Default is 300s
   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  brave-search
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-brave-search
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    │
    └ 
  ```  

  6. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  brave-search
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-brave-search
    │
    ◆  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◇  Would you like to add a description?
    │  No
    // highlight-end
    │
    └ 
  ```  

  7. Obtain a [Brave Search API Key](https://api-dashboard.search.brave.com/app/keys) and paste it in.
  :::info
  Sign up for a [Brave Search API account](https://brave.com/search/api/), choose a plan (the free tier gives you 2000 queries per month), and generate your API key from [the developer dashboard](https://api-dashboard.search.brave.com/app/keys).
  :::

   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  brave-search
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-brave-search
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    ◇  Would you like to add a description?
    │  No
    │
    // highlight-start
    ◆  Would you like to add environment variables?
    │  Yes 
    │
    ◇  Environment variable name:
    │  BRAVE_API_KEY
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  No 
    // highlight-end
    └  Added brave-search extension
  ```  

  </TabItem>
</Tabs>

## Example Usage

The Brave Search MCP server gives you access to the Brave Search API, providing both web and local search capabilities:

- **Web Search**: General queries, news, articles, with pagination and freshness controls
- **Local Search**: Find businesses, restaurants, and services with detailed information (requires a Pro API key)
- **Flexible Filtering**: Control result types, safety levels, and content freshness
- **Smart Fallbacks**: Local search automatically falls back to web when no results are found


#### goose Prompt

```
( O)> Organize a Italian dinner party with Ian, Rizel, Ace, and Ebony. Find 3 or 4 YouTube cooking channels (with URLs) that focus on Italian cuisine for beginners, and then find one recipe (with URL) for each person to make: one salad, two entrees, and one dessert.
```

#### goose Output

```

I'll help you organize the Italian dinner party and search for appropriate cooking resources and recipes. Let me break this down into parts.

First, let's search for some beginner-friendly Italian cooking YouTube channels:

Brave Web Search
query: best youtube channels italian cooking for beginners authenti...

Let me search for some specific recipes:

Brave Web Search
query: authentic italian salad recipes panzanella caprese insalata

Brave Web Search
query: authentic italian main course pasta recipes carbonara osso b...

Brave Web Search
query: authentic italian dessert recipes tiramisu panna cotta


Based on my research, I'll organize the dinner party plan with recommended YouTube channels and specific recipes for each person. Here's the plan:

Recommended Italian Cooking YouTube Channels for Beginners

1. Vincenzo's Plate (https://www.youtube.com/channel/UCcsSowAamCLJv-xeF9geXoA)
  - Authentic Italian recipes with clear instructions
  - Run by an Italian native who explains techniques well for beginners

2. Italia Squisita (https://www.youtube.com/c/ItaliaSquisita)
  - Features traditional Italian recipes
  - Often shows multiple ways to make classic dishes

3. Giallo Zafferano (https://www.youtube.com/user/GialloZafferano)
  - Italy's most popular cooking channel
  - Comprehensive collection of authentic Italian recipes


Recipe Assignments

1. Rizel - Salad Course
  - Classic Panzanella (Tuscan Bread and Tomato Salad)
  - Recipe: https://www.themediterraneandish.com/tomato-panzanella-salad-recipe/
  - Perfect summer salad with crusty bread, tomatoes, and fresh basil

2. Ian - First Entrée
  - Authentic Spaghetti Carbonara
  - Recipe: https://anitalianinmykitchen.com/spaghetti-carbonara/
  - Classic Roman pasta dish with eggs, pecorino, and guanciale

3. Ace - Second Entrée
  - Osso Buco alla Milanese
  - Recipe: https://www.pastagrammar.com/post/osso-buco-authentic-italian-veal-recipe
  - Traditional Milanese braised veal shanks with gremolata

4. Ebony - Dessert
  - Classic Vanilla Panna Cotta
  - Recipe: https://www.recipesfromitaly.com/panna-cotta-recipe/
  - Elegant, creamy dessert that can be made ahead


Tips for Success

- Watch the YouTube channels beforehand to familiarize yourself with Italian cooking techniques
- Make sure to read through the entire recipe before starting
- The Panna Cotta should be made at least 4 hours ahead (or the night before) to allow proper setting time
- The Panzanella can be assembled just before serving for the best texture
- The Osso Buco can be made slightly ahead and reheated, as it often tastes better the next day
- The Carbonara should be made just before serving and served immediately while hot

```
