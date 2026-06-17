---
title: Sofya Extension
description: Add Sofya MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Sofya MCP Server](https://sofya.co) as a goose extension to give goose web access through one remote server: search the web, fetch any page as clean markdown, extract structured data with AI, and run deep multi-source research.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.sofya.co%2Fmcp&id=sofya&name=Sofya&description=Web%20search%2C%20fetch%2C%20extract%2C%20and%20deep%20research&header=Authorization%3DBearer%20ay_live_YOUR_API_KEY)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streaming HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://mcp.sofya.co/mcp
  ```
  </TabItem>
</Tabs>

  **Custom Request Header**
  ```
  Authorization: Bearer ay_live_<YOUR_SOFYA_API_KEY>
  ```
:::

## Configuration

These steps configure the remote Sofya MCP server. You'll need a Sofya API key, which you can get from the [Sofya dashboard](https://sofya.co/dashboard). The same key works for both the REST API and MCP.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="sofya"
      extensionName="Sofya"
      description="Web search, fetch, extract, and deep research"
      type="http"
      url="https://mcp.sofya.co/mcp"
      envVars={[
        { name: "Authorization", label: "Bearer ay_live_YOUR_API_KEY" }
      ]}
      apiKeyLink="https://sofya.co/dashboard"
      apiKeyLinkText="Sofya API Key"
    />

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="sofya"
      description="Web search, fetch, extract, and deep research"
      type="http"
      url="https://mcp.sofya.co/mcp"
      timeout={300}
      envVars={[
        { key: "Authorization", value: "Bearer ay_live_xxxxxxxxxxxxxxxxxxxxxxxxxxxx" }
      ]}
      infoNote={
        <>
          Obtain your <a href="https://sofya.co/dashboard" target="_blank" rel="noopener noreferrer">Sofya API key</a> and paste it in as the <code>Bearer</code> token.
        </>
      }
    />

  </TabItem>
</Tabs>

## Example Usage

Sofya exposes four tools that ground goose in current web information:

1. `search`: web search that returns full page content, not just snippets, with an optional AI-synthesized answer
2. `fetch`: retrieve one or more URLs as clean markdown (also handles PDFs and documents)
3. `extract`: pull specific structured data from a page using AI
4. `research`: deep multi-source research that decomposes a question, reads many sources, and synthesizes a cited report

### goose Prompt

```
Search the news for the latest open-weight language model releases from the past month and summarize the most important ones.
```

### goose Output

:::note Desktop

I'll search recent news for open-weight model releases.

─── search | sofya ──────────────────────────
query: latest open-weight large language model releases
topic: news
freshness: month
include_answer: true



Here are the most notable open-weight releases from the past month:

- NVIDIA Nemotron 3 Ultra (released 2026-06-04), a 550B-parameter Mixture-of-Experts model with 55B active parameters, built for frontier reasoning and orchestration in long-running agent workflows.
- Cohere Command A+, a 218B-parameter Sparse Mixture-of-Experts model released under a permissive Apache 2.0 license and available on Hugging Face.
- Apertus, Switzerland's first large-scale, fully open multilingual LLM from EPFL, ETH Zurich, and CSCS, now served through public inference endpoints.

Established open families like Meta's Llama, Mistral, DeepSeek, and Minimax continue to gain enterprise traction, alongside open offshoots of proprietary models such as Google's Gemma and OpenAI's GPT-OSS.

Want me to fetch any of these announcements in full, or run deeper research comparing them?

:::

:::tip
For broad, comparative questions, ask goose to use Sofya's `research` tool, which reads many sources in parallel and returns a synthesized report with citations. For targeted lookups, `search` (optionally with an answer) is faster and cheaper. See the [Sofya documentation](https://sofya.co) for the full tool reference and credit costs.
:::
