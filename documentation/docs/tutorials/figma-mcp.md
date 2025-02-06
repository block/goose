---
title: Figma Extension
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

The Figma MCP server allows you to interact with your Figma files and designs programmatically. When creating your access token, make sure you give it the appropriate access and scope:

1. Read access for files
2. Write access for Comments
3. And any other scopes of your choice.

### Goose Prompt
```
( O)> Convert my figma design with file ID KvR8Wpe6ITaCD3et3H7l6k into a webpage. Export and include any necessary assets and create the HTML, CSS, and JavaScript files needed.
```

### Goose Output

```                                                                                                                     
I'll help you convert your Figma design into a webpage. I'll break this down into steps:

1. First, let's get information about the Figma file to understand its structure
2. Then we'll get any components and styles
3. Finally, we'll create the necessary web files

Let's start by examining the file:

─── get_file_info | figma ──────────────────────────
depth: 2
file_key: KvR8Wpe6ITaCD3et3H7l6k



Let's also check for any components and styles:

─── get_components | figma ──────────────────────────
file_key: KvR8Wpe6ITaCD3et3H7l6k




─── get_styles | figma ──────────────────────────
file_key: KvR8Wpe6ITaCD3et3H7l6k



I see that this is an AI Landing Page design. Let's create the basic file structure first:

─── shell | developer ──────────────────────────
command: mkdir -p assets css js



Now let's create the HTML file:

─── text_editor | developer ──────────────────────────
path: ~/Documents/www/learn-goose/my-figma-project/test/index.html
command: write
file_text: ...


### .../my-figma-project/test/index.html

... HTML content hidden here ...

Let's create the CSS file with the basic structure based on what we can see from the Figma file:

─── text_editor | developer ──────────────────────────
path: ~/D/w/l/m/t/css/styles.css
command: write
file_text: ...


### .../my-figma-project/test/css/styles.css

... CSS content hidden here ...

Let's create a basic JavaScript file for any interactivity we might need:

─── text_editor | developer ──────────────────────────
path: ~/Documents/www/learn-goose/my-figma-project/test/js/main.js
command: write
file_text: ...


### .../my-figma-project/test/js/main.js

... JavaScript content hidden here ...

I've created the basic structure for your webpage based on the Figma design. To fully implement the design, we'll need to:

1. Get more detailed information about the specific elements within each frame
2. Export any images or assets from the Figma file
3. Implement the detailed HTML structure
4. Add specific styles for all components
5. Implement any interactions or animations

Would you like me to:
1. Get more detailed information about specific sections of the design?
2. Export specific assets from the Figma file?
3. Focus on implementing a particular section first?

Please let me know which aspect you'd like me to work on next, and I'll help you complete the implementation.
```