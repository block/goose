---
title: Google Drive Extension
description: Add Google Drive MCP Server as a goose Extension
unlisted: true
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/p9HGYbJk9wU" />

Server archived 

This tutorial covers how to add the [Google Drive MCP Server](https://www.pulsemcp.com/servers/modelcontextprotocol-gdrive) as a goose extension, allowing you to list, read, and search files in Google Drive.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40modelcontextprotocol%2Fserver-gdrive&id=google-drive&name=Google%20Drive&description=Google%20Drive%20integration&env=GDRIVE_CREDENTIALS_PATH%3DPath%20to%20Google%20Drive%20credentials&env=GDRIVE_OAUTH_PATH%3DPath%20to%20OAuth%20token)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  GDRIVE_OAUTH_PATH=$USER_HOME/.config/gcp-oauth.keys.json \ 
  GDRIVE_CREDENTIALS_PATH=$USER_HOME/.config/.gdrive-server-credentials.json \ 
  npx -y @modelcontextprotocol/server-gdrive auth \
  npx -y @modelcontextprotocol/server-gdrive 
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  GDRIVE_CREDENTIALS_PATH: $USER_HOME/.config/.gdrive-server-credentials.json
  GDRIVE_OAUTH_PATH: $USER_HOME/.config/gcp-oauth.keys.json  
  ```
:::

:::info
Note that you *must* use absolute paths in the environment variables. Make sure you replace `$USER_HOME` with your home directory.
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

To obtain your Google Drive server credentials and oauth keys, follow the steps below:

  1. Set up your Google Cloud Credentials, to enable API access:
        - Create Google Cloud Project
            - Go to [Google Cloud Console](https://console.cloud.google.com/projectcreate) and create a new project
            - You can leave `location` as `No organization`
        - Enable Google Drive API
            - In your project, go to the [API Product Library`](https://console.cloud.google.com/workspace-api/products)
            - Confirm you're in the right project by checking the top left corner
            - Search `Google Drive API` and enable it

  2. Configure OAuth Consent Screen 
        -  Go to the [OAuth Consent Screen](https://console.cloud.google.com/auth/overview/create)
        -  Enter required information, `project name` , `user support email` 
        -  Choose `Internal` for `Audience` and press `create`
        -  If you are unable to choose `Internal` select `External` and follow these additional steps:
            - Navigate to the [Audience](https://console.cloud.google.com/auth/audience) screen
            - Under `Test users` click `Add Users`

 3. Create OAuth Credential
        -  Go to [OAuth Clients](https://console.cloud.google.com/apis/credentials/oauthclient)
        -  Click `Create Client` 
        -  Choose **Application Type: Desktop App**
        -  Download the JSON key file
        -  Rename it to `gcp-oauth.keys.json`
        -  Move it to a secure location where the extension can access it:
            ```sh
            mv ~/Downloads/gcp-oauth.keys.json ~/.config/gcp-oauth.keys.json
            ```
  4. Connect Google Account
  
     To connect your Google account, run the following authentication command in your terminal:
          ```sh
          GDRIVE_OAUTH_PATH=$USER_HOME/.config/gcp-oauth.keys.json \ 
          GDRIVE_CREDENTIALS_PATH=$USER_HOME/.config/.gdrive-server-credentials.json \ 
          npx -y @modelcontextprotocol/server-gdrive auth
          ```
         :::info
         Replace `$USER_HOME` with your home directory.
         :::

      A browser window will open for authentication. Follow the prompts to connect your Google account and complete the OAuth process. At this stage, your environment variable `GDRIVE_CREDENTIALS_PATH` will be set with the saved credentials.

:::tip
You'll need to re-authenticate once a day when using the Google Drive extension. To re-authenticate, simply re-run the authentication command in your terminal.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="google-drive"
    extensionName="Google Drive"
    description="Google Drive integration"
    command="npx"
    args={["-y", "@modelcontextprotocol/server-gdrive"]}
    envVars={[
      { name: "GDRIVE_CREDENTIALS_PATH", label: "Path to Google Drive credentials" },
      { name: "GDRIVE_OAUTH_PATH", label: "Path to OAuth token" }
    ]}
  />

  :::info
  - For `GDRIVE_CREDENTIALS_PATH`, enter `$USER_HOME/.config/.gdrive-server-credentials.json`
  - For `GDRIVE_OAUTH_PATH`, enter `$USER_HOME/.config/gcp-oauth.keys.json`
  
  Replace `$USER_HOME` with your home directory. You must specify an absolute path for this extension to work.
  :::
  </TabItem>
  <TabItem value="cli" label="goose CLI">
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
    │  google drive
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
    │  google drive
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y @modelcontextprotocol/server-gdrive 
    // highlight-end
    └ 
  ```  

  5. Enter the number of seconds goose should wait for actions to complete before timing out. Default is 300s
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
    │  google drive
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-gdrive 
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
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
    │  google drive
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-gdrive 
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◇  Would you like to add a description?
    │  No
    // highlight-end
    └ 
  ```

  7. Add your environment variables 

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
    │  google drive
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-gdrive 
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
    │  GDRIVE_CREDENTIALS_PATH
    │
    ◇  Environment variable value:
    │  ~/.config/.gdrive-server-credentials.json
    │
    ◇  Add another environment variable?
    │  yes 
    |
    ◇  Environment variable name:
    │  GDRIVE_OAUTH_PATH
    │
    ◇  Environment variable value:
    │  ~/.config/gcp-oauth.keys.json
    │
    // highlight-end
    └  Added google drive extension
  ```  

  </TabItem>
</Tabs>

## Example Usage

In this example, I’ll use goose to quickly gather and summarize important information for my upcoming marketing budget review meeting in 30 minutes. I’ll ask goose to find relevant documents from my Google Drive, summarize Q1 performance, highlight critical decisions on marketing automation and video production, and identify any outstanding action items from our last meeting.

### goose Prompt
```
I have an important marketing budget review meeting in 30 minutes and I need your help getting prepared. I have several documents in my Google Drive from our previous meetings and planning sessions. Could you help me by:

1. Finding all relevant documents about our marketing budget and performance
2. Giving me a quick summary of our Q1 performance
3. Highlighting the key decisions we need to make about the marketing automation tool and video production
4. Identifying any outstanding action items from our last meeting

I need a quick but comprehensive overview so I can walk into this meeting well-prepared.
```

### goose Output

```
I'll help you prepare for your meeting. Let me first list available resources from Google Drive to make sure we access the correct files. 

[... list resources ...]

Great! I can see several relevant documents. Let me access them one by one to gather the information you need. 

[... reads resources ...]

Based on all these documents, here's your comprehensive overview for the meeting: 

Q1 Performance Summary:

  - Overall 5% under budget ($104,200 spent vs $107,000 budgeted)
  - Digital Marketing performing well with highest ROI:
      - Email marketing: 3.2 ROI
      - Google Ads: 2.4 ROI
      - Content Marketing: 2.1 ROI
  - Social media engagement up 23% from previous quarter
  - Content marketing blog posts generating 2x expected leads

Key Decisions Needed:

  1. Marketing Automation Tool (License Renewal Due):
    - Current tool (BasicAutomation Pro): $12,000/year
    - Options:
        - GrowthStack ($15,000/year) - recommended option, balanced features/price
        - MarketingHub Enterprise ($18,000/year) - premium option with AI features
        - Decision needed on whether to upgrade or maintain current solution
    - Implementation would take 6 weeks if changing providers

  2. Video Production Investment:
    - Three vendor options:
        - CreativeForce Media ($16,200) - most comprehensive, unlimited revisions
        - VisualCraft Studios ($14,500) - solid middle option
        - Digital Narrative Pro ($12,800) - basic package
    - CreativeForce Media is recommended despite higher cost due to comprehensive deliverables
    - Timeline ranges from 3-5 weeks depending on vendor choice

Outstanding Action Items from Last Meeting:

  1. Sarah needs to present ROI analysis for video content investment
  2. John needs to complete software license review
  3. Lisa to present new PPC strategy
  4. Mike has completed the video production agency quotes

Q2 Planning Considerations:

  - Proposed 12% budget increase for Q2 ($120,000 total)
  - Potential reallocation needed based on new tool and video production decisions
  - PPC strategy review pending

Would you like me to focus on any particular aspect of this overview in more detail?
```
