---
title: Apify Extension
description: Add Apify MCP server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import { PanelLeft } from 'lucide-react';

This tutorial covers how to add the [Apify MCP server](https://mcp.apify.com) as a goose extension, enabling goose to call over 6.000+ Actors from [Apify Store](https://apify.com/store) to extract data from social media, e-commerce sites, search engines, online maps, or any other website.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  Use `Add custom extension` in Settings → Extensions to add a `Streamable HTTP` extension type with:
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  Use `goose configure` to add a `Remote Extension (Streaming HTTP)` extension type with:
  </TabItem>
</Tabs>

  **Endpoint URL**
  ```
  https://mcp.apify.com
  ```
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    1. Obtain a [Apify Token](https://console.apify.com/settings/integrations)
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
    8. You will be taken to the OAuth screen
    9. Navigate to the chat
  </TabItem>
  <TabItem value="cli" label="Goose CLI" default>
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

## Example Usage

The Apify MCP server lets you connect goose to [Apify Store](https://apify.com/store), a marketplace of over 6.000+ ready-made tools (called _Actors_) for all kinds of web scraping, crawling, and data extraction use cases. Learn more about Apify Actors in the [official documentation](https://docs.apify.com/platform/actors)

In this example, goose will use the [Google Places Scraper](https://apify.com/compass/crawler-google-places) Actor to identify the most credible, top-rated Italian restaurant in Prague based on public ratings, reviews, and fine-dining recognition.

### Goose Prompt

```
( O)> Find the top-rated Italian restaurant in Prague with verifiable credibility — use Apify MCP to find Google Maps review scraper Actor, focus on places with at least 200+ reviews, an average rating of 4.7 or higher on Google, TripAdvisor, or OpenTable, and any Michelin or fine-dining recognition. Include the restaurant’s name, rating, total review count, address, cuisine style, and a short summary of what makes it stand out (e.g., service, ambiance, or signature dishes). Return to me only 1 restaurant with the highest rating to review count ratio
```

### Goose Output

```
Based on the gathered information, the top-rated Italian restaurant in Prague with verifiable credibility is:

### **Al Tagliere**
- **Rating:** 4.7/5
- **Total Reviews:** 457
- **Address:** [Al Tagliere on TripAdvisor](https://www.tripadvisor.com/Restaurant_Review-g274707-d6835155-Reviews-Al_Tagliere-Prague_Bohemia.html)
- **Cuisine Style:** Italian
- **Summary:** Al Tagliere is celebrated for its authentic Italian dishes that showcase regional flavors. The restaurant is known for its delightful ambiance, attentive service, and signature dishes such as homemade pasta and traditional desserts. Patrons frequently highlight the freshness of ingredients and the welcoming atmosphere, making it a preferred choice for both locals and tourists.

This restaurant has a strong reputation and meets your criteria for quality and credibility.
```
