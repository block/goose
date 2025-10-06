---
title: Apify Extension
description: Add Apify MCP server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import { PanelLeft } from 'lucide-react';

This tutorial covers how to add the [Apify MCP server](https://mcp.apify.com) as a Goose extension, enabling Goose to call thousands of tools from [Apify Store](https://apify.com/store) to extract data from social media, e-commerce sites, search engines, online maps, or any other website.

:::tip Quick setup
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
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `Extensions`
    3. Click `Add custom extension`
    4. On the `Add custom extension` modal, enter the following:
       - **Extension Name**: Apify
       - **Type**: Streamable HTTP
       - **Endpoint**: `https://mcp.apify.com` 
    5. Click `+ Add` to save the header
    6. Click `Add Extension` to save the extension
    7. You will be taken to the OAuth screen
    8. Navigate to the chat
  </TabItem>
  <TabItem value="cli" label="Goose CLI" default>
      <CLIExtensionInstructions
        name="apify-mcp"
        type="http"
        url="https://mcp.apify.com"
        timeout={300}
      />
  </TabItem>
</Tabs>

## Example Usage

The Apify MCP server lets you connect Goose to [Apify Store](https://apify.com/store), a marketplace of thousands of ready-made tools (called _Actors_) for all kinds of web scraping, crawling, and data extraction use cases. Learn more about Apify Actors in the [official documentation](https://docs.apify.com/platform/actors)

In this example, Goose will use the [RAG Website Browser](https://console.apify.com/actors/3ox4R101TgZz67sLr/input) Actor to identify the most credible, top-rated Italian restaurant in Prague based on public ratings, reviews, and fine-dining recognition.

### Goose Prompt

```
( O)> Find the top-rated Italian restaurant in Prague with verifiable credibility — focus on places with at least 200+ reviews, an average rating of 4.7 or higher on Google, TripAdvisor, or OpenTable, and any Michelin or fine-dining recognition. Include the restaurant’s name, rating, total review count, address, cuisine style, and a short summary of what makes it stand out (e.g., service, ambiance, or signature dishes).
```

### Goose Output

```
Here are some top-rated Italian restaurants in Prague that meet your criteria for verifiable credibility:

### 1. [Divinis](https://guide.michelin.com/us/en/prague/prague/restaurant/divinis)
- **Rating:** 4.7
- **Review Count:** Approximately 200+ on Google
- **Address:** Prague, Czechia
- **Cuisine Style:** Italian
- **Summary:** Divinis is celebrated for its creative Italian cuisine, featuring a selection of fresh pasta and seafood dishes. The ambiance is elegant yet cozy, making it perfect for date nights. The restaurant holds a Michelin Bib Gourmand designation, indicating good quality at a reasonable price.

### 2. [CottoCrudo](https://guide.michelin.com/us/en/prague/prague/restaurant/cottocrudo)
- **Rating:** 4.7+
- **Review Count:** Over 200 reviews on various platforms
- **Address:** Prague, Czechia
- **Cuisine Style:** Italian
- **Summary:** Located in the Four Seasons Hotel, CottoCrudo offers a luxurious dining experience with a focus on authentic Italian flavors prepared with locally sourced ingredients. The restaurant features an open kitchen concept, allowing guests to watch chefs at work. It is well-known for its vibrant atmosphere and extensive wine list.

### 3. [La Finestra in Cucina](https://guide.michelin.com/us/en/prague/prague/restaurant/la-finestra)
- **Rating:** 4.8
- **Review Count:** Approximately 1,300 reviews on Google
- **Address:** Prague, Czechia
- **Cuisine Style:** Italian
- **Summary:** This restaurant is a favorite among locals and tourists alike for its authentic Italian dishes and warm hospitality. Signature dishes include handmade pastas and perfectly grilled meats. The ambiance is stylish yet comfortable, ideal for both casual dinners and special occasions.

### 4. [Aromi](https://guide.michelin.com/us/en/prague/prague/restaurant/aromi)
- **Rating:** 4.6
- **Review Count:** Over 200 reviews across platforms.
- **Address:** Prague, Czechia
- **Cuisine Style:** Italian
- **Summary:** Aromi specializes in Italian seafood and pasta, and its menu features fresh ingredients and classic recipes. The restaurant’s sleek interior and attentive service create a refined dining experience. Aromi is well-regarded for its dedicated attention to detail and a relaxing atmosphere.

### 5. [Casa De Carli](https://guide.michelin.com/us/en/prague/prague/restaurant/casa-de-carli)
- **Rating:** 4.5
- **Review Count:** Over 200 reviews.
- **Address:** Prague, Czechia
- **Cuisine Style:** Italian
- **Summary:** Casa De Carli is known for its charming decor and welcoming atmosphere. The menu offers a range of traditional Italian dishes, with an emphasis on homemade pasta. This restaurant often receives praise for its exceptional service and quality of food.

---

These restaurants not only have high ratings and substantial review counts but also feature recognitions that affirm their credibility and quality in Italian fine dining.

```
