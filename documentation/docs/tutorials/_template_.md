---
title: {name} Extension
description: Add {name} MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/VIDEO_ID" />


This tutorial covers how to add the [{name} MCP Server](/) as a Goose extension to enable file operations, repository management, search functionality, and more.


:::tip TLDR

**Command**
```sh
{command}
```

**Environment Variable**
```
{env_var}: <YOUR_TOKEN>
```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`.
:::

:::info
Note that you'll need [JBang](https://www.jbang.dev/download) installed on your system to run this command, as it uses `jbang`.
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◆  What type of extension would you like to add?
    │  ○ Built-in Extension 
    // highlight-start    
    │  ● Command-line Extension (Run a local command or script)
    // highlight-end    
    │  ○ Remote Extension 
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
    │  Command-line Extension 
    │
    // highlight-start
    ◆  What would you like to call this extension?
    │  {name}
    // highlight-end
    └ 
  ```

  4. Enter the command
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  {name}
    │
    // highlight-start
    ◆  What command should be run?
    │  {command}
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
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  {name}
    │
    ◇  What command should be run?
    │  {command}
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
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  {name}
    │
    ◇  What command should be run?
    │  {command}
    │
    ◆  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◇  Would you like to add a description?
    │  No
    // highlight-end
    │
    └ 
  ```

  7. Obtain a [GitHub Personal Access Token](https://github.com/settings/personal-access-tokens) and paste it in.
  :::info
  When creating your access token, you can specify the repositories and granular permissions you'd like Goose to have access to.
  :::

   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  {name}
    │
    ◇  What command should be run?
    │  {command}
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
    │  {env_var}
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  No 
    // highlight-end
    └  Added github extension
  ```  

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40hapins%2Ffigma-mcp&id=figma&name=Figma&description=Figma%20design%20tool%20integration&env=FIGMA_ACCESS_TOKEN%3DAccess%20token%20from%20Figma%20user%20settings)
  2. Press `Yes` to confirm the installation
  3. Obtain a [XYZ Access Token](/) and paste it in
  4. Click `Save Configuration`
  5. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
</Tabs>

## Example Usage

{describe any environment setup, access controls, and what you want to accomplish.}

### Goose Prompt

> _exact prompt_


### Goose Output

:::note CLI

{exact output}

:::