---
title: Brave Search Extension
description: Add Brave Search API as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';


This tutorial will get you started with the [Brave Search MCP Server](https://github.com/modelcontextprotocol/servers/tree/main/src/brave-search) as a Goose extension to enable interactive searches for both web and local searches.


:::tip TLDR

**Command**
```sh
npx @modelcontextprotocol/server-brave-search
```

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
  <TabItem value="cli" label="Goose CLI" default>
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◆  What type of extension would you like to add?
    │  ○ Built-in Extension 
    // highlight-start    
    │  ● Command-line Extension (Run a local command or script)
    // highlight-end    
    │  ○ Remote Extension 
    └ 
  ```

  3. Give your extension a name
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
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
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  brave-search
    │
    // highlight-start
    ◆  What command should be run?
    │  npx @modelcontextprotocol/server-brave-search
    // highlight-end
    └ 
  ```  

  5. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  brave-search
    │
    ◇  What command should be run?
    │  npx @modelcontextprotocol/server-brave-search
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
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  brave-search
    │
    ◇  What command should be run?
    │  npx @modelcontextprotocol/server-brave-search
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
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  brave-search
    │
    ◇  What command should be run?
    │  npx @modelcontextprotocol/server-brave-search
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
  <TabItem value="ui" label="Goose Desktop">
  1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40modelcontextprotocol%2Fseerver-brave-search&id=brave-search&name=Brave%20Search&description=Brave%20Search%20API&env=BRAVE_API_KEY%3DYour%20API%20Key)
  2. Press `Yes` to confirm the installation
  3. Get your [Brave Search API Key](https://api-dashboard.search.brave.com/app/keys) and paste it in
  4. Click `Save Configuration`
  5. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
</Tabs>

## Example Usage

The Brave Search MCP server gives you access to the Brave Search API, providing both web and local search capabilities:

- **Web Search**: General queries, news, articles, with pagination and freshness controls
- **Local Search**: Find businesses, restaurants, and services with detailed information (requires a Pro API key)
- **Flexible Filtering**: Control result types, safety levels, and content freshness
- **Smart Fallbacks**: Local search automatically falls back to web when no results are found


#### Goose Prompt

```
( O)> I'd like to learn to cook Italian food. Can you search for the top 3 YouTube channels to watch? Also, can you find me the top 5 Italian food recipes for beginners?
```

#### Goose Output

```
I'll help you search for YouTube cooking channels and beginner-friendly Italian recipes. Let me break this down into two searches.

First, let's search for top YouTube channels for Italian cooking.

─── brave_web_search | brave-search ──────────────────────────
count: 3
query: top 3 Italian cooking YouTube channels


─── brave_web_search | brave-search ──────────────────────────
count: 5
query: top 5 easy Italian recipes for beginners


Okay, I have some information for you.

**Top 3 Italian Cooking YouTube Channels:**

Based on the search results, here are a few channels to consider:

1.  **Pasta Grannies:** Feature authentic Italian grandmothers sharing traditional recipes and techniques. Perfect for learning authentic, traditional methods.
2.  **Italia Squisita:** A professional channel that shows multiple versions of classic Italian dishes, great for understanding different approaches to traditional recipes.
3.  **Vincenzo's Plate:** Run by Vincenzo and his Italian family, focusing on authentic Italian recipes with detailed instructions perfect for beginners.

**Top 5 Easy Italian Recipes for Beginners:**

1.  **Classic Spaghetti Aglio e Olio (Carlig and Oil Pasta):**
  - One of the simplest yet most delicious Italian pasta dishes
  - Requires minimal ingredients: pasta, garlic, olive oil, and red pepper flakes

2.  **Basic Marinara Sauce:**
  - Foundation of many Italian dishes
  - Made with tomatoes, garlic, herbs, and olive oil

3.  **Bruschetta:**
  - Simple appetize with toasted bread, tomatoes, garlic, and basil
  - Great way to learn about balancing Italian flavors

4.  **Chicken Piccata:**
  - Classic Italian-American dish
  - Made with chicken cutlets, lemon, capers, and white wine sauce

5.  **Panzanella:**
  - Traditional Tuscan bread salad
  - Perfect for learning to work with fresh ingredients and understanding Italian flavor combinations

**Tips for Beginners:**

- Start with simple recipes that have few ingredients
- Focus on quality ingredients, especially fresh good olive oil
- Learn basic Italian cooking techniques like al dente pasta cooking
- Don't be afraid to make mistakes - Italian cooking is about feeling and adjusting to taste
- Watch videos multiple times before attempting recipes to understand the techniques

```
