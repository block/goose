---
title: Google Drive Extension
description: Add Google Drive MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/p9HGYbJk9wU" />

This tutorial covers how to add the [Google Drive MCP Server](https://github.com/modelcontextprotocol/servers/tree/main/src/gdrive) as a Goose extension, allowing you to list, read, and search files in Google Drive.

:::tip TLDR

**Authentication Command**

In your terminal, run the following:

```sh
GDRIVE_OAUTH_PATH=/Users/<username>/.config/gcp-oauth.keys.json \ 
GDRIVE_CREDENTIALS_PATH=/Users/<username>/.config/.gdrive-server-credentials.json \ 
npx -y @modelcontextprotocol/server-gdrive auth
```
**Command**
```sh
npx -y @modelcontextprotocol/server-gdrive 
```

**Environment Variables**
```
GDRIVE_CREDENTIALS_PATH: ~/.config/.gdrive-server-credentials.json
```
```
GDRIVE_OAUTH_PATH: ~/.config/gcp-oauth.keys.json
```
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
          GDRIVE_OAUTH_PATH=/Users/<username>/.config/gcp-oauth.keys.json \ 
          GDRIVE_CREDENTIALS_PATH=/Users/<username>/.config/.gdrive-server-credentials.json \ 
          npx -y @modelcontextprotocol/server-gdrive auth
          ```
      A browser window will open for authentication. Follow the prompts to connect your Google account and complete the OAuth process. At this stage, your environment variable `GDRIVE_CREDENTIALS_PATH` will be set with the saved credentials.

:::tip
You'll need to re-authenticate once a day when using the Google Drive extension. To re-authenticate, simply re-run the authentication command in your terminal.
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
    │  google drive
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
    │  google drive
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y @modelcontextprotocol/server-gdrive 
    // highlight-end
    └ 
  ```  

  5. Add your environment variables 

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
    │  google drive
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-gdrive 
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
  <TabItem value="ui" label="Goose Desktop">
  1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40modelcontextprotocol%2Fserver-gdrive&id=google-drive&name=Google%20Drive&description=Google%20Drive%20integration&env=GDRIVE_CREDENTIALS_PATH%3DPath%20to%20Google%20Drive%20credentials&env=GDRIVE_OAUTH_PATH%3DPath%20to%20OAuth%20token)
  2. Press `Yes` to confirm the installation
  3. For `GDRIVE_CREDENTIALS_PATH`, enter the following:
  ```
  ~/.config/.gdrive-server-credentials.json
  ```
  4. For `GDRIVE_OAUTH_PATH`, enter the following:
  ```
  ~/.config/gcp-oauth.keys.json
  ```
  5. Click `Save Configuration`
  6. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
</Tabs>

## Example Usage

In this example, I’ll use Goose to quickly gather and summarize important information for my upcoming marketing budget review meeting in 30 minutes. I’ll ask Goose to find relevant documents from my Google Drive, summarize Q1 performance, highlight critical decisions on marketing automation and video production, and identify any outstanding action items from our last meeting.

### Goose Prompt
```
I have an important marketing budget review meeting in 30 minutes and I need your help getting prepared. I have several documents in my Google Drive from our previous meetings and planning sessions. Could you help me by:

1. Finding all relevant documents about our marketing budget and performance
2. Giving me a quick summary of our Q1 performance
3. Highlighting the key decisions we need to make about the marketing automation tool and video production
4. Identifying any outstanding action items from our last meeting

I need a quick but comprehensive overview so I can walk into this meeting well-prepared.
```

### Goose Output

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