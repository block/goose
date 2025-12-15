---
title: Tavily Web Search Extension
description: Add Tavily MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

<YouTubeShortEmbed videoUrl="https://youtube.com/embed/mxS2G9afGxE" />

This tutorial covers how to add the [Tavily Web Search MCP Server](https://github.com/tavily-ai/tavily-mcp) as a goose extension to enable AI-powered web search functionality.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=tavily-mcp&id=tavily&name=Tavily%20Web%20Search&description=Search%20the%20web%20with%20Tavily%20MCP&env=TAVILY_API_KEY%3DTavily%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y tavily-mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  TAVILY_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="tavily"
    extensionName="Tavily Web Search"
    description="Search the web with Tavily MCP"
    command="npx"
    args={["-y", "tavily-mcp"]}
    envVars={[
      { name: "TAVILY_API_KEY", label: "Tavily API Key" }
    ]}
    apiKeyLink="https://tavily.com/"
    apiKeyLinkText="Tavily API Key"
  />
</TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Tavily Web Search"
      description="Search the web with Tavily MCP"
      command="npx -y tavily-mcp"
      envVars={[
        { key: "TAVILY_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          Obtain your <a href="https://tavily.com/" target="_blank" rel="noopener noreferrer">Tavily API Key</a> and paste it in. You can get your API key by signing up at <a href="https://app.tavily.com/" target="_blank" rel="noopener noreferrer">tavily.com</a> and navigating to your account settings.
        </>
      }
    />
  </TabItem>
</Tabs>

## Example Usage

The Tavily MCP server enables AI-powered web search functionality in your goose interactions. When you obtain your API key, you'll have access to Tavily's advanced search capabilities including:

1. Real-time web search
2. Structured data extraction
3. Content analysis
4. Topic-specific search

### goose Prompt

```
Search for recent news about artificial intelligence breakthroughs in medicine and summarize the key findings.
```

### goose Output

```
I'll use the Tavily news search tool to find recent news about AI breakthroughs in medicine.

─── tavily_news_search | tavily ──────────────────────────
max_results: 5
query: ...



Let me summarize the key findings from the recent news about AI in medicine:

1. Personalized AI-Driven Healthcare:
- Development of AI systems that act as "personal doctors" that work alongside human clinicians rather than replacing them
- Focus on augmenting clinical capabilities while maintaining human oversight

2. Breakthrough in Medical Imaging:
- A significant advancement in CT scan analysis has been reported in Nature
- Development of a "Unified total-body CT image" (Uni-CT) algorithm that:
  - Automatically generates single series of total-body CT images
  - Concatenates multiple images of segmented organs using organ-specific window settings
  - Reduces diagnosis time by 35% without compromising accuracy
  - Improves efficiency in trauma cases

3. Future Trends:
- Movement toward democratized AI in healthcare, potentially reducing the gap between large and small healthcare providers
- Development of hyper-personalized medical experiences powered by AI
- Integration of AI systems that can perform human-like reasoning in medical contexts

The most concrete breakthrough appears to be in medical imaging, with the Uni-CT algorithm showing promising results in improving both the speed and accuracy of trauma diagnosis. This represents a practical application of AI that could have immediate benefits in emergency medicine.

Would you like me to search for more specific information about any of these developments or explore other areas of AI in medicine?
```

:::tip
You can adjust the [search parameters](https://github.com/tavily-ai/tavily-mcp#tavily-search-examples) for different types of queries and depths of information. The extension supports both quick searches and comprehensive research.
:::