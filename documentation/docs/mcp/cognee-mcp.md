---
title: Cognee Extension
description: Add Cognee MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Cognee MCP Server](https://github.com/topoteretes/cognee) as a Goose extension to enable knowledge graph memory capabilities, connecting to over 30 data sources for enhanced context and retrieval.

:::tip TLDR
**Command**
```sh
uv --directory /path/to/cognee-mcp run python src/server.py
```
**Environment Variables**
```
LLM_API_KEY: <YOUR_OPENAI_API_KEY>
EMBEDDING_API_KEY: <YOUR_OPENAI_API_KEY>
```
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uv`.
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>

1. First, install Cognee:
```bash
# Clone and install Cognee
git clone https://github.com/topoteretes/cognee
cd cognee-mcp
uv sync --dev --all-extras --reinstall

# On Linux, install additional dependencies
sudo apt install -y libpq-dev python3-dev
```

2. Run the `configure` command:
```sh
goose configure
```

3. Choose to add a `Command-line Extension`
```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◆  What type of extension would you like to add?
  │  ○ Built-in Extension 
  // highlight-start    
  │  ● Command-line Extension (Run a local command or script)
  // highlight-end    
  │  ○ Remote Extension (SSE) 
  │  ○ Remote Extension (Streaming HTTP) 
  └ 
```

4. Give your extension a name
```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Command-line Extension 
  │
  // highlight-start
  ◆  What would you like to call this extension?
  │  Cognee
  // highlight-end
  └ 
```

5. Enter the command
```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Command-line Extension 
  │
  ◇  What would you like to call this extension?
  │  Cognee
  │
  // highlight-start
  ◆  What command should be run?
  │  uv --directory /path/to/cognee-mcp run python src/server.py
  // highlight-end
  └ 
```  

6. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Command-line Extension 
  │
  ◇  What would you like to call this extension?
  │  Cognee
  │
  ◇  What command should be run?
  │  uv --directory /path/to/cognee-mcp run python src/server.py
  │
  // highlight-start
  ◆  Please set the timeout for this tool (in secs):
  │  300
  // highlight-end
  │
  └ 
``` 

7. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Command-line Extension 
  │
  ◇  What would you like to call this extension?
  │  Cognee
  │
  ◇  What command should be run?
  │  uv --directory /path/to/cognee-mcp run python src/server.py
  │
  ◇  Please set the timeout for this tool (in secs):
  │  300
  │
  // highlight-start
  ◇  Would you like to add a description?
  │  No
  // highlight-end
  │
  └ 
```

8. Add the required environment variables:
:::info
You'll need OpenAI API keys for both LLM and embedding models. [Get your API keys here](https://platform.openai.com/api-keys).
:::

 ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Command-line Extension 
  │
  ◇  What would you like to call this extension?
  │  Cognee
  │
  ◇  What command should be run?
  │  uv --directory /path/to/cognee-mcp run python src/server.py
  │     
  ◇  Please set the timeout for this tool (in secs):
  │  300
  │
  ◇  Would you like to add a description?
  │  No
  │    
  // highlight-start
  ◆  Would you like to add environment variables?
  │  Yes 
  │
  ◇  Environment variable name:
  │  LLM_API_KEY
  │
  ◇  Environment variable value:
  │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
  │
  ◇  Add another environment variable?
  │  Yes
  │
  ◇  Environment variable name:
  │  EMBEDDING_API_KEY
  │
  ◇  Environment variable value:
  │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
  │
  ◇  Add another environment variable?
  │  No 
  // highlight-end
  └  Added Cognee extension
```

  </TabItem>
</Tabs>

## Example Usage

Cognee provides knowledge graph memory capabilities for Goose, allowing it to remember and connect information across conversations and documents.

:::info LLM
OpenAI's GPT-4o was used for this task.
:::

### Goose Prompt

> _Goose, please cognify this information: "I prefer Python for data analysis and use pandas extensively. My current project involves analyzing customer behavior data." Then search for information about my programming preferences._

### Goose Output

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
