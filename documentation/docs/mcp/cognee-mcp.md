---
title: Cognee Extension
description: Add Cognee MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Cognee MCP Server](https://github.com/topoteretes/cognee/tree/main/cognee-mcp) as a goose extension to enable knowledge graph memory capabilities, connecting to over 30 data sources for enhanced context and retrieval.

:::tip TLDR
**Command**
```sh
uv --directory /path/to/cognee-mcp run python src/server.py
```
**Environment Variables**
```
LLM_API_KEY: <YOUR_OPENAI_API_KEY>
```
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uv`.
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="goose CLI" default>

**Install Cognee:**

```bash
# Clone and install Cognee
git clone https://github.com/topoteretes/cognee
cd cognee/cognee-mcp
uv sync --dev --all-extras --reinstall

# On Linux, install additional dependencies
sudo apt install -y libpq-dev python3-dev
```

**Configure the extension:**

<CLIExtensionInstructions
  name="Cognee"
  description="Knowledge graph memory capabilities with 30+ data source connections"
  command="uv --directory /path/to/cognee-mcp run python src/server.py"
  envVars={[
    { key: "LLM_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
  ]}
  commandNote={
    <>
      Replace <code>/path/to/cognee-mcp</code> with the actual path to your cloned cognee-mcp directory.
    </>
  }
  infoNote={
    <>
      Obtain an API key for your LLM provider and paste it in. By default, this is an <a href="https://platform.openai.com/api-keys" target="_blank" rel="noopener noreferrer">OpenAI API key</a>.
    </>
  }
/>
  </TabItem>
</Tabs>

:::info
See the [Cognee MCP documentation](https://docs.cognee.ai/how-to-guides/deployment/mcp) for supported configuration options. 
:::

## Example Usage

Cognee provides knowledge graph memory capabilities for goose, allowing it to remember and connect information across conversations and documents.

:::info LLM
OpenAI's GPT-4o was used for this task.
:::

### goose Prompt

> _goose, please cognify this information: "I prefer Python for data analysis and use pandas extensively. My current project involves analyzing customer behavior data." Then search for information about my programming preferences._

### goose Output

:::note CLI

<details>
    <summary>Tool Calls</summary>

    ─── cognee_cognify | abcdefgh ──────────────────────────

    data: I prefer Python for data analysis and use pandas extensively. My current project involves analyzing customer behavior data.


    ─── cognee_search | abcdefgh ──────────────────────────

    search_query: programming preferences Python data analysis

    search_type: GRAPH_COMPLETION

</details>

I'll help you store that information in the knowledge graph and then search for your programming preferences.

First, let me cognify the information about your preferences:

**Cognify Status:**
Background process launched due to MCP timeout limitations. The information about your Python and pandas preferences for data analysis has been added to the knowledge graph.

Now let me search for information about your programming preferences:

**Search Results:**
Based on the knowledge graph, I found information about your programming preferences:

- **Language Preference**: You prefer Python for data analysis
- **Key Libraries**: You use pandas extensively for data manipulation
- **Current Focus**: Your current project involves analyzing customer behavior data
- **Domain**: Data science and analytics workflows
- **Technical Stack**: Python + pandas for data analysis tasks

The knowledge graph has successfully stored and can now retrieve your programming preferences and project context for future conversations.

:::
