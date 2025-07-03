---
title: Cognee Extension
description: Add Cognee MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Cognee MCP Server](https://github.com/topoteretes/cognee) as a Goose extension to enable knowledge graph memory capabilities, connecting to over 30 data sources for enhanced context and retrieval.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  Manual configuration required - see Configuration section below
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  uv --directory /path/to/cognee-mcp run python src/server.py
  ```
  </TabItem>
</Tabs>
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

### Installation

First, install Cognee:

```bash
# Install uv if you haven't already
curl -LsSf https://astral.sh/uv/install.sh | sh

# Clone and install Cognee
git clone https://github.com/topoteretes/cognee
cd cognee-mcp
uv sync --dev --all-extras --reinstall

# On Linux, install additional dependencies
sudo apt install -y libpq-dev python3-dev
```

:::info
The initial setup requires a first start to install dependencies. Press CTRL+C to stop after it starts:
```bash
uv run cognee
```
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  
  Manual configuration is required for Cognee. Add the following to your Goose configuration file:

  **Method 1: Direct Server Start (Slower)**
  ```yaml
  extensions:
    cognee_mcp:
      bundled: false
      display_name: "cognee-mcp"
      enabled: true
      name: "cognee-mcp"
      timeout: 300
      type: stdio
      cmd: uv
      args:
        - --directory
        - /home/YOURNAME/.local/share/cognee/cognee-mcp
        - run
        - python
        - src/server.py
      description: "Runs the cognee-mcp server instance"
      envs:
        DEBUG: "true"
        HOST: "localhost"
        COGNEE_DIR: "/home/YOURNAME/.local/share/cognee"
        COGNEE_MCP_DIR: "/home/YOURNAME/.local/share/cognee/cognee-mcp"
        ENVIRONMENT: "LOCAL"
        ENV: "LOCAL"
        LOG_LEVEL: "INFO"
        LLM_API_KEY: "your-openai-api-key"
        LLM_MODEL: "openai/gpt-4.1-nano-2025-04-14"
        EMBEDDING_API_KEY: "your-openai-api-key"
        EMBEDDING_MODEL: "openai/text-embedding-3-large"
        RATE_LIMIT_INTERVAL: "60"
  ```

  **Method 2: Connect to Running Server (Preferred)**
  ```yaml
  extensions:
    cognee-mcp:
      bundled: null
      uri: http://0.0.0.0:8000/sse
      description: Connects to a running Cognee memory server.
      enabled: true
      name: cognee-mcp
      timeout: 300
      type: sse
  ```

  :::warning
  Replace `YOURNAME` with your actual username. Goose configuration doesn't expand variables like `$HOME` or `~`.
  :::

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  <CLIExtensionInstructions
    name="cognee-mcp"
    command="uv --directory /path/to/cognee-mcp run python src/server.py"
    timeout={300}
    envVars={[
      { key: "LLM_API_KEY", value: "••••••••••••••••" },
      { key: "EMBEDDING_API_KEY", value: "••••••••••••••••" },
      { key: "LLM_MODEL", value: "openai/gpt-4.1-nano-2025-04-14" },
      { key: "EMBEDDING_MODEL", value: "openai/text-embedding-3-large" }
    ]}
    infoNote={
      <>
        You'll need OpenAI API keys for both LLM and embedding models.{" "}
        <a
          href="https://platform.openai.com/api-keys"
          target="_blank"
          rel="noopener noreferrer"
        >
          Get your API keys here
        </a>.
      </>
    }
  />

  </TabItem>
</Tabs>

### Running Cognee Server (Method 2 - Recommended)

For better performance, run Cognee as a separate server:

1. Create a startup script `start-cognee.sh`:
```bash
#!/bin/bash
set -e

export DEBUG=true
export HOST=localhost
export ENVIRONMENT=LOCAL
export LLM_API_KEY=${OPENAI_API_KEY}
export LLM_MODEL=openai/gpt-4.1-nano-2025-04-14
export EMBEDDING_API_KEY=${OPENAI_API_KEY}
export EMBEDDING_MODEL=openai/text-embedding-3-large
export RATE_LIMIT_INTERVAL=60

cd /path/to/cognee-mcp
uv sync --dev --all-extras
uv run python src/server.py --transport sse
```

2. Make it executable and run:
```bash
chmod +x start-cognee.sh
./start-cognee.sh
```

## Example Usage

Cognee provides knowledge graph memory capabilities for Goose, allowing it to remember and connect information across conversations.

### Goose Prompt

> _Goose, please cognify this information: "I prefer Python for data analysis and use pandas extensively. My current project involves analyzing customer behavior data." Then search for information about my programming preferences._

### Goose Output

:::note Desktop

I'll help you store that information in the knowledge graph and then search for your programming preferences.

First, let me cognify the information about your preferences:

```
Background process launched due to MCP timeout limitations.
To check current cognify status use the cognify_status tool
or check the log file at: /path/to/logs
```

Now let me search for information about your programming preferences:

```
Based on the knowledge graph, I found information about your programming preferences:

- You prefer Python for data analysis
- You use pandas extensively 
- Your current project involves analyzing customer behavior data
- This suggests you work with data science and analytics workflows

The knowledge graph has successfully stored and can retrieve your programming preferences and project context.
```

:::

## Available Tools

Cognee provides several tools for knowledge graph operations:

- **cognify**: Transform data into structured knowledge graph
- **search**: Query the knowledge graph with different search types (GRAPH_COMPLETION, INSIGHTS, CHUNKS, etc.)
- **codify**: Analyze code repositories and create code knowledge graphs
- **cognify_status**: Check the status of cognify operations
- **prune**: Reset the knowledge graph (use with caution)
- **cognee_add_developer_rules**: Ingest developer configuration files

For advanced usage patterns and automation strategies, see our [detailed Cognee tutorial](../tutorials/advanced-cognee-usage.md).
