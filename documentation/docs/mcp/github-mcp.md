---
title: GitHub Extension
description: Add GitHub MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/TbmQDv3SQOE" />

This tutorial covers how to add the [GitHub MCP Server](https://github.com/github/github-mcp-server) as a Goose extension to enable file operations, repository management, search functionality, and more.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  Use `Add custom extension` in Settings → Extensions to add a `Streamable HTTP` extension type with:
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  Use `goose configure` to add a `Remote Extension (Streaming HTTP)` extension type with:
  </TabItem>
</Tabs>

  **Endpoint URL**
  ```
  https://api.githubcopilot.com/mcp/
  ```
  **Custom Request Header**
  ```
  Authorization: Bearer <YOUR_GITHUB_PERSONAL_ACCESS_TOKEN>
  ```
:::

## Configuration
These steps configure the Remote MCP Server. For other deployment options, see the [official GitHub MCP Server documentation](https://github.com/github/github-mcp-server).

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    1. Obtain a [GitHub Personal Access Token](https://github.com/settings/personal-access-tokens)
    2. Click the gear icon `⚙️` in the top right corner
    3. Click `Advanced settings`
    4. Under `Extensions`, click `Add custom extension`
    5. On the `Add custom extension` modal, enter the following:
       - **Extension Name**: GitHub
       - **Type**: Streamable HTTP
       - **Endpoint**: `https://api.githubcopilot.com/mcp/`
       - **Request Headers**: 
         - **Header name**: `Authorization`
         - **Value**: `Bearer <YOUR_GITHUB_PERSONAL_ACCESS_TOKEN>` 
    6. Click `+ Add` to save the header
    7. Click `Add Extension` to save the extension

  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    1. Run the `configure` command:
    ```sh
    goose configure
    ```

    2. Choose to add a `Remote Extension (Streaming HTTP)`
    ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◆  What type of extension would you like to add?
        │  ○ Built-in Extension 
        │  ○ Command-line Extension (Run a local command or script)
        │  ○ Remote Extension (SSE) 
        // highlight-start    
        │  ● Remote Extension (Streaming HTTP) 
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
        │  Remote Extension (Streaming HTTP) 
        │
        // highlight-start
        ◆  What would you like to call this extension?
        │  github
        // highlight-end
        └ 
    ```

    4. Enter the Streaming HTTP endpoint URI
    ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (Streaming HTTP) 
        │
        ◇  What would you like to call this extension?
        │  github
        │
        // highlight-start
        ◆  What is the Streaming HTTP endpoint URI?
        │  https://api.githubcopilot.com/mcp/
        // highlight-end
        └ 
    ``` 

    5. Set the timeout
    ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (Streaming HTTP) 
        │
        ◇  What would you like to call this extension?
        │  github
        │
        ◇  What is the Streaming HTTP endpoint URI?
        │  https://api.githubcopilot.com/mcp/
        │
        // highlight-start
        ◆  Please set the timeout for this tool (in secs):
        │  300
        // highlight-end
        └ 
    ``` 

    6. Choose whether to add a description
    ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (Streaming HTTP) 
        │
        ◇  What would you like to call this extension?
        │  github
        │
        ◇  What is the Streaming HTTP endpoint URI?
        │  https://api.githubcopilot.com/mcp/
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        // highlight-start
        ◆  Would you like to add a description?
        │  No
        // highlight-end
        └ 
    ```

    7. Add a custom header for authentication
    ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (Streaming HTTP) 
        │
        ◇  What would you like to call this extension?
        │  github
        │
        ◇  What is the Streaming HTTP endpoint URI?
        │  https://api.githubcopilot.com/mcp/
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        ◇  Would you like to add a description?
        │  No
        │
        // highlight-start
        ◆  Would you like to add custom headers?
        │  Yes
        // highlight-end
        └ 
    ```

    8. Enter the Authorization header
    ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (Streaming HTTP) 
        │
        ◇  What would you like to call this extension?
        │  github
        │
        ◇  What is the Streaming HTTP endpoint URI?
        │  https://api.githubcopilot.com/mcp/
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        ◇  Would you like to add a description?
        │  No
        │
        ◇  Would you like to add custom headers?
        │  Yes
        │
        // highlight-start
        ◆  Header name:
        │  Authorization
        // highlight-end
        └ 
    ```

    9. Enter your [GitHub Personal Access Token](https://github.com/settings/personal-access-tokens)
    ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (Streaming HTTP) 
        │
        ◇  What would you like to call this extension?
        │  github
        │
        ◇  What is the Streaming HTTP endpoint URI?
        │  https://api.githubcopilot.com/mcp/
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        ◇  Would you like to add a description?
        │  No
        │
        ◇  Would you like to add custom headers?
        │  Yes
        │
        ◇  Header name:
        │  Authorization
        │
        // highlight-start
        ◆  Header value:
        │  Bearer ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
        // highlight-end
        └ 
    ```

    10. Choose not to add another header
    ```sh
        ┌   goose-configure 
        │
        ◇  What would you like to configure?
        │  Add Extension 
        │
        ◇  What type of extension would you like to add?
        │  Remote Extension (Streaming HTTP) 
        │
        ◇  What would you like to call this extension?
        │  github
        │
        ◇  What is the Streaming HTTP endpoint URI?
        │  https://api.githubcopilot.com/mcp/
        │
        ◇  Please set the timeout for this tool (in secs):
        │  100
        │
        ◇  Would you like to add a description?
        │  No
        │
        ◇  Would you like to add custom headers?
        │  Yes
        │
        ◇  Header name:
        │  Authorization
        │
        ◇  Header value:
        │  Bearer ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
        │
        // highlight-start
        ◆  Add another header?
        │  No 
        │
        // highlight-end
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