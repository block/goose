---
title: Using Figma Extension
description: Add Figma MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';


This tutorial covers how to add the [Figma MCP Server](https://github.com/hapins/figma-mcp) as a Goose extension to enable interaction with Figma files, designs, and components through the Figma API.


:::tip TLDR

**Command**
```sh
npx @hapins/figma-mcp
```

**Environment Variable**
```
FIGMA_ACCESS_TOKEN: <YOUR_TOKEN>
```
:::

## Configuration

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
    │  figma
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
    │  figma
    │
    // highlight-start
    ◆  What command should be run?
    │  npx @hapins/figma-mcp
    // highlight-end
    └ 
  ```  

  5. Obtain a [Figma Access Token](https://www.figma.com/developers/api#access-tokens) and paste it in.
  :::info
  You can generate an access token from your Figma account settings under the Personal access tokens section.
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
    │  figma
    │
    ◇  What command should be run?
    │  npx @hapins/figma-mcp
    // highlight-start
    ◆  Would you like to add environment variables?
    │  Yes 
    │
    ◇  Environment variable name:
    │  FIGMA_ACCESS_TOKEN
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  No 
    // highlight-end
    └  Added figma extension
  ```  

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. Click `...` in the upper right corner
  2. Click `Settings`
  3. Under `Extensions`, click the `Add` link
  4. On the `Add Extension Manually` modal, enter the following:
        * **Type**: `Standard IO`
        * **ID**: `figma-mcp` (_set this to whatever you want_)
        * **Name**: `Figma` (_set this to whatever you want_)
        * **Description**: `Figma MCP Server` (_set this to whatever you want_)
        * **Command**: `npx @hapins/figma-mcp`
        * **Environment Variables**
            * **Name**: `FIGMA_ACCESS_TOKEN`
            * **Value**: (_Obtain a [Figma Access Token](https://www.figma.com/developers/api#access-tokens) and paste it in._)
            * Click `Add` button
  5. Click `Add Extension` button
  </TabItem>
</Tabs>

## Example Usage

The Figma MCP server allows you to interact with your Figma files and designs programmatically. When creating your access token, make sure you give it the right scopes including:

1. Read access for files
2. Write access for Comments
3. And any other scopes of your choice.

### Goose Prompt
```
( O)> Get the comments on  my Figma design with ID "oDXDHd4fUhIdAS2JK4mIQo"
```

### Goose Output

```                                                                                                                     
I'll help you get the comments on your Figma file using the `figma__get_file_comments` function. The file key you provided is "oDXDHd4fUhIdAS2JK4mIQo".

─── get_file_comments | figma ──────────────────────────
file_key: oDXDHd4fUhIdAS2JK4mIQo


I've retrieved all the comments from your Figma file. The file has several comments from different team members discussing various aspects of the design, including:

1. Discussions about light/dark mode toggles
2. Comments about layout and design elements
3. Feedback on status banners and toasts
4. Questions about specific UI components
5. Comments about navigation and user interface elements

The comments are from various team members including John Doe, Tony Stark, Nick Fury, and Ho Yinsen. The comments are arranged chronologically with the most recent ones first, dating from January 23, 2025, back to January 15, 2025.

Would you like me to focus on any specific aspect of these comments or provide more details about particular discussions?
```