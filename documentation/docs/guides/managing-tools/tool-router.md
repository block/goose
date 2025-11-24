---
sidebar_position: 3
title: Tool Selection Strategy
sidebar_label: Tool Selection Strategy
description: Configure smart tool selection to load only relevant tools, improving performance with multiple extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';

:::warning Experimental Feature
Tool Selection Strategy is an experimental feature and currently only tested with Claude models. Behavior and configuration may change in future releases.
:::

When you enable an [extension](/docs/getting-started/using-extensions), you gain access to all of its tools. For example, the Google Drive extension provides tools for reading documents, updating permissions, managing comments, and more. By default, goose loads all tools into context when interacting with the LLM.

Enabling multiple extensions gives you access to a wider range of tools, but loading a lot of tools into context can be inefficient and confusing for the LLM. It's like having every tool in your workshop spread out on your bench when you only need one or two. 

To manage this more efficiently, you can enable a tool selection strategy. Instead of loading all tools for every interaction, it loads only the tools needed for your current task. This ensures that only the functionality you need is loaded into context, so you can keep more of your favorite extensions enabled. This provides:

- Reduced token consumption
- Improved LLM performance
- Better context management
- More accurate and efficient tool selection

## Tool Selection Options

| Option | Speed | Best For | How It Works |
|--------|-------|----------|--------------|
| **Disabled** | Fastest | Few extensions, simple setups | Loads all tools from enabled extensions |
| **Enabled** | Slower | Many extensions, complex queries | Uses LLM intelligence to select relevant tools |

:::tip
You can also use [tool permissions](/docs/guides/managing-tools/tool-permissions) to limit tool use.
:::

### Disabled (Default)
When tool selection strategy is disabled, goose loads all tools from enabled extensions into context. This is the traditional behavior and works well if you only have a few extensions enabled.

**Best for:**
- Simple setups with few extensions
- When you want all tools available at all times
- Maximum tool availability without selection logic

### Enabled (LLM-based Strategy)
When enabled, goose uses LLM intelligence to analyze your query and select only the most relevant tools from your enabled extensions. This reduces token consumption and improves tool selection accuracy when you have many extensions enabled.

**Best for:**
- Complex or ambiguous queries that require understanding context
- Setups with many extensions enabled
- When you want more accurate tool selection and reduced token usage

**Example:**
- Prompt: "help me analyze the contents of my document"
- Result: Intelligently selects document reading and analysis tools while ignoring unrelated tools like calendar or email extensions

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click the `Settings` button on the sidebar
    3. Click the `Chat` tab
    4. Under `Tool Selection Strategy`, choose your preferred option:
       - `Disabled` - Use the default tool selection strategy
       - `Enabled` - Use LLM-based intelligence to select tools
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    1. Run the configuration command:
    ```sh
    goose configure
    ```

    2. Select `goose settings`:
    ```sh
    ┌   goose-configure
    │
    ◆  What would you like to configure?
    │  ○ Configure Providers
    │  ○ Add Extension
    │  ○ Toggle Extensions
    │  ○ Remove Extension
    // highlight-start
    │  ● goose settings (Set the goose mode, Tool Output, Tool Permissions, Experiment, goose recipe github repo and more)
    // highlight-end
    └ 
    ```

    3. Select `Router Tool Selection Strategy`:
    ```sh
    ┌   goose-configure
    │
    ◇  What would you like to configure?
    │  goose settings
    │
    ◆  What setting would you like to configure?
    │  ○ goose mode 
    // highlight-start
    │  ● Router Tool Selection Strategy (Experimental: configure a strategy for auto selecting tools to use)
    // highlight-end
    │  ○ Tool Permission 
    │  ○ Tool Output 
    │  ○ Toggle Experiment 
    │  ○ goose recipe github repo 
    └ 
    ```

    4. Choose whether to enable smart tool routing:
    ```sh
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  goose settings
   │
   ◇  What setting would you like to configure?
   │  Router Tool Selection Strategy 
   │
    // highlight-start
   ◆  Would you like to enable smart tool routing?
   │  ● Enable Router (Use LLM-based intelligence to select tools)
   │  ○ Disable Router
    // highlight-end
   └  
    ```

    This example output shows that the router was enabled:
    ```
    ┌   goose-configure
    │
    ◇  What would you like to configure?
    │  goose settings
    │
    ◇  What setting would you like to configure?
    │  Router Tool Selection Strategy
    │
    ◇  Would you like to enable smart tool routing?
    │  Enable Router
    │
    └  Router enabled - using LLM-based intelligence for tool selection
    ```

    When the router is enabled, goose CLI displays a message indicating when the `llm_search` strategy is in use.

  </TabItem>
</Tabs>

## Environment Variable Configuration

You can also configure tool selection using environment variables or in your [configuration file](/docs/guides/config-files):

```bash
# Enable LLM-based tool selection
export GOOSE_ENABLE_ROUTER=true

# Disable (use default behavior)
export GOOSE_ENABLE_ROUTER=false
```

Or in your `config.yaml` file:
```yaml
GOOSE_ENABLE_ROUTER: 'true'
```