---
title: Cognee Extension
description: Add Cognee MCP Server as a Goose Extension
authors: 
    - KevinCojean
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Using the Cognee MCP with Goose

:::warning
This guide is tailored for a Linux system; the outline would remain the same for any other platform.
:::

This tutorial explains how to integrate Cognee as an extension for Goose, enhancing its memory capabilities. This allows Goose to connect to a knowledge graph. [Cognee](https://github.com/topoteretes/cognee) facilitates supports over 30 data sources.

Key features include:
- Interconnecting and retrieving past conversations, documents, images, and audio transcriptions.
- Replace traditional RAG systems.
- Manipulate data dynamically while ingesting from 30+ supported sources.

## Configuration


### Installation

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI">

You first need to install `uv`, which is a fast package manager for Python which is strongly recommended for usage with Cognee.
```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Then, install `Cognee`.

```bash
git clone https://github.com/topoteretes/cognee
cd cognee-mcp
uv sync --dev --all-extras --reinstall
sudo apt install -y libpq-dev python3-dev
```
:::info
The initial setup requires a first start; so press CTRL+C to stop the MCP after it has started and installed dependencies.
```bash
uv run cognee
```
:::

  </TabItem>
</Tabs>

You can configure Goose and the Cognee MCP server in two ways.
- Method 1: Goose starts a Cognee-MCP server (slow)
- Method 2: Goose connects to a running Cognee-MCP instance (preferred)

<Tabs groupId="interface">
<TabItem value="method-1" label="Method 1 (slow)">

Every time you start Goose, an Cognee MCP instance will also start. This adds significant at start for Goose.

#### Goose configuration

Here is the expected [extension configuration](https://block.github.io/goose/docs/guides/config-file#extensions-configuration) for the cognee-mcp extension.

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
    envs: {
      "DEBUG": "true",
      "HOST": "localhost",
      "COGNEE_DIR": "/home/YOURNAME/.local/share/cognee",
      "COGNEE_MCP_DIR": "/home/YOURNAME/.local/share/cognee/cognee-mcp",
      "COGNEE_VENV_DIR": "/home/YOURNAME/.local/share/cognee/cognee-mcp",
      "ENVIRONMENT": "LOCAL",
      "ENV": "LOCAL",
      "LOG_LEVEL": "INFO",
      "LLM_API_KEY": "xxxxxxxxxxxx",
      "LLM_MODEL": "openai/gpt-4.1-nano-2025-04-14",
      "EMBEDDING_API_KEY": "xxxxxxxxxxx",
      "EMBEDDING_MODEL": "openai/text-embedding-3-large",
      "RATE_LIMIT_INTERVAL": "60"
  }
```

:::warning
Do not forget to replace `YOURNAME` in the above configuration. The Goose extension configuration is not capable of expanding variables such as `$HOME` or symbols such as `~`; you must write the complete path to your home.
:::

You may then start `goose`, which will take between 5 and 20 seconds (depending on the machine).

:::info Logs and errors
You may check the generated logs for `cognee-mcp` in this relative directory, where you installed cognee:  
`.venv/lib/python3.11/site-packages/logs`
:::

</TabItem>

  <TabItem value="method-2" label="Method 2 (preferred)">

Every time you start Goose, it will attempt to connect to an already running Cognee MCP server. This significantly saves time when Goose starts.

#### Creating the script which starts the MCP Server

##### Bash

1. Place the following script at the root directory where you installed Cognee.

:::info
Don't forget to give the script executable permissions with `chmod +x`
:::

```bash
#!/bin/bash
set -e

# Configuration
# Replace LLM_API_KEY, EMBEDDING_API_KEY, and models as your prefer.
export DEBUG=true
export HOST=localhost
export ENVIRONMENT=LOCAL
export ENV=${ENVIRONMENT}
export LLM_API_KEY=${OPENAI_API_KEY}
export LLM_MODEL=openai/gpt-4.1-nano-2025-04-14
export EMBEDDING_API_KEY=${OPENAI_API_KEY}
export EMBEDDING_MODEL=openai/text-embedding-3-large
export RATE_LIMIT_INTERVAL=60

uv init || echo "Error $?: encountered, perhaps the project is already initialized?"
uv sync --dev --all-extras
uv run python run-cognee-mcp-server.py --transport sse
```

##### Python

Copy paste the following into a file `run-cognee-mcp-server.py`, at the root directory where you installed Cognee.

> This is mostly a copy-paste of the default `server.py` from Cognee; with the difference that we specify endpoints for the `sse` and `streamable-http` protocols. You may change the server host and port as well.

<details>
<summary>A slightly modified `cognee-mcp/server.py` file.</summary>

```python title="server.py"
import json
import os
import sys
import argparse
import cognee
import asyncio
from cognee.shared.logging_utils import get_logger, get_log_file_location
import importlib.util
from contextlib import redirect_stdout
import mcp.types as types
from mcp.server import FastMCP
from cognee.modules.pipelines.operations.get_pipeline_status import get_pipeline_status
from cognee.modules.data.methods.get_unique_dataset_id import get_unique_dataset_id
from cognee.modules.users.methods import get_default_user
from cognee.api.v1.cognify.code_graph_pipeline import run_code_graph_pipeline
from cognee.modules.search.types import SearchType
from cognee.shared.data_models import KnowledgeGraph
from cognee.modules.storage.utils import JSONEncoder

# https://github.com/jlowin/fastmcp/blob/eeedd175a55f7ddcde21b8fb201f0fac1f3810e0/src/fastmcp/server/server.py#L109
mcp = FastMCP(
    name="Cognee",
    host="0.0.0.0",
    port="8000",
    log_level="INFO",
    sse_path="/sse",
    message_path="/message",
    streamable_http_path="/streamble-http"
)

logger = get_logger()
log_file = get_log_file_location()


@mcp.tool()
async def cognee_add_developer_rules(
    base_path: str = ".", graph_model_file: str = None, graph_model_name: str = None
) -> list:
    """
    Ingest core developer rule files into Cognee's memory layer.

    This function loads a predefined set of developer-related configuration,
    rule, and documentation files from the base repository and assigns them
    to the special 'developer_rules' node set in Cognee. It ensures these
    foundational files are always part of the structured memory graph.

    Parameters
    ----------
    base_path : str
        Root path to resolve relative file paths. Defaults to current directory.

    graph_model_file : str, optional
        Optional path to a custom schema file for knowledge graph generation.

    graph_model_name : str, optional
        Optional class name to use from the graph_model_file schema.

    Returns
    -------
    list
        A message indicating how many rule files were scheduled for ingestion,
        and how to check their processing status.

    Notes
    -----
    - Each file is processed asynchronously in the background.
    - Files are attached to the 'developer_rules' node set.
    - Missing files are skipped with a logged warning.
    """

    developer_rule_paths = [
        ".cursorrules",
        ".cursor/rules",
        ".same/todos.md",
        ".windsurfrules",
        ".clinerules",
        "CLAUDE.md",
        ".sourcegraph/memory.md",
        "AGENT.md",
        "AGENTS.md",
    ]

    async def cognify_task(file_path: str) -> None:
        with redirect_stdout(sys.stderr):
            logger.info(f"Starting cognify for: {file_path}")
            try:
                await cognee.add(file_path, nodeset="developer_rules")
                model = KnowledgeGraph
                if graph_model_file and graph_model_name:
                    model = load_class(graph_model_file, graph_model_name)
                await cognee.cognify(graph_model=model)
                logger.info(f"Cognify finished for: {file_path}")
            except Exception as e:
                logger.error(f"Cognify failed for {file_path}: {str(e)}")

    tasks = []
    for rel_path in developer_rule_paths:
        abs_path = os.path.join(base_path, rel_path)
        if os.path.isfile(abs_path):
            tasks.append(asyncio.create_task(cognify_task(abs_path)))
        else:
            logger.warning(f"Skipped missing developer rule file: {abs_path}")

    return [
        types.TextContent(
            type="text",
            text=(
                f"Started cognify for {len(tasks)} developer rule files in background.\n"
                f"All are added to the `developer_rules` node set.\n"
                f"Use `cognify_status` or check logs at {log_file} to monitor progress."
            ),
        )
    ]


@mcp.tool()
async def cognify(data: str, graph_model_file: str = None, graph_model_name: str = None) -> list:
    """
    Transform data into a structured knowledge graph in Cognee's memory layer.

    This function launches a background task that processes the provided text/file location and
    generates a knowledge graph representation. The function returns immediately while
    the processing continues in the background due to MCP timeout constraints.

    Parameters
    ----------
    data : str
        The data to be processed and transformed into structured knowledge.
        This can include natural language, file location, or any text-based information
        that should become part of the agent's memory.

    graph_model_file : str, optional
        Path to a custom schema file that defines the structure of the generated knowledge graph.
        If provided, this file will be loaded using importlib to create a custom graph model.
        Default is None, which uses Cognee's built-in KnowledgeGraph model.

    graph_model_name : str, optional
        Name of the class within the graph_model_file to instantiate as the graph model.
        Required if graph_model_file is specified.
        Default is None, which uses the default KnowledgeGraph class.

    Returns
    -------
    list
        A list containing a single TextContent object with information about the
        background task launch and how to check its status.

    Notes
    -----
    - The function launches a background task and returns immediately
    - The actual cognify process may take significant time depending on text length
    - Use the cognify_status tool to check the progress of the operation
    """

    async def cognify_task(
        data: str, graph_model_file: str = None, graph_model_name: str = None
    ) -> str:
        """Build knowledge graph from the input text"""
        # NOTE: MCP uses stdout to communicate, we must redirect all output
        #       going to stdout ( like the print function ) to stderr.
        with redirect_stdout(sys.stderr):
            logger.info("Cognify process starting.")
            if graph_model_file and graph_model_name:
                graph_model = load_class(graph_model_file, graph_model_name)
            else:
                graph_model = KnowledgeGraph

            await cognee.add(data)

            try:
                await cognee.cognify(graph_model=graph_model)
                logger.info("Cognify process finished.")
            except Exception as e:
                logger.error("Cognify process failed.")
                raise ValueError(f"Failed to cognify: {str(e)}")

    asyncio.create_task(
        cognify_task(
            data=data,
            graph_model_file=graph_model_file,
            graph_model_name=graph_model_name,
        )
    )

    text = (
        f"Background process launched due to MCP timeout limitations.\n"
        f"To check current cognify status use the cognify_status tool\n"
        f"or check the log file at: {log_file}"
    )

    return [
        types.TextContent(
            type="text",
            text=text,
        )
    ]


@mcp.tool()
async def codify(repo_path: str) -> list:
    """
    Analyze and generate a code-specific knowledge graph from a software repository.

    This function launches a background task that processes the provided repository
    and builds a code knowledge graph. The function returns immediately while
    the processing continues in the background due to MCP timeout constraints.

    Parameters
    ----------
    repo_path : str
        Path to the code repository to analyze. This can be a local file path or a
        relative path to a repository. The path should point to the root of the
        repository or a specific directory within it.

    Returns
    -------
    list
        A list containing a single TextContent object with information about the
        background task launch and how to check its status.

    Notes
    -----
    - The function launches a background task and returns immediately
    - The code graph generation may take significant time for larger repositories
    - Use the codify_status tool to check the progress of the operation
    - Process results are logged to the standard Cognee log file
    - All stdout is redirected to stderr to maintain MCP communication integrity
    """

    async def codify_task(repo_path: str):
        # NOTE: MCP uses stdout to communicate, we must redirect all output
        #       going to stdout ( like the print function ) to stderr.
        with redirect_stdout(sys.stderr):
            logger.info("Codify process starting.")
            results = []
            async for result in run_code_graph_pipeline(repo_path, False):
                results.append(result)
                logger.info(result)
            if all(results):
                logger.info("Codify process finished succesfully.")
            else:
                logger.info("Codify process failed.")

    asyncio.create_task(codify_task(repo_path))

    text = (
        f"Background process launched due to MCP timeout limitations.\n"
        f"To check current codify status use the codify_status tool\n"
        f"or you can check the log file at: {log_file}"
    )

    return [
        types.TextContent(
            type="text",
            text=text,
        )
    ]


@mcp.tool()
async def search(search_query: str, search_type: str) -> list:
    """
    Search the Cognee knowledge graph for information relevant to the query.

    This function executes a search against the Cognee knowledge graph using the
    specified query and search type. It returns formatted results based on the
    search type selected.

    Parameters
    ----------
    search_query : str
        The search query in natural language. This can be a question, instruction, or
        any text that expresses what information is needed from the knowledge graph.

    search_type : str
        The type of search to perform. Valid options include:
        - "GRAPH_COMPLETION": Returns an LLM response based on the search query and Cognee's memory
        - "RAG_COMPLETION": Returns an LLM response based on the search query and standard RAG data
        - "CODE": Returns code-related knowledge in JSON format
        - "CHUNKS": Returns raw text chunks from the knowledge graph
        - "INSIGHTS": Returns relationships between nodes in readable format

        The search_type is case-insensitive and will be converted to uppercase.

    Returns
    -------
    list
        A list containing a single TextContent object with the search results.
        The format of the result depends on the search_type:
        - For CODE: JSON-formatted search results
        - For GRAPH_COMPLETION/RAG_COMPLETION: A single text completion
        - For CHUNKS: String representation of the raw chunks
        - For INSIGHTS: Formatted string showing node relationships
        - For other types: String representation of the search results

    Notes
    -----
    - Different search types produce different output formats
    - The function handles the conversion between Cognee's internal result format and MCP's output format
    """

    async def search_task(search_query: str, search_type: str) -> str:
        """Search the knowledge graph"""
        # NOTE: MCP uses stdout to communicate, we must redirect all output
        #       going to stdout ( like the print function ) to stderr.
        with redirect_stdout(sys.stderr):
            search_results = await cognee.search(
                query_type=SearchType[search_type.upper()], query_text=search_query
            )

            if search_type.upper() == "CODE":
                return json.dumps(search_results, cls=JSONEncoder)
            elif (
                search_type.upper() == "GRAPH_COMPLETION" or search_type.upper() == "RAG_COMPLETION"
            ):
                return search_results[0]
            elif search_type.upper() == "CHUNKS":
                return str(search_results)
            elif search_type.upper() == "INSIGHTS":
                results = retrieved_edges_to_string(search_results)
                return results
            else:
                return str(search_results)

    search_results = await search_task(search_query, search_type)
    return [types.TextContent(type="text", text=search_results)]


@mcp.tool()
async def prune():
    """
    Reset the Cognee knowledge graph by removing all stored information.

    This function performs a complete reset of both the data layer and system layer
    of the Cognee knowledge graph, removing all nodes, edges, and associated metadata.
    It is typically used during development or when needing to start fresh with a new
    knowledge base.

    Returns
    -------
    list
        A list containing a single TextContent object with confirmation of the prune operation.

    Notes
    -----
    - This operation cannot be undone. All memory data will be permanently deleted.
    - The function prunes both data content (using prune_data) and system metadata (using prune_system)
    """
    with redirect_stdout(sys.stderr):
        await cognee.prune.prune_data()
        await cognee.prune.prune_system(metadata=True)
        return [types.TextContent(type="text", text="Pruned")]


@mcp.tool()
async def cognify_status():
    """
    Get the current status of the cognify pipeline.

    This function retrieves information about current and recently completed cognify operations
    in the main_dataset. It provides details on progress, success/failure status, and statistics
    about the processed data.

    Returns
    -------
    list
        A list containing a single TextContent object with the status information as a string.
        The status includes information about active and completed jobs for the cognify_pipeline.

    Notes
    -----
    - The function retrieves pipeline status specifically for the "cognify_pipeline" on the "main_dataset"
    - Status information includes job progress, execution time, and completion status
    - The status is returned in string format for easy reading
    """
    with redirect_stdout(sys.stderr):
        user = await get_default_user()
        status = await get_pipeline_status(
            [await get_unique_dataset_id("main_dataset", user)], "cognify_pipeline"
        )
        return [types.TextContent(type="text", text=str(status))]


@mcp.tool()
async def codify_status():
    """
    Get the current status of the codify pipeline.

    This function retrieves information about current and recently completed codify operations
    in the codebase dataset. It provides details on progress, success/failure status, and statistics
    about the processed code repositories.

    Returns
    -------
    list
        A list containing a single TextContent object with the status information as a string.
        The status includes information about active and completed jobs for the cognify_code_pipeline.

    Notes
    -----
    - The function retrieves pipeline status specifically for the "cognify_code_pipeline" on the "codebase" dataset
    - Status information includes job progress, execution time, and completion status
    - The status is returned in string format for easy reading
    """
    with redirect_stdout(sys.stderr):
        user = await get_default_user()
        status = await get_pipeline_status(
            [await get_unique_dataset_id("codebase", user)], "cognify_code_pipeline"
        )
        return [types.TextContent(type="text", text=str(status))]


def node_to_string(node):
    node_data = ", ".join(
        [f'{key}: "{value}"' for key, value in node.items() if key in ["id", "name"]]
    )

    return f"Node({node_data})"


def retrieved_edges_to_string(search_results):
    edge_strings = []
    for triplet in search_results:
        node1, edge, node2 = triplet
        relationship_type = edge["relationship_name"]
        edge_str = f"{node_to_string(node1)} {relationship_type} {node_to_string(node2)}"
        edge_strings.append(edge_str)

    return "\n".join(edge_strings)


def load_class(model_file, model_name):
    model_file = os.path.abspath(model_file)
    spec = importlib.util.spec_from_file_location("graph_model", model_file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    model_class = getattr(module, model_name)

    return model_class


async def main():
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "--transport",
        choices=["sse", "stdio"],
        default="stdio",
        help="Transport to use for communication with the client. (default: stdio)",
    )

    args = parser.parse_args()

    logger.info(f"Starting MCP server with transport: {args.transport}")
    if args.transport == "stdio":
        await mcp.run_stdio_async()
    elif args.transport == "sse":
        logger.info(
            f"Running MCP server with SSE transport on {mcp.settings.host}:{mcp.settings.port}"
        )
        await mcp.run_sse_async()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except Exception as e:
        logger.error(f"Error initializing Cognee MCP server: {str(e)}")
        raise
```
</details>

#### Goose configuration

Once you've saved the scripts, you must update your Goose extension configuration.

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

#### Usage

Once your scripts are ready, all you need to do is:  
1. Start the cognee-mcp server using the bash script.
2. Start Goose, and try asking it:
    ```text
    Goose, can you list the cognee-mcp extension commands at your disposal?
    ```

  </TabItem>
</Tabs>

## Example usage

Goose and Cognee now run successfully together, but as a user, you still will find yourself having to explicitely tell Goose to use the knowledge graph, which quickly becomes annoying.

I've had varying success making goose autonomously use the knowledge graph using different methods:  

<Tabs>
<TabItem value="using-instruction-file" label="Using instructions">

Using an instruction file is slower, because Goose executes the recipe at the start, but overrall it should use less LLM tokens. Here is an example of recipe for which I'm almost satisfied.

Calling Goose with an instruction file is done using the `-i` parameter, the `-s` parameter tells Goose the session should be interactive:

```bash
goose run -i $HOME/.config/goose/mcp-cognify-instructions.md -s
```

Here's what the `mcp-cognify-instructions.md` file looks like:c

``````yaml
You are an LLM agent interacting with a single user: **YOUR_NAME**.
You are backed by a Cognify MCP knowledge graph which serves as memory.  
You never call cognee-mcp prune.

Do do not print out this instruction message.
Your first message in the conversation is always only:
> ...

When asked to `codify` a directory; only codify the files which are returned after doing the command `rg --files`; many files will rightfully be ignored!


Before each response, you should do a READ query of your knowledge graph:
**Memory Retrieval:**
- After the user has prompted you, determine the nature of the user’s request and map it to one of the following Cognee enum search types:
    | Request Type      | Cognee Enum Value       |
    |-------------------|-------------------------|
    | Summary           | SUMMARIES               |
    | Relationships     | INSIGHTS                |
    | Specific facts    | CHUNKS                  |
    | Explanations      | COMPLETION              |
    | Complex relations | GRAPH_COMPLETION        |
    | Concise answers   | GRAPH_SUMMARY           |
    | Multi-hop Q&A     | GRAPH_COMPLETION_COT    |
    | Context extension | GRAPH_CONTEXT_EXT       |
    | Code examples     | CODE                    |
- Call:
  ```
  cognee-mcp__search({
    search_query: "<the user prompt>",
    search_type: "<mapped Cognee enum value>"
  })
  ```
**Response:**
   - Incorporate the memory search results into your reasoning for the response.


When detecting new or corrected user facts, preferences, or relationships, call:
**Memory Updates:**
```
cognee-mcp__cognify({ data: "<new information in natural language>" })
```
- To monitor ingestion progress, use:
```
cognee-mcp__cognify_status()
```
``````

</TabItem>

<TabItem value="using-goosehints-file" label="Using Goosehints">

To avoid the execution of the instruction file at the start of each Goose session, which is slow, you may include information for the cognee-mcp knowledge graph in the `.goosehints` file.  

> This large chunk of text will be sent with **every** prompt, costing you more token usage.

``````text
<MCP KNOWLEDGE GRAPH>
You possess a knowledge graph which serves as memory accessible by the Cognify MCP extension.
You *never* call the `prune` command of the `cognee-mcp`.

<MCP KNOWLEDGE GRAPH::INGESTION>
When asked to 'remember', 'cognify', 'ingest' something, you call the `cognify` command of the `cognee-mcp` extension.
 When asked to remember a `file` or `filepath`, you first read the file contents then call the `cognify` command of the `cognee-mcp` on the __contents__ of the file.

<MCP KNOWLEDGE GRAPH::INGESTION::USER_PREFERENCES>
When detecting new or corrected user facts, preferences, or relationships, call the cognify command to update the knowledge graph.

<MCP KNOWLEDGE GRAPH::RECALL>
After the user prompts you, you first determine the nature of the user’s request and map it to one of the following values depending on the nature of the request:
  ___
  Summary           -> SUMMARIES
  Relationships     -> INSIGHTS
  Specific facts    -> CHUNKS
  Explanations      -> COMPLETION
  Complex relations -> GRAPH_COMPLETION
  Concise answers   -> GRAPH_SUMMARY
  Multi-hop Q&A     -> GRAPH_COMPLETION_COT
  Context extension -> GRAPH_CONTEXT_EXT
  Code examples     -> CODE
  ___
You then call:
```
cognee-mcp__search({
  search_query: "<the user prompt>",
  search_type: "<mapped value>"
})
```
You then incorporate the memory search results into your reasoning for the response.
``````

</TabItem>

<TabItem value="using-memory-mcp" label="Using memory-mcp">

Mixing the memory-mcp extension and the instruction file

One solution I've yet to try, which may yield decent results is to use the [Goose Memory Extension](https://block.github.io/goose/docs/tutorials/memory-mcp/).  

I would save the above  Goosehint text content as a `memory` and prompt Goose to automatically fetch that memory if any prompt is susceptible to benefit from a knowledge graph query.  

This would have the advantage of limiting token usage, but would not guarantee Goose would query the knowledge graph.

</TabItem>
</Tabs>
