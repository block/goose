---
title: YouTube Transcript Extension
description: Add YouTube Transcript MCP Server as a Goose Extension for accessing YouTube video transcripts
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

This tutorial covers how to add the [YouTube Transcript MCP Server](https://github.com/jkawamoto/mcp-youtube-transcript) as a Goose extension to enable fetching and working with YouTube video transcripts.

:::tip TLDR

**Command**
```sh
npx @jkawamoto/mcp-youtube-transcript
```

No environment variables are required for this extension.
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
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
    │  youtube-transcript
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
    │  youtube-transcript
    │
    // highlight-start
    ◆  What command should be run?
    │  npx @jkawamoto/mcp-youtube-transcript
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
    │  youtube-transcript
    │
    ◇  What command should be run?
    │  npx @jkawamoto/mcp-youtube-transcript
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    │
    └ 
  ``` 
  
  6. No environment variables are required for this extension
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
    │  youtube-transcript
    │
    ◇  What command should be run?
    │  npx @jkawamoto/mcp-youtube-transcript
    │     
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │    
    // highlight-start
    ◆  Would you like to add environment variables?
    │  No
    // highlight-end
    └  Added youtube-transcript extension
  ```  

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40jkawamoto%2Fmcp-youtube-transcript&id=youtube-transcript&name=YouTube%20Transcript&description=Access%20YouTube%20video%20transcripts)
  2. Press `Yes` to confirm the installation
  3. Click `Save Configuration`
  4. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
</Tabs>

## Example Usage

The YouTube Transcript extension allows you to fetch and work with transcripts from YouTube videos. You'll need the video ID from the YouTube URL you want to get the transcript for.

### Goose Prompt

```
Get me the transcript for this YouTube video: https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

### Goose Output

:::note CLI
I'll help you get the transcript for that video. The video ID is "dQw4w9WgXcQ". Let me fetch the transcript for you.

Here's the transcript:

[Transcript content would appear here with timestamps and text]

I've retrieved the transcript for Rick Astley's "Never Gonna Give You Up" music video. The transcript shows the lyrics of the song, though there are some minor transcription errors due to the automated nature of the system. The transcript includes the iconic chorus and verses of this famous 1987 song, which has become one of the most well-known internet memes, often used for "rickrolling."

Would you like me to help you with anything else regarding the video or its transcript?
:::