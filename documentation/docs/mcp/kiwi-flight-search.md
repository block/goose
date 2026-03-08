---
title: Kiwi Flight Search Extension
description: Add Kiwi Flight Search MCP Server as a goose Extension
unlisted: true
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';
import { PanelLeft } from 'lucide-react';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/MhLU91zKE4M" />

Server moved: https://apify.com/agentify/kiwi-mcp-server

This tutorial covers how to add the [Kiwi Flight Search MCP Server](https://mcp.kiwi.com) as a goose extension to enable flight search and price comparison.

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
  https://mcp.kiwi.com
  ```
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `Extensions`
    3. Click `Add custom extension`
    4. On the `Add custom extension` modal, enter the following:
       - **Extension Name**: Kiwi Flight Search
       - **Type**: Streamable HTTP
       - **Endpoint**: `https://mcp.kiwi.com`
    5. Click `Add Extension` to save the extension
    6. Navigate to the chat

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="kiwi-flight-search"
      type="http"
      url="https://mcp.kiwi.com"
      timeout={300}
    />

  </TabItem>
</Tabs>

## Example Usage

Let's search for flights between any two locations on specific dates. You can search by city names or airport codes, and the extension will find available flights with pricing, duration, and booking links.

### goose Prompt

```
Can you help me search for a flight from Paris to Rome for August 9?
```

### goose Output

```
I'd be happy to help you search for flights from Paris to Rome for August 9th! Let me search for available options for you.

## ✈️ Flight Options: Paris → Rome (August 9, 2025)

### 💰 **Best Prices** (Cheapest Options)

| Route | Departure → Arrival | Class | Price | Book |
|-------|-------------------|-------|-------|------|
| Paris ORY → Rome FCO | 09/08 22:10 → 00:10+1 (2h 0m) | Economy | €138 | [Book Now](https://on.kiwi.com/ZyIJ2r) |
| Paris ORY → Rome FCO | 09/08 09:05 → 11:10 (2h 5m) | Economy | €154 | [Book Now](https://on.kiwi.com/31IDMW) |
| Paris ORY → Rome FCO | 09/08 06:25 → 08:25 (2h 0m) | Economy | €157 | [Book Now](https://on.kiwi.com/2dNs3f) |

### ⚡ **Shortest Flights** (Fastest Duration)

| Route | Departure → Arrival | Class | Price | Book |
|-------|-------------------|-------|-------|------|
| Paris ORY → Rome FCO | 09/08 06:25 → 08:25 (2h 0m) | Economy | €157 | [Book Now](https://on.kiwi.com/2dNs3f) |
| Paris ORY → Rome FCO | 09/08 22:10 → 00:10+1 (2h 0m) | Economy | €138 | [Book Now](https://on.kiwi.com/ZyIJ2r) |
| Paris CDG → Rome FCO | 09/08 21:10 → 23:10 (2h 0m) | Economy | €187 | [Book Now](https://on.kiwi.com/62jyt7) |

### 🕐 **Other Good Options**

| Route | Departure → Arrival | Class | Price | Book |
|-------|-------------------|-------|-------|------|
| Paris ORY → Rome FCO | 09/08 12:00 → 14:05 (2h 5m) | Economy | €164 | [Book Now](https://on.kiwi.com/m55oIC) |
| Paris ORY → Rome FCO | 09/08 21:10 → 23:10 (2h 0m) | Economy | €165 | [Book Now](https://on.kiwi.com/tkAU0T) |

## 📋 **Summary & Recommendations**

**🏆 Best Overall Value:** The **€138 evening flight** (ORY 22:10 → FCO 00:10+1) offers the best price with the shortest flight time, though it arrives after midnight.

**🌅 Best Morning Option:** The **€157 early flight** (ORY 06:25 → FCO 08:25) gets you to Rome early in the day with a quick 2-hour flight.

**⏰ Best Daytime Flight:** The **€154 mid-morning flight** (ORY 09:05 → FCO 11:10) offers great value and convenient timing.

All direct flights are around 2 hours, departing from either Paris Orly (ORY) or Charles de Gaulle (CDG) to Rome Fiumicino (FCO).

Have a wonderful trip to Rome! 🇮🇹 Fun fact: Rome has more fountains than any other city in the world - over 2,000 of them! Don't forget to toss a coin into the Trevi Fountain to ensure your return to the Eternal City! ⛲✨

```
