---
title: GitHub Extension
description: Add GitHub MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

<!-- YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/TbmQDv3SQOE" / -->

This tutorial covers how to add the [GitHub MCP Server](https://github.com/github/github-mcp-server) as a Goose extension to enable file operations, repository management, search functionality, and more.

:::tip TLDR
Clone the repo to your system and run the command below to install the GitHub MCP Server as a Goose extension. This will require a Go compiler v 1.23.7 or newer to be installed on your system.

```sh
git clone https://github.com/github/github-mcp-server
cd github-mcp-server/cmd/github-mcp-server
go build
```
This will create a binary called `github-mcp-server` in the current directory which you can move anywhere else on your system if you wish.


<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  [Launch the installer](goose://extension?cmd=/path/to/github-mcp-server-binary&arg=stdio&id=github&name=GitHub&description=GitHub%20API&env=GITHUB_PERSONAL_ACCESS_TOKEN%3DGitHub%20Personal%20Access%20Token&env=GITHUB_TOOLSETS%3Dtool%20scopes)
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  /path/to/github-mcp-server stdio
  ```
  </TabItem>
</Tabs>

  **Environment Variable**
  ```
  GITHUB_PERSONAL_ACCESS_TOKEN: <YOUR_TOKEN>
  GITHUB_TOOLSETS: <tool scopes>
  ```

  Tool scopes can be found in [their documentation](https://github.com/github/github-mcp-server?tab=readme-ov-file#tool-configuration).
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  1. [Launch the installer](goose://extension?cmd=/path/to/github-mcp-server-binary&arg=stdio&id=github&name=GitHub&description=GitHub%20API&env=GITHUB_PERSONAL_ACCESS_TOKEN%3DGitHub%20Personal%20Access%20Token&env=GITHUB_TOOLSETS%3Dtool%20scopes)
  2. Press `Yes` to confirm the installation
  3. Obtain a [GitHub Personal Access Token](https://github.com/settings/personal-access-tokens) and paste it in
  4. Set the toolset scopes you want to use. You can find the available scopes in the [GitHub MCP Server documentation](https://github.com/github/github-mcp-server?tab=readme-ov-file#tool-configuration)
  5. Click `Save Configuration`
  6. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  1. Choose to add a `Command-line Extension`
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
    │  ○ Remote Extension 
    └ 
  ```

  1. Give your extension a name
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
    │  github
    // highlight-end
    └ 
  ```

  1. Enter the command
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
    │  github
    │
    // highlight-start
    ◆  What command should be run?
    │  /path/to/github-mcp-server-binary stdio
    // highlight-end
    └ 
  ```  

  1. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
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
    │  github
    │
    ◇  What command should be run?
    │  /path/to/github-mcp-server-binary stdio
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    │
    └ 
  ``` 

  1. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
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
    │  github
    │
    ◇  What command should be run?
    │  /path/to/github-mcp-server-binary stdio
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

  1. Obtain a [GitHub Personal Access Token](https://github.com/settings/personal-access-tokens) and paste it in.
  :::info
  When creating your access token, you can specify the repositories and granular permissions you'd like Goose to have access to.
  :::
  You will also need to set GITHUB_TOOLSETS to the tool scopes you want to use. You can find the available scopes in the [GitHub MCP Server documentation](https://github.com/github/github-mcp-server?tab=readme-ov-file#tool-configuration)

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
    │  github
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-github
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
    │  GITHUB_PERSONAL_ACCESS_TOKEN
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  Yes
    │
    ◇  Environment variable name:
    │  GITHUB_GITHUB_TOOLSETS
    │
    ◇  Environment variable value:
    │  repos,issues,pull_requests
    │
    ◇  Add another environment variable?
    │  No
    // highlight-end
    │
    └  Added github extension
  ```  

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