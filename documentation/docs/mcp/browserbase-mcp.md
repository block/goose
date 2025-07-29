---
title: Browserbase Extension
description: Add Browserbase MCP Server as a Goose Extension for Web Automation
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the Browserbase MCP Server as a Goose extension for browser automation, enabling programmatic control over navigation, page interactions, and content capture.

:::tip TLDR

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=@browserbasehq/mcp&id=browserbase&name=Browserbase&description=Automate%20web%20browsing%20and%20data%20extraction&env=BROWSERBASE_PROJECT_ID%3DBrowserbase%20Project%20ID&env=BROWSERBASE_API_KEY%3DBrowserbase%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  npx @browserbasehq/mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variables**
  ```
  BROWSERBASE_PROJECT_ID: <YOUR_PROJECT_ID>
  BROWSERBASE_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="browserbase"
    extensionName="Browserbase"
    description="Automate web browsing and data extraction"
    command="npx"
    args={["@browserbasehq/mcp"]}
    envVars={[
      { name: "BROWSERBASE_PROJECT_ID", label: "Browserbase Project ID" },
      { name: "BROWSERBASE_API_KEY", label: "Browserbase API Key" }
    ]}
    apiKeyLink="https://browserbase.io/dashboard"
    apiKeyLinkText="Get your Browserbase credentials"
  />
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
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

  3. Give your extension a name
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
    │  browserbase
    // highlight-end
    └ 
  ```

  4. Enter the command
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
    │  browserbase
    │
    // highlight-start
    ◆  What command should be run?
    │  npx @browserbasehq/mcp
    // highlight-end
    └ 
  ```  

  5. Enter the timeout (default 300s)
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
    │  browserbase
    │
    ◇  What command should be run?
    │  npx @browserbasehq/mcp
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    └ 
  ```  

  6. Add a description (optional)
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
    │  browserbase
    │
    ◇  What command should be run?
    │  npx @browserbasehq/mcp
    │
    ◆  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◇  Would you like to add a description?
    │  No
    // highlight-end
    └ 
  ```

  7. Add environment variables
  ```sh
    ┌   goose-configure 
    │
    // highlight-start
    ◆  Would you like to add environment variables?
    │  Yes
    │
    ◇  Environment variable name:
    │  BROWSERBASE_PROJECT_ID
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  Yes
    │
    ◇  Environment variable name:
    │  BROWSERBASE_API_KEY
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  No
    // highlight-end
    └  Added browserbase extension
  ```

  </TabItem>
</Tabs>

## Example Usage

Let's use the Browserbase extension to gather information about trending MCP-related repositories on GitHub.

:::info LLM
Claude 3.5 Sonnet was used for this task.
:::

### Goose Prompt

```
1. Go to https://github.com/trending

2. Search for "MCP server" or scroll through and find repos with "MCP" or "Model Context Protocol" in the title or description

3. For the top 3 relevant repositories:
    • Click into the repo
    • Take a screenshot of the repo page
    • Extract:
        - Repo name
        - Description
        - Stars today
        - First paragraph of README 
    • Navigate back

- Close all tabs

- Output a Markdown report showing:
    • Repo name (link)
    • Screenshot
    • Description + Stars
    • README excerpt
```

### Goose Output

```
# MCP Repositories Report

## 1. [FastMCP](https://github.com/jlowin/fastmcp)

![FastMCP Repository Screenshot]

**Description**: The fast, Pythonic way to build MCP servers and clients.  
**Stars Today**: 10.2k stars

**README Excerpt**: The Model Context Protocol (MCP) is a new, standardized way to provide context and tools to your LLMs, and FastMCP makes building MCP servers and clients simple and intuitive. Create tools, expose resources, define prompts, and connect components with clean, Pythonic code.

## 2. [GitHub MCP Server](https://github.com/github/github-mcp-server)

![GitHub MCP Server Screenshot]

**Description**: GitHub's official MCP Server that provides seamless integration with GitHub APIs, enabling advanced automation and interaction capabilities for developers and tools.  
**Stars Today**: 13.9k stars

**README Excerpt**: The GitHub MCP Server is a Model Context Protocol (MCP) server that provides seamless integration with GitHub APIs, enabling advanced automation and interaction capabilities for developers and tools.

## 3. [Playwright MCP](https://github.com/microsoft/playwright-mcp)

![Playwright MCP Screenshot]

**Description**: A Model Context Protocol (MCP) server that provides browser automation capabilities using Playwright.  
**Stars Today**: 10.2k stars

**README Excerpt**: A Model Context Protocol (MCP) server that provides browser automation capabilities using Playwright. This server enables LLMs to interact with web pages through structured accessibility snapshots, bypassing the need for screenshots or visually-tuned models.
```