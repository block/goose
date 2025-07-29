---
title: Pieces for Developers Extension
description: Add Pieces for Developers MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/V8zp9m9__t4" />

This tutorial covers how to add the [Pieces for Developers MCP Server](https://docs.pieces.app/products/mcp/get-started?utm_source=goose&utm_medium=collab&utm_campaign=mcp) as a Goose extension to enable interaction with your Pieces Long-Term Memory.

## Configuration

1. Download [PiecesOS](https://pieces.app/)

2. Enable [Long-Term Memory Context](https://docs.pieces.app/products/quick-guides/ltm-context) in PiecesOS

3. Locate your MCP Server URL
   - In PiecesOS, navigate to Settings > Model Context Protocol (MCP)
   - Copy the server URL

:::tip
The default server URL is shown below. PiecesOS may use a different port if 39300 is already in use on your system:

```
http://localhost:39300/model_context_protocol/2024-11-05/sse
```
:::

### Add Pieces MCP Server

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="pieces"
    extensionName="Pieces for Developers"
    description="Provides access to your Pieces Long-Term Memory. You need to have Pieces installed to use this."
    url="http://localhost:39300/model_context_protocol/2024-11-05/sse"
  />
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  1. Run the `configure` command:

      ```sh
      goose configure
      ```

  2. Choose to add a `Remote Extension`

      ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension) 
        │
        ◆  What type of extension would you like to add?
        │  ○ Built-in Extension 
        │  ○ Command-line Extension (Run a local command or script)
        // highlight-start    
        │  ● Remote Extension (SSE)
        // highlight-end
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
        │  Remote Extension (SSE)
        │
        // highlight-start
        ◆  What would you like to call this extension?
        │  Pieces
        // highlight-end
        └ 
      ```

  4. Enter the SSE endpoint URI.

      :::info
      Use the server URL you copied from PiecesOS settings earlier.
      :::

      ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension) 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (SSE)
        │
        ◇  What would you like to call this extension?
        │  Pieces
        │
        // highlight-start
        ◆  What is the SSE endpoint URI?
        │  http://localhost:39300/model_context_protocol/2024-11-05/sse
        // highlight-end
        └ 
      ```  

  5. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s

      ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension) 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (SSE)
        │
        ◇  What would you like to call this extension?
        │  Pieces
        │
        ◇  What is the SSE endpoint URI?
        │  http://localhost:39300/model_context_protocol/2024-11-05/sse
        │
        // highlight-start
        ◆  Please set the timeout for this tool (in secs):
        │  300
        // highlight-end
        └ 
      ```

  6. Add a description if you want, otherwise to leave the description blank select No.

      ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension) 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (SSE)
        │
        ◇  What would you like to call this extension?
        │  Pieces
        │
        ◇  What is the SSE endpoint URI?
        │  http://localhost:39300/model_context_protocol/2024-11-05/sse
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        // highlight-start
        ◆  Would you like to add a description?
        │  ○ Yes  / ● No 
        // highlight-end
        │
        └ 
      ```

  7. No environment variables are needed, so select No.

      ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension (Connect to a new extension) 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (SSE)
        │
        ◇  What would you like to call this extension?
        │  Pieces
        │
        ◇  What is the SSE endpoint URI?
        │  http://localhost:39300/model_context_protocol/2024-11-05/sse
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        ◇  Would you like to add a description?
        │  No 
        │
        // highlight-start
        ◆  Would you like to add environment variables?
        │  ○ Yes  / ● No 
        // highlight-end
        └ 
      ```

  </TabItem>
</Tabs>

## Example Usage

Let's use the Pieces for Developers MCP server to generate a status update for a standup meeting based on your activities on the previous day.

### Goose Prompt

```
I need a status update for what I was using yesterday. Create a report with 5 bullet points based off the most important activities, and give each one a brief description.
```

### Goose Output

```
Here's a report summarizing your key activities from yesterday, based on the most important interactions and applications you were using:

1. Code Development with Goose:

    Activity: You worked on integrating SSE MCP servers into the Goose CLI, making enhancements to handle extensions via schema URLs.
    Description: This task involved modifying code and testing new integrations to support advanced functionalities. It also included interactions with the Goose community on Discord for feedback and troubleshooting.

2. Documentation Updates:

    Activity: You updated and managed documentation related to the Goose project and MCP integrations.
    Description: This involved preparing documentation for new features, including usage guidelines and technical details, aiding in the improvement of user experience and technical instructions.

3. Community Interaction on Discord:

  Activity: Engaged with the Block Open Source Community to discuss project developments and potential collaborations.
  Description: You actively participated in community discussions, exchanging insights on project enhancements, and exploring collaborative opportunities with other developers and contributors.

4. Managing and Testing Extensions:

    Activity: You focused on testing and managing various extensions through the Goose platform.
    Description: This included configuring and validating different extensions, ensuring their compatibility and functionality, and incorporating feedback from testing.

5. Content and Integration Planning:

    Activity: Planned integration workflows for MCP and documented their use cases.
    Description: You worked on strategies for implementing MCP integrations effectively, involving planning sessions to optimize future project deployments and align them with user requirements and project objectives.
    
These activities demonstrate a productive day with a focus on development, collaboration, and content management within your technical community.
```


:::tip
For more examples of prompts you can use with the Pieces for Developers MCP Server, see the [Pieces MCP prompting guide](https://docs.pieces.app/products/mcp/prompting?utm_source=goose&utm_medium=collab&utm_campaign=mcp).
:::