---
title: Dagger Container Use MCP
description: Integrate container workflows with Goose using the Dagger Container Use MCP
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

[Dagger's "Container Use" MCP](https://container-use.com/) is a powerful extension that brings seamless containerized workflows to your Goose environment. With this integration, you can:

- run code in standardized environments
- chain build steps using containers
- fetch & mutate files in containers
- automate development, CI, and DevOps workflows

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/your-video-id" />

## Configuration

:::info
You'll need [Node.js](https://nodejs.org/) and [Docker](https://www.docker.com/) installed on your system.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    <GooseDesktopInstaller
        extensionId="dagger-container-use"
        extensionName="Dagger Container Use MCP"
        extensionDescription="Run container automation with Dagger's container-use MCP server"
        command="npx"
        args={["-y", "mcp-remote", "https://container-use.com/mcp"]}
        cliCommand="npx -y mcp-remote https://container-use.com/mcp"
        timeout={300}
        note="Requires Node.js and Docker installed; see [container-use.com/quickstart](https://container-use.com/quickstart)."
    />
 </TabItem>
  <TabItem value="cli" label="Goose CLI">
      <CLIExtensionInstructions
        name="Dagger Container Use MCP"
        command="npx -y mcp-remote https://container-use.com/mcp"
        timeout={300}
      />
  </TabItem>
</Tabs>


## Example Usage

Here's a simple example of how to use the Dagger Container Use MCP to run a Node.js project with unit tests and coverage reporting:

#### Prompt

```
( O)> 
Using the Dagger Container Use MCP, do the following:
- Use node:20-alpine
- Install dependencies from package.json
- Run my unit tests
- Show me the resulting coverage report
```

#### Goose Output

```
```
