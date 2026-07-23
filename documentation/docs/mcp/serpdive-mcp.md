---
title: SERPdive Web Search Extension
description: Add SERPdive MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [SERPdive MCP Server](https://github.com/serpdive/serpdive-mcp) as a goose extension to enable real-time web search that returns extracted, answer-ready page content instead of a list of links.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=serpdive-mcp&id=serpdive&name=SERPdive%20Web%20Search&description=Real-time%20web%20search%20with%20SERPdive&env=SERPDIVE_API_KEY%3DSERPdive%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y serpdive-mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  SERPDIVE_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="serpdive"
    extensionName="SERPdive Web Search"
    description="Real-time web search with SERPdive"
    command="npx"
    args={["-y", "serpdive-mcp"]}
    envVars={[
      { name: "SERPDIVE_API_KEY", label: "SERPdive API Key" }
    ]}
    apiKeyLink="https://serpdive.com/dashboard/keys"
    apiKeyLinkText="SERPdive API Key"
  />
</TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="SERPdive Web Search"
      description="Real-time web search with SERPdive"
      command="npx -y serpdive-mcp"
      envVars={[
        { key: "SERPDIVE_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          Obtain your <a href="https://serpdive.com/dashboard/keys" target="_blank" rel="noopener noreferrer">SERPdive API Key</a> and paste it in. The free tier gives 1,000 credits per month with no card required.
        </>
      }
    />
  </TabItem>
</Tabs>

## Example Usage

The SERPdive MCP server gives goose a single `serpdive_search` tool. Instead of a list of links, each result carries the extracted, answer-ready content of the source page, sized for a model's context window, with automatic localization and an optional synthesized answer.

### goose Prompt

```
Search the web for the latest changes to the EU AI Act and summarize what changed for general-purpose AI providers.
```

### goose Output

```
I'll search the web with SERPdive to find the latest on the EU AI Act.

─── serpdive_search | serpdive ──────────────────────────
query: latest changes EU AI Act general-purpose AI providers

Here's what changed for general-purpose AI (GPAI) providers, based on the extracted sources:

1. Transparency obligations
   - GPAI providers must publish a sufficiently detailed summary of the content
     used for training and keep up-to-date technical documentation.

2. Systemic-risk tier
   - Models above a compute threshold are classified as posing systemic risk and
     face additional obligations: model evaluations, adversarial testing, and
     incident reporting.

3. Timelines
   - The GPAI obligations apply on a staggered schedule, with the code-of-practice
     route offered as an interim path to demonstrate compliance.

Each point above is drawn from the answer-ready content SERPdive returned, so I
can quote or cite any of the sources directly. Want me to pull the exact passages
and their URLs?
```

:::tip
`serpdive_search` accepts a retrieval depth (`mako` for the fact-carrying sentences of each source, `moby` for the full readable page) and an optional written answer built from the sources at no extra credits. See the [SERPdive docs](https://serpdive.com/docs) for the full parameter list.
:::
