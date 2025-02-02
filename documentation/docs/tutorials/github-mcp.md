---
title: GitHub Extension
description: Add GitHub MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';


This tutorial covers how to add the [GitHub MCP Server](https://github.com/modelcontextprotocol/servers/tree/main/src/github) as a Goose extension to enable file operations, repository management, search functionality, and more.


:::tip TLDR

**Command**
```sh
npx -y @modelcontextprotocol/server-github
```

**Environment Variable**
```
GITHUB_PERSONAL_ACCESS_TOKEN: <YOUR_TOKEN>
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
    │  github
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
    │  github
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y @modelcontextprotocol/server-github
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
    │  github
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-github
    // highlight-start
    ◆  Would you like to add environment variables?
    │  Yes 
    │
    ◇  Environment variable name:
    │  GITHUB_PERSONAL_ACCESS_TOKEN
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
  3. Under `Extensions`, click the `Add` link
  4. On the `Add Extension Manually` modal, enter the following:
        * **Type**: `Standard IO`
        * **ID**: `gh-mcp` (_set this to whatever you want_)
        * **Name**: `github` (_set this to whatever you want_)
        * **Description**: `GitHub MCP Server` (_set this to whatever you want_)
        * **Command**: `npx -y @modelcontextprotocol/server-github`
        * **Environment Variables**
            * **Name**: `GITHUB_PERSONAL_ACCESS_TOKEN`
            * **Value**: (_Obtain a [GitHub Personal Access Token](https://github.com/settings/personal-access-tokens) and paste it in._)
              :::info
              When creating your access token, you can specify the repositories and granular permissions you'd like Goose to have access to.
              :::
            * Click `Add` button
  5. Click `Add Extension` button
  </TabItem>
</Tabs>

## Example Usage

When creating my [GitHub Personal Access Token](https://github.com/settings/personal-access-tokens), I gave it the following access:
* Repository: [goose-demo](https://github.com/angiejones/goose-demo)
* Read access to metadata
* Read and Write access to code, commit status, issues, and pull requests

### Goose Prompt
```
create a new branch called hello-world in my angiejones/goose-demo repository. Update the README.md file to say "this was written by goose" and commit it. Open a pull request with your changes.
```

### Goose Output

```
I'll help you create a new branch, update the README, and open a pull request. Let me break this down into steps:

1. First, let's create the new branch called "hello-world" in your repository
2. Then update the README.md file
3. Finally, create a pull request

Let me execute these steps:

1. Creating the new branch:

─── create_branch | github ──────────────────────────
branch: hello-world
owner: angiejones
repo: goose-demo

2. Now, updating the README.md file in the new branch:

─── create_or_update_file | github ──────────────────────────
branch: hello-world
content: this was written by goose
message: Update README.md with Goose message
owner: angiejones
path: README.md
repo: goose-demo



3. Finally, creating the pull request:

─── create_pull_request | github ──────────────────────────
base: main
body: ...
head: hello-world
owner: angiejones
repo: goose-demo
title: Update README.md



Great! I've completed all the requested tasks:
1. Created a new branch called "hello-world"
2. Updated the README.md file with the text "this was written by goose"
3. Created a pull request to merge these changes into the main branch

You can view the pull request at: https://github.com/angiejones/goose-demo/pull/1

The pull request is now ready for your review. Would you like me to do anything else with it?
```