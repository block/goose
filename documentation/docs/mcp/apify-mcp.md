---
title: Apify Extension
description: Add Apify MCP server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import { PanelLeft } from 'lucide-react';

This tutorial covers how to add the [Apify MCP server](https://mcp.apify.com) as a goose extension, enabling goose to call thousands of tools from [Apify Store](https://apify.com/store) to extract data from social media, e-commerce sites, search engines, online maps, or any other website.

## Configuration

<Tabs groupId="remote-or-local">
<!-- REMOTE SETUP -->
<TabItem value="remote" label="Apify Remote MCP" default>

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  Use `Add custom extension` in Settings → Extensions to add a `Streamable HTTP` extension type with:
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Use `goose configure` to add a `Remote Extension (Streaming HTTP)` extension type with:
  </TabItem>
</Tabs>

  **Endpoint URL**
```
https://mcp.apify.com
```
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. Obtain an [Apify Token](https://console.apify.com/settings/integrations)  
    2. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar  
    3. Click `Extensions`  
    4. Click `Add custom extension`  
    5. On the `Add custom extension` modal, enter the following:  
       - **Extension Name**: Apify  
       - **Type**: Streamable HTTP  
       - **Endpoint**: `https://mcp.apify.com`  
       - **Request Headers**:  
          - **Header name**: `Authorization`  
          - **Value**: `Bearer <YOUR_APIFY_TOKEN>`  
    6. Click `+ Add` to save the header  
    7. Click `Add Extension` to save the extension
    9. Navigate to the chat  
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="apify-mcp"
      type="http"
      url="https://mcp.apify.com"
      timeout={300}
      envVars={[
        { key: "Authorization", value: "Bearer apify_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" }
      ]}
    />
    :::info
    Obtain your <a href="https://console.apify.com/settings/integrations" target="_blank" rel="noopener noreferrer">Apify Token</a> and paste it in.
    :::
  </TabItem>
</Tabs>

</TabItem>

<!-- LOCAL SETUP -->
<TabItem value="local" label="Apify Local MCP">

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=@apify/actors-mcp-server&arg=start&id=mcp_apify_local&name=Apify%20Local%20MCP%20Server&description=Run%20Apify%20MCP%20server%20locally%20using%20your%20token&env=APIFY_TOKEN%3DYour%20Apify%20Token)
  </TabItem>

  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @apify/actors-mcp-server
  ```
  </TabItem>
</Tabs>

**Environment Variables**
```
APIFY_TOKEN: <YOUR_APIFY_TOKEN>
```
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. Obtain an [Apify Token](https://console.apify.com/settings/integrations)  
    2. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar  
    3. Click `Extensions`  
    4. Click `Add custom extension`  
    5. On the `Add custom extension` modal, enter the following:  
       - **Extension Name**: Apify  
       - **Type**: STDIO
       - **Command**: `npx -y @apify/actors-mcp-server`  
       - **Environment Variables**:  
          - **Variable name**: `APIFY_TOKEN`  
          - **Value**: `<YOUR_APIFY_TOKEN>`  
    6. Click `+ Add` to save the header  
    7. Click `Add Extension` to save the extension  
    9. Navigate to the chat  
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="apify-mcp"
      type="stdio"
      command="npx -y @apify/actors-mcp-server"
      timeout={300}
      envVars={[
        { key: "APIFY_TOKEN", value: "apify_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" }
      ]}
    />
    :::info
    Obtain your <a href="https://console.apify.com/settings/integrations" target="_blank" rel="noopener noreferrer">Apify Token</a> and paste it in.
    :::  
  </TabItem>
</Tabs>

</TabItem>
</Tabs>

## Example Usage

The Apify MCP server lets you connect goose to [Apify Store](https://apify.com/store), a marketplace of thousands of ready-made tools (called _Actors_) for all kinds of web scraping, crawling, and data extraction use cases. Learn more about Apify Actors in the [official documentation](https://docs.apify.com/platform/actors)

In this example, goose will use the [Google Places Scraper](https://apify.com/compass/crawler-google-places) Actor to identify the most credible, top-rated Italian restaurant in Prague based on public ratings, reviews, and fine-dining recognition.

### Goose Prompt

```

( O)> Find the top-rated Italian restaurant in Prague with verifiable credibility — use Apify MCP to find Google Maps review scraper Actor, focus on places with at least 200+ reviews, an average rating of 4.7 or higher on Google, TripAdvisor, or OpenTable, and any Michelin or fine-dining recognition. Include the restaurant’s name, rating, total review count, address, cuisine style, and a short summary of what makes it stand out (e.g., service, ambiance, or signature dishes). Return to me only 1 restaurant with the highest rating to review count ratio

```

### Goose Output

```

Based on the gathered information, the top-rated Italian restaurant in Prague with verifiable credibility is:

### **Al Tagliere**

* **Rating:** 4.7/5
* **Total Reviews:** 457
* **Address:** [Al Tagliere on TripAdvisor](https://www.tripadvisor.com/Restaurant_Review-g274707-d6835155-Reviews-Al_Tagliere-Prague_Bohemia.html)
* **Cuisine Style:** Italian
* **Summary:** Al Tagliere is celebrated for its authentic Italian dishes that showcase regional flavors. The restaurant is known for its delightful ambiance, attentive service, and signature dishes such as homemade pasta and traditional desserts. Patrons frequently highlight the freshness of ingredients and the welcoming atmosphere, making it a preferred choice for both locals and tourists.

This restaurant has a strong reputation and meets your criteria for quality and credibility.

```
