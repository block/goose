---
title: Scavio Extension
description: Add Scavio MCP server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Scavio MCP server](https://mcp.scavio.dev) as a goose extension, giving goose real-time structured data from Google search, YouTube, Amazon, Walmart, eBay, Target, TikTok, Instagram, LinkedIn, Reddit, Zillow, Indeed and around twenty other platforms, plus a generic extract endpoint that turns any URL into clean Markdown. Everything comes back as JSON, so goose never has to parse HTML.

## Configuration

<Tabs groupId="remote-or-local">
<!-- REMOTE SETUP -->
<TabItem value="remote" label="Scavio Remote MCP" default>

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.scavio.dev%2Fmcp&id=scavio&name=Scavio&description=Real-time%20structured%20web%20data%20from%20Google%2C%20YouTube%2C%20Amazon%2C%20TikTok%2C%20LinkedIn%20and%20more&header=x-api-key%3DYOUR_SCAVIO_API_KEY)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streamable HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://mcp.scavio.dev/mcp
  ```
  </TabItem>
</Tabs>

  **Custom Request Header**
  ```
  x-api-key: <YOUR_SCAVIO_API_KEY>
  ```
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="scavio"
      extensionName="Scavio"
      description="Real-time structured web data from Google, YouTube, Amazon, TikTok, LinkedIn and more"
      type="http"
      url="https://mcp.scavio.dev/mcp"
      envVars={[
        { name: "x-api-key", label: "YOUR_SCAVIO_API_KEY" }
      ]}
      apiKeyLink="https://dashboard.scavio.dev/sign-up"
      apiKeyLinkText="Scavio API Key"
    />
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Scavio"
      description="Real-time structured web data from Google, YouTube, Amazon, TikTok, LinkedIn and more"
      type="http"
      url="https://mcp.scavio.dev/mcp"
      timeout={300}
      envVars={[
        { key: "x-api-key", value: "sk_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" }
      ]}
      infoNote={
        <>
          Obtain your <a href="https://dashboard.scavio.dev/sign-up" target="_blank" rel="noopener noreferrer">Scavio API Key</a> and paste it in as the <code>x-api-key</code> header.
        </>
      }
    />
  </TabItem>
</Tabs>

</TabItem>

<!-- LOCAL SETUP -->
<TabItem value="local" label="Scavio Local MCP">

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=@scavio/mcp-server&id=mcp_scavio_local&name=Scavio%20Local%20MCP%20Server&description=Run%20the%20Scavio%20MCP%20server%20locally%20using%20your%20API%20key&env=SCAVIO_API_KEY%3DYour%20Scavio%20API%20Key)
  </TabItem>

  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @scavio/mcp-server
  ```
  </TabItem>
</Tabs>

**Environment Variables**
```
SCAVIO_API_KEY: <YOUR_SCAVIO_API_KEY>
```
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="mcp_scavio_local"
      extensionName="Scavio Local MCP Server"
      description="Real-time structured web data from Google, YouTube, Amazon, TikTok, LinkedIn and more"
      type="stdio"
      command="npx"
      args={["-y", "@scavio/mcp-server"]}
      envVars={[
        { name: "SCAVIO_API_KEY", label: "<YOUR_SCAVIO_API_KEY>" }
      ]}
      apiKeyLink="https://dashboard.scavio.dev/sign-up"
      apiKeyLinkText="Scavio API Key"
    />
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Scavio Local MCP Server"
      description="Real-time structured web data from Google, YouTube, Amazon, TikTok, LinkedIn and more"
      type="stdio"
      command="npx -y @scavio/mcp-server"
      timeout={300}
      envVars={[
        { key: "SCAVIO_API_KEY", value: "sk_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" }
      ]}
      infoNote={
        <>
          Obtain your <a href="https://dashboard.scavio.dev/sign-up" target="_blank" rel="noopener noreferrer">Scavio API Key</a> and paste it in.
        </>
      }
    />
  </TabItem>
</Tabs>

</TabItem>
</Tabs>

## Available Tools

The server exposes one tool per endpoint, grouped by platform:

| Group | Platforms |
| --- | --- |
| Search | Google (organic SERP, News, Maps, Shopping, Trends, Flights, Hotels, AI Mode, Ads Transparency) |
| Video and social | YouTube, TikTok, Instagram, Threads, X, LinkedIn, Reddit, Kuaishou |
| Commerce | Amazon, Walmart, eBay, Target, Home Depot, TikTok Shop |
| Travel | Airbnb, Booking.com, Tripadvisor |
| Jobs and reviews | Indeed, Glassdoor, Yelp, G2, Capterra |
| Real estate | Zillow, Redfin |
| App stores | Apple App Store, Google Play |
| Company filings | SEC EDGAR, UK Companies House |
| Ads | Meta Ad Library |
| Utility | Extract any URL as Markdown, plain text or HTML |

By default the server registers a subset of the most-used platforms to keep the tool list small.
Set the `x-scavio-platforms` header (remote) or `SCAVIO_PLATFORMS` environment variable (local) to
`all` to register every platform, or pass a comma-separated list such as `default,zillow,sec`.

## Example Usage

Scavio is useful when goose needs facts from the live web rather than from training data, for
example picking between similar products by current price and review volume.

### goose Prompt

```

( O)> Search Amazon for Anker power banks that can charge a laptop. Compare the top results on
price, rating and review count, and tell me which one is the best buy and why.

```

### goose Output

```

I searched Amazon and compared the laptop-capable Anker power banks:

### Anker 737 Power Bank (140W, 24,000mAh)
* Price: $109.99
* Rating: 4.4/5 from 17,400 reviews
* ASIN: B09VPHVT2Z
* Three ports, smart display, 140W is enough for full-speed MacBook Pro charging

### Anker Laptop Power Bank (165W, 25,000mAh)
* Price: $119.99
* Rating: 4.4/5 from 11,300 reviews
* ASIN: B0DCBB2YTR
* Higher total output and built-in retractable cables

### Anker Power Bank (20,000mAh, built-in USB-C cable)
* Price: $69.99
* Rating: 4.6/5 from 9,200 reviews
* ASIN: B0CXDXP8VR
* Cheapest and best-rated, but lower output

### Recommendation
The 737 at $109.99 is the best buy for laptop charging: same 4.4 rating as the 165W model but
$10 cheaper, and with 17,400 reviews it has by far the most evidence behind that score. Only go
for the 20,000mAh at $69.99 if you mainly charge phones, where its higher 4.6 rating and lower
price matter more than the output ceiling.

```

## Notes

- Requests are billed in credits rather than per request. Most calls cost 1 credit; a few cost
  more. The free tier included on signup does not require a credit card.
- Rate limits are per plan and reported in the response headers. A `429` means the plan limit was
  reached, not that the key is wrong.
