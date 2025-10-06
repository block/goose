---
title: Apify Extension
description: Add Apify MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import { PanelLeft } from 'lucide-react';

This tutorial covers how to add the [Apify MCP Server](https://mcp.apify.com) as a Goose extension, enabling Goose to call thousands tools from [Apify Store](https://apify.com/store) to extract data from social media, e-commerce sites, search engines, online maps, or any other website.

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

The Apify MCP server lets you connect Goose to [Apify Store](https://apify.com/store), a marketplace of thousands of ready-made tools (called _Actors_) for all kinds of web scraping, crawling, and data extraction use cases.

In this example, Goose will find an Actor for [scraping LinkedIn posts](https://apify.com/store/categories?search=linkedin) to perform a quick research.

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
