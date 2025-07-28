---
sidebar_position: 3
title: Tool Selection Strategy
sidebar_label: Tool Selection Strategy
description: Configure smart tool selection to load only relevant tools, improving performance with multiple extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';

:::info Preview Feature
The Tool Selection Strategy is currently in preview. The Vector selection strategy is currently limited to Claude models served on Databricks.
:::

When you enable an [extension](/docs/getting-started/using-extensions), you gain access to all of its tools. For example, the Google Drive extension provides tools for reading documents, updating permissions, managing comments, and more. By default, Goose loads all tools into context when interacting with the LLM.

Enabling multiple extensions gives you access to a wider range of tools, but loading a lot of tools into context can be inefficient and confusing for the LLM. It's like having every tool in your workshop spread out on your bench when you only need one or two. 

Choosing an intelligent tool selection strategy helps avoid this problem. Instead of loading all tools for every interaction, it loads only the tools needed for your current task. Both vector and LLM-based strategies ensure that only the functionality you need is loaded into context, so you can keep more of your favorite extensions enabled. These strategies provide:

- Reduced token consumption
- Improved LLM performance
- Better context management
- More accurate and efficient tool selection

## Tool Selection Strategies

| Strategy | Speed | Best For | Example Query |
|----------|-------|----------|---------------|
| **Default** | Fastest | Few extensions, simple setups | Any query (loads all tools) |
| **Vector** | Fast | Keyword-based matching | "read pdf file" |
| **LLM-based** | Slower | Complex, ambiguous queries | "analyze document contents" |

### Default Strategy
The default strategy loads all tools from enabled extensions into context, which works well if you only have a few extensions enabled. When you have more than a few extensions enabled, you should use the vector or LLM-based strategy for intelligent tool selection.

**Best for:**
- Simple setups with few extensions
- When you want all tools available at all times
- Maximum tool availability without selection logic

### Vector Strategy
The vector strategy uses mathematical similarity between embeddings to find relevant tools, providing efficient matching based on vector similarity between your query and available tools.

**Best for:**
- Situations where fast response times are critical
- Queries with keywords that match tool names or descriptions

**Example:**
- Prompt: "read pdf file"
- Result: Quickly matches with PDF-related tools based on keyword similarity

:::info Embedding Model
The default embedding model is `text-embedding-3-small`. You can change it using [environment variables](/docs/guides/environment-variables#tool-selection-strategy).
:::

### LLM-based Strategy
The LLM-based strategy leverages natural language understanding to analyze tools and queries semantically, making selections based on the full meaning of your request.

**Best for:**
- Complex or ambiguous queries that require understanding context
- Cases where exact keyword matches might miss relevant tools
- Situations where nuanced tool selection is important

**Example:**
- Prompt: "help me analyze the contents of my document"
- Result: Understands context and might suggest both PDF readers and content analysis tools

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click the `Settings` button on the sidebar
    3. Click `Chat`
    4. Under `Tool Selection Strategy`, select your preferred strategy:
       - `Default`
       - `Vector`
       - `LLM-based`
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    1. Run the `configuration` command:
    ```sh
    goose configure
    ```

    2. Select `Goose Settings`:
    ```sh
    ┌   goose-configure
    │
    ◆  What would you like to configure?
    │  ○ Configure Providers
    │  ○ Add Extension
    │  ○ Toggle Extensions
    │  ○ Remove Extension
    // highlight-start
    │  ● Goose Settings (Set the Goose Mode, Tool Output, Tool Permissions, Experiment, Goose recipe github repo and more)
    // highlight-end
    └ 
    ```

    3. Select `Router Tool Selection Strategy`:
    ```sh
    ┌   goose-configure
    │
    ◇  What would you like to configure?
    │  Goose Settings
    │
    ◆  What setting would you like to configure?
    │  ○ Goose Mode 
    // highlight-start
    │  ● Router Tool Selection Strategy (Configure the strategy for selecting tools to use)
    // highlight-end
    │  ○ Tool Permission 
    │  ○ Tool Output 
    │  ○ Toggle Experiment 
    │  ○ Goose recipe github repo 
    └ 
    ```

    4. Select your preferred strategy:
    ```sh
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Goose Settings 
   │
   ◇  What setting would you like to configure?
   │  Router Tool Selection Strategy 
   │
    // highlight-start
   ◆  Which router strategy would you like to use?
   │  ● Vector Strategy (Use vector-based similarity to select tools)
   │  ○ Default Strategy 
    // highlight-end
   └  
    ```
      
       :::info
       Currently, the LLM-based strategy can't be configured using the CLI.
       :::

       This example output shows the `Vector Strategy` was selected:

    ```
    ┌   goose-configure
    │
    ◇  What would you like to configure?
    │  Goose Settings
    │
    ◇  What setting would you like to configure?
    │  Router Tool Selection Strategy
    │
    ◇  Which router strategy would you like to use?
    │  Vector Strategy
    │
    └  Set to Vector Strategy - using vector-based similarity for tool selection
    ```

    Goose CLI display a message indicating when the vector or LLM-based strategy is currently being used.

  </TabItem>
</Tabs>