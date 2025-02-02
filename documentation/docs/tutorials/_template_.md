---
title: {name} Extension
description: Add {name} MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';


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

  5. Obtain a [GitHub Personal Access Token](https://github.com/settings/personal-access-tokens) and paste it in.
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
    │  {name}}
    │
    ◇  What command should be run?
    │  {command}}
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
  1. Click `...` in the upper right corner
  2. Click `Settings`
  3. On `Extensions` section, click the `Add` link
  4. On the `Add Extension Manually` modal, enter the following:
        * **Type**: `Standard IO`
        * **ID**: `{name}-mcp` (_set this to whatever you want_)
        * **Name**: `{name}` (_set this to whatever you want_)
        * **Description**: `{name} MCP Server` (_set this to whatever you want_)
        * **Command**: `{command}`
        * **Environment Variables**
            * **Name**: `{env_var}`
            * **Value**: (_Obtain a [{env_var}](/) and paste it in._)
            * Click `Add` button
  5. Click `Add Extension` button
  </TabItem>
</Tabs>

## Example Usage

{describe any environment setup, access controls, and what you want to accomplish.}

### Goose Prompt
```
{exact prompt}.
```

### Goose Output

```
{exact output}
```