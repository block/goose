---
title: Apify MCP Extension
description: Add Apify MCP as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Apify MCP Server](https://github.com/apify/apify-mcp-server) as a Goose extension, enabling Goose to fetch context directly from Apify’s 6,000+ ready-to-use web scrapers.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40apify%2Factors-mcp-server&id=apify-mcp&name=Apify20MCP&description=Apify%20MCP&env=APIFY_TOKEN%3DYour%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  npx -y @apify/actors-mcp-server
  ```
  </TabItem>
</Tabs>
  **Endpoint URL (HTTP)**
  ```
    https://mcp.apify.com
  ```
  **Environment Variable**
  ```
  APIFY_TOKEN: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="apify-mcp"
    extensionName="Apify MCP"
    description="Apify MCP server to fetch data from 6.000+ ready-to-use web scrapers"
    command="npx"
    args={["-y", "@apify/actors-mcp-server"]}
    envVars={[{ name: "APIFY_TOKEN", label: "Your Apify Token" }]}
    apiKeyLink="https://console.apify.com/settings/integrations"
    apiKeyLinkText="Apify MCP Api Token"
  />
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

The Apify MCP server connects Goose with [Apify](https://apify.com), a platform for web scraping and automation.  
On Apify, tasks are packaged as **Actors** — serverless apps that run scrapers, crawlers, or automation scripts.  

- **Thousands of public Actors** for sites like LinkedIn, Google, Amazon, Airbnb  
- **Custom Actors** you can build and run privately  
- **Datasets** store structured results automatically, accessible via API  

#### Goose Prompt

```
( O)> Use the LinkedIn Posts Search Scraper to find all posts mentioning 'artificial intelligence' AND 'healthcare' from the past month. Extract post content, author information, company affiliations, and engagement metrics. Sort by relevance and identify thought leaders in healthcare AI.
```

#### Goose Output

```

The search for LinkedIn posts mentioning **"artificial intelligence"** and **"healthcare"** from the past month has returned a total of **48 posts**. Here’s a summary of the first **29 results**:

### Sample Results
1. **[Post by Dr. Samitria Beasley](https://www.linkedin.com/posts/dr-samitria-beasley-dha-mba-cpc-crc-401b9b70_ai-in-healthcare-will-change-many-careers-activity-7378540378960064512-8N1L?utm_source=social_share_send&utm_medium=member_desktop_web&rcm=ACoAAF8LBu8BX77nJLnHq2BuG0Oqb7swvZMxG94)**  
   - **Content:** "AI in Healthcare will change many careers."  
   - **Author:** Dr. Samitria Beasley  
   - **Profile:** [Link](https://www.linkedin.com/in/dr-samitria-beasley-dha-mba-cpc-crc-401b9b70?miniProfileUrn=urn%3Ali%3Afsd_profile%3AACoAAA8gKYEBuqDo_9hdw-oKEGJo0zaLatRClcg)  
   - **Engagement:** 5 reactions  

2. **[Post by Anderson A.](https://www.linkedin.com/posts/activity-7379847744380854272-1O1Z?utm_source=social_share_send&utm_medium=member_desktop_web&rcm=ACoAAF8LBu8BX77nJLnHq2BuG0Oqb7swvZMxG94)**  
   - **Content:** "AI in Healthcare: Balancing Innovation with Human Dignity..."  
   - **Author:** Anderson A.  
   - **Profile:** [Link](https://www.linkedin.com/in/andersonandrade25?miniProfileUrn=urn%3Ali%3Afsd_profile%3AACoAAFL9_2cBzioLyEJYJlqiUISyXJdjJQWIhY8)  
   - **Engagement:** No reactions yet  

3. **[Post by Sarkis Hakopdjanian](https://www.linkedin.com/posts/sarkis-hakopdjanian_ai-artificialintelligence-aiinhealthcare-activity-7378821010919321600-WMRb?utm_source=social_share_send&utm_medium=member_desktop_web&rcm=ACoAAF8LBu8BX77nJLnHq2BuG0Oqb7swvZMxG94)**  
   - **Content:** "AI in healthcare is being used to work smarter and faster..."  
   - **Author:** Sarkis Hakopdjanian  
   - **Profile:** [Link](https://www.linkedin.com/in/sarkis-hakopdjanian?miniProfileUrn=urn%3Ali%3Afsd_profile%3AACoAAAXXFqQBVDa-M5oSdYbz7kZcqTWzZyVbBvI)  
   - **Engagement:** 2 reactions  

4. **[Post by Data Mining Insights](https://www.linkedin.com/posts/data-mining-insight_artificial-artificial-artificial-activity-7379847713661706240-XkH2?utm_source=social_share_send&utm_medium=member_desktop_web&rcm=ACoAAF8LBu8BX77nJLnHq2BuG0Oqb7swvZMxG94)**  
   - **Content:** "Artificial Intelligence In Healthcare Sector in the Real World: 5 Uses..."  
   - **Author:** Data Mining Insights  
   - **Profile:** [Link](https://www.linkedin.com/company/data-mining-insight/posts)  
   - **Engagement:** No reactions yet  

5. **[Post by Dr. Sivakumar Subramaniam](https://www.linkedin.com/posts/drsivakumarsubramaniam_aiinhealthcare-digitalhealth-ethicalai-activity-7378267235427106816-hLyE?utm_source=social_share_send&utm_medium=member_desktop_web&rcm=ACoAAF8LBu8BX77nJLnHq2BuG0Oqb7swvZMxG94)**  
   - **Content:** "The future of healthcare is here - and Artificial Intelligence (AI)..."  
   - **Author:** Dr. Sivakumar Subramaniam  
   - **Profile:** [Link](https://www.linkedin.com/in/drsivakumarsubramaniam?miniProfileUrn=urn%3Ali%3Afsd_profile%3AACoAAAJmMngBV57EIcsSBpd_8tqfKFCiBNxrA1A)  
   - **Engagement:** 12 reactions, 7 comments  

### Observations
- A range of topics related to artificial intelligence in healthcare, including innovation, ethical concerns, and practical implications for patient care.
- Prominent figures in healthcare advocating for responsible AI usage and collaboration.
  
### Next Steps
If you would like to explore more posts or analyze specific thought leaders, let me know! I can retrieve additional data or focus on specific interests such as engagement metrics or author influence.

```
