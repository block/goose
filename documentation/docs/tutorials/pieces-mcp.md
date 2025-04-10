---
title: Pieces for Developers Extension
description: Add Pieces for Developers MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

This tutorial covers how to add the [Pieces for Developers MCP Server](https://pieces.app?utm_source=goose&utm_medium=collab&utm_campaign=mcp) as a Goose extension to enable interaction with your Pieces Long-Term Memory.

To learn more about MCP in Pieces, check out the Pieces [MCP documentation](https://docs.pieces.app/products/mcp/get-started?utm_source=goose&utm_medium=collab&utm_campaign=mcp).

:::tip TLDR

**URL**

```bash
http://localhost:39300/model_context_protocol/2024-11-05/sse
```

This can change if the port is already in use when you start PiecesOS. Check your URL in Pieces

:::

## Configuration

:::info
Note that you'll need [Pieces](https://pieces.app?utm_source=goose&utm_medium=collab&utm_campaign=mcp) installed to use this extension.
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
  1. Run the `configure` command:

      ```sh
      goose configure
      ```

  2. Choose to add a `Remote Extension`

      ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◆  What type of extension would you like to add?
        │  ○ Built-in Extension 
        │  ○ Command-line Extension (Run a local command or script)
        // highlight-start    
        │  ● Remote Extension 
        // highlight-end    
        └ 
      ```

  3. Give your extension a name
  
      ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension 
        │
        // highlight-start
        ◆  What would you like to call this extension?
        │  Pieces
        // highlight-end
        └ 
      ```

  4. Enter the SSE endpoint URI.

      You can get this from the Pieces settings. The default is `http://localhost:39300/model_context_protocol/2024-11-05/sse`, but the port may be different from 39300 to avoid clashing with other applications running locally.

      ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension 
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
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension 
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
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension 
        │
        ◇  What would you like to call this extension?
        │  Pieces
        │
        ◇  What is the SSE endpoint URI?
        │  http://localhost:39300/model_context_protocol/2024-11-05/sse
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        // highlight-start
        ◆  Would you like to add a description?
        │  ○ Yes  / ● No 
        // highlight-end
        └ 
      ```

  7. No environment variables are needed, so select No.

      ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension 
        │
        ◇  What would you like to call this extension?
        │  Pieces
        │
        ◇  What is the SSE endpoint URI?
        │  http://localhost:39300/model_context_protocol/2024-11-05/sse
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
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
  <TabItem value="ui" label="Goose Desktop">
  1. [Launch the installer](goose://extension?url=http%3A%2F%2Flocalhost%3A39300%2Fmodel_context_protocol%2F2024-11-05%2Fsse&id=pieces&name=Pieces%20for%20Developers&description=Provides%20access%20to%20your%20Pieces%20Long-Term%20Memory.%20You%20need%20to%20have%20Pieces%20installed%20to%20use%20this.)
  2. Press `Yes` to confirm the installation
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

## Learn more

To see some more examples of prompting capabilities unlocked by using the Pieces MCP server, check out the [Pieces MCP prompting guide](https://docs.pieces.app/products/mcp/prompting?utm_source=goose&utm_medium=collab&utm_campaign=mcp).
