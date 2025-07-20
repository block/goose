---
title: Asana Extension
description: Add Asana MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

<!--<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/VIDEO_ID" />-->


This tutorial covers how to add the [Asana MCP Server](https://github.com/roychri/mcp-server-asana) as a Goose extension to enable task automation, project tracking, and team collaboration.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40roychri%2Fmcp-server-asana&id=asana&name=Asana&description=enable%20task%20automation%2C%20project%20tracking%2C%20and%20team%20collaboration&env=ASANA_ACCESS_TOKEN%3DAsana%20Access%20Token)
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  npx -y @roychri/mcp-server-asana
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  ASANA_ACCESS_TOKEN: <YOUR_TOKEN>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::


<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="asana"
    extensionName="Asana"
    description="enable task automation, project tracking, and team collaboration"
    command="npx"
    args={["-y", "@roychri/mcp-server-asana"]}
    envVars={[
      { name: "ASANA_ACCESS_TOKEN", label: "Asana Access Token" }
    ]}
    apiKeyLink="https://app.asana.com/0/my-apps"
    apiKeyLinkText="Asana Access Token"
  />
  :::info
  See [Asana's developer docs](https://developers.asana.com/docs/personal-access-token) if you need detailed instructions on creating an access token.
  :::
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
    │  Asana
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
    │  Asana
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y @roychri/mcp-server-asana
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
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Asana
    │
    ◇  What command should be run?
    │  npx -y @roychri/mcp-server-asana
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    │
    └ 
  ``` 

  6. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
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
    │  Asana
    │
    ◇  What command should be run?
    │  npx -y @roychri/mcp-server-asana
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
  
  7. Obtain a [Asana Access Token](https://app.asana.com/0/my-apps) and paste it in.
  :::info
  See [Asana's developer docs](https://developers.asana.com/docs/personal-access-token) if you need detailed instructions on creating an access token.
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
    │  Asana
    │
    ◇  What command should be run?
    │  npx -y @roychri/mcp-server-asana
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
    │  ASANA_ACCESS_TOKEN
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  No 
    // highlight-end
    └  Added Asana extension
  ```  

  </TabItem>
</Tabs>

## Example Usage

:::info LLM
OpenAI's GPT-4o was used for this task. There's an [open bug](https://github.com/block/goose/issues/1804) for Amazon Bedrock models.
:::

### Goose Prompt

> _Goose, I have one hour. Look through uncompleted tasks assigned to me in Asana and show me ones that you estimate will take an hour or less. Order them by deadline._


### Goose Output

Note that IDs have been changed to generic ones

:::note CLI

<details>
    <summary>Tool Calls</summary>

    ─── asana_search_tasks | abcdefgh ──────────────────────────

    assignee.any: me

    completed: false

    workspace: 1234567890123



    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000001


    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000002


    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000003


    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000004


    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000005


    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000006


    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000007


    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000008


    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000009


    ─── asana_get_task | abcdefgh ──────────────────────────

    opt_fields: due_on,estimated_hours

    task_id: 1100000000000010
</details>



Here are your uncompleted tasks in Asana estimated to take an hour or less, ordered by their deadline:

1. **Task:** Review team meeting notes
   - **Due Date:** 2025-03-21

2. **Task:** Draft quick status update
   - **Due Date:** 2025-03-21

3. **Task:** Proofread blog post
   - **Due Date:** 2025-03-21

4. **Task:** Send reminder to partners
   - **Due Date:** 2025-03-27

Please let me know if you'd like more information or help with any specific task!
:::