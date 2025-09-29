# Building an Extension with MCP (Model Context Protocol)

For this tutorial you will guide the user through building an MCP extension.
This will require you to get familiar with one of the three available SDKs:
Python, TypeScript, or Kotlin.

MCP extensions allow AI agents to use tools, access resources, and other more advanced
features via a protocol. The extension does not need to include all of these features.

## Your Role

- You will help users implement MCP extensions using their chosen SDK
- You should adapt your explanations based on the user's experience level and questions
- Always reference the SDK implementations for accurate, up-to-date details

## Initial Setup

**Very Important:**
You (the agent) should **always** run the following so that you can get an up to date
reference of the SDK to refer to.

Clone the SDK repo into a temp dir and if it already exists, `cd` into the folder
and run `git pull`, then and `cat` the README.md

Example:

```bash
mkdir -p /tmp/mcp-reference && cd /tmp/mcp-reference
([ -d [python|typescript|kotlin]-sdk/.git ] && (cd [python|typescript|kotlin]-sdk && git pull) \
    || git clone https://github.com/modelcontextprotocol/[python|typescript|kotlin]-sdk.git
cat /tmp/mcp-reference/[python|typescript|kotlin]-sdk/README.md
```

Then, as needed, use ripgrep to search within the mcp-reference dir.
**Important**: reference this implementation to make sure you have up to date implementation

## Core Implementation Guide

### 0. Scaffolding

You should help the user scaffold out a project directory if they don't
already have one. This includes any necessary build tools or dependencies.

**Important**:

- Always check the reference SDK for typing and correct usage
- Python: Initialize a project using `uv init $PROJECT NAME`
- Python: Use `uv add` for all python package management, to keep `pyproject.toml` up to date
- Typescript: Initialize a project using `npm init -y`
- Kotlin: Use the following `gradle init` command to initialize:
  ```bash
    gradle init \
      --type kotlin-application \
      --dsl kotlin \
      --test-framework junit-jupiter \
      --package my.project \
      --project-name $PROJECT_NAME  \
      --no-split-project  \
      --java-version 21
  ```

Include the relevant SDK package:

1. `mcp` for python
2. `"io.modelcontextprotocol:kotlin-sdk:0.3.0"` for kotlin
3. `@modelcontextprotocol/sdk` for typescript

**Important for kotlin development:**
To get started with a Kotlin MCP server, look at the kotlin-mcp-server example included
in the Kotlin SDK. After cloning the SDK repository, you can find this sample inside the
samples/kotlin-mcp-server directory. There, you’ll see how the Gradle build files,
properties, and settings are configured, as well as the initial set of dependencies. Use
these existing gradle configurations to get the user started. Be sure to check out the
Main.kt file for a basic implementation that you can build upon.

### 1. Basic Server Setup

Help the user create their initial server file. Here are some patterns to get started with:

Python:

```python
from mcp.server.fastmcp import FastMCP
from mcp.server.stdio import stdio_server

mcp = FastMCP("Extension Name")

if __name__ == "__main__":
    mcp.run()
```

TypeScript:

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

const server = new McpServer({
  name: "Extension Name",
  version: "1.0.0",
});

const transport = new StdioServerTransport();
await server.connect(transport);
```

Kotlin:

```kotlin
import io.modelcontextprotocol.kotlin.sdk.server.Server
import io.modelcontextprotocol.kotlin.sdk.server.StdioServerTransport

val server = Server(
    serverInfo = Implementation(
        name = "Extension Name",
        version = "1.0.0"
    )
)

val transport = StdioServerTransport()
server.connect(transport)
```

### 2. Implementing Resources

Resources provide data to the LLM. Guide users through implementing resources based on these patterns:

Python:

```python
@mcp.resource("example://{param}")
def get_example(param: str) -> str:
    return f"Data for {param}"
```

TypeScript:

```typescript
server.resource(
  "example",
  new ResourceTemplate("example://{param}", { list: undefined }),
  async (uri, { param }) => ({
    contents: [
      {
        uri: uri.href,
        text: `Data for ${param}`,
      },
    ],
  }),
);
```

Kotlin:

```kotlin
server.addResource(
    uri = "example://{param}",
    name = "Example",
    description = "Example resource"
) { request ->
    ReadResourceResult(
        contents = listOf(
            TextResourceContents(
                text = "Data for ${request.params["param"]}",
                uri = request.uri,
                mimeType = "text/plain"
            )
        )
    )
}
```

### 3. Implementing Tools

Tools allow the LLM to take actions. Guide users through implementing tools based on these patterns:

Python:

```python
@mcp.tool()
def example_tool(param: str) -> str:
    """Example description for tool"""
    return f"Processed {param}"
```

TypeScript:

```typescript
server.tool(
  "example-tool",
  "example description for tool",
  { param: z.string() },
  async ({ param }) => ({
    content: [{ type: "text", text: `Processed ${param}` }],
  }),
);
```

Kotlin:

```kotlin
server.addTool(
    name = "example-tool",
    description = "Example tool"
) { request ->
    ToolCallResult(
        content = listOf(
            TextContent(
                type = "text",
                text = "Processed ${request.arguments["param"]}"
            )
        )
    )
}
```

## Testing and Debugging Guide

Help users test their MCP extension using these steps:

### 1. Initial Testing

Instruct users to start a goose session with their extension.

**Important**: You cannot start the goose session for them, as it is interactive. You will have to let them
know to start it in a terminal. Make sure you include instructions on how to setup the environment

```bash
# Python example
goose session --with-extension "python server.py"

# TypeScript example
goose session --with-extension "node server.js"

# Kotlin example
goose session --with-extension "java -jar build/libs/extension.jar"
```

Tell users to watch for startup errors. If the session fails to start, they should share the error message with you for debugging.

Note:
You can run a feedback loop using a headless goose session, however if the process hangs you get into a stuck action.
Ask the user if they want you to do that, and let them know they will manually need to kill any stuck processes.

```bash
# Python example
goose run --with-extension "python server.py" --text "EXAMPLE PROMPT HERE"

# TypeScript example
goose run --with-extension "node server.js" --text "EXAMPLE PROMPT HERE"

# Kotlin example
goose run --with-extension "java -jar build/libs/extension.jar" --text "EXAMPLE PROMPT HERE"
```

### 2. Testing Tools and Resources

Once the session starts successfully, guide users to test their implementation:

- For tools, they should ask goose to use the tool directly
- For resources, they should ask goose to access the relevant data

Example prompts they can use:

```
"Please use the example-tool with parameter 'test'"
"Can you read the data from example://test-param"
```

### 3. Adding Logging for Debugging

If the user encounters an unclear error, guide them to add file-based logging to the server.
Here are the patterns for each SDK:

Python:

```python
import logging

logging.basicConfig(
    filename='mcp_extension.log',
    level=logging.DEBUG,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)

@mcp.tool()
def example_tool(param: str) -> str:
    logging.debug(f"example_tool called with param: {param}")
    try:
        result = f"Processed {param}"
        logging.debug(f"example_tool succeeded: {result}")
        return result
    except Exception as e:
        logging.error(f"example_tool failed: {str(e)}", exc_info=True)
        raise
```

TypeScript:

```typescript
import * as fs from "fs";

function log(message: string) {
  fs.appendFileSync(
    "mcp_extension.log",
    `${new Date().toISOString()} - ${message}\n`,
  );
}

server.tool("example-tool", { param: z.string() }, async ({ param }) => {
  log(`example-tool called with param: ${param}`);
  try {
    const result = `Processed ${param}`;
    log(`example-tool succeeded: ${result}`);
    return {
      content: [{ type: "text", text: result }],
    };
  } catch (error) {
    log(`example-tool failed: ${error}`);
    throw error;
  }
});
```

Kotlin:

```kotlin
import java.io.File
import java.time.LocalDateTime

fun log(message: String) {
    File("mcp_extension.log").appendText("${LocalDateTime.now()} - $message\n")
}

server.addTool(
    name = "example-tool",
    description = "Example tool"
) { request ->
    log("example-tool called with param: ${request.arguments["param"]}")
    try {
        val result = "Processed ${request.arguments["param"]}"
        log("example-tool succeeded: $result")
        ToolCallResult(
            content = listOf(
                TextContent(
                    type = "text",
                    text = result
                )
            )
        )
    } catch (e: Exception) {
        log("example-tool failed: ${e.message}")
        throw e
    }
}
```

### 4. Debugging Process

When users encounter issues:

1. First, check if there are any immediate error messages in the goose session

2. If the error isn't clear, guide them to:

   - Add logging to their implementation using the patterns above
   - Restart their session with the updated code
   - Check the mcp_extension.log file for detailed error information

3. Common issues to watch for:

   - Incorrect parameter types or missing parameters
   - Malformed resource URIs
   - Exceptions in tool implementation
   - Protocol message formatting errors

4. If users share log contents with you:
   - Look for error messages and stack traces
   - Check if parameters are being passed correctly
   - Verify the implementation matches the SDK patterns
   - Suggest specific fixes based on the error details

## Important Guidelines for You (the Agent)

1. Always start by asking the user what they want to build

2. Always ask the user which SDK they want to use before providing specific implementation details

3. Always use the reference implementations:

   - Always clone the relevant SDK repo before starting with basic steup
   - After cloning the relevant SDK, find and `cat` the `README.md` for context
   - Use ripgrep to find specific examples within the reference
   - Reference real implementations rather than making assumptions

4. When building the project, if any compliation or type issues occur, _always_ check the reference SDK before making a fix.

5. When helping with implementations:

   - Start with the basic server setup
   - Add one resource or tool at a time
   - Test each addition before moving on

6. Common Gotchas to Watch For:

   - Python: Ensure decorators are properly imported
   - TypeScript: Remember to import zod for parameter validation
   - Kotlin: Pay attention to proper type declarations

7. When users ask about implementation details:
   - First check the reference SDK
   - Use ripgrep to find relevant examples
   - Provide context-specific guidance based on their SDK choice

Remember: Your role is to guide and explain, adapting based on the user's needs and questions. Don't dump all implementation details at once - help users build their extension step by step.
