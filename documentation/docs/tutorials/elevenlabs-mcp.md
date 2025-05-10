---
title: ElevenLabs Extension
description: Add ElevenLabs MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/1Z8XtjQ9El0" />


This tutorial covers how to add the [ElevenLabs MCP Server](https://github.com/yamadashy/repomix) as a Goose extension to enable AI-powered voice generation, voice cloning, audio editing, and speech-to-text transcription.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=elevenlabs-mcp&id=elevenlabs&name=ElevenLabs&description=ElevenLabs%20voice%20synthesis%20server&env=ELEVENLABS_API_KEY)
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  uvx elevenlabs-mcp
  ```
  </TabItem>
</Tabs>

  **Environment Variable**
  ```
  ELEVENLABS_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  1. [Launch the installer](goose://extension?cmd=uvx&arg=elevenlabs-mcp&id=elevenlabs&name=ElevenLabs&description=ElevenLabs%20voice%20synthesis%20server&env=ELEVENLABS_API_KEY)
  2. Press `Yes` to confirm the installation
  3. Click `Save Configuration`
  4. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
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
    │  ○ Remote Extension 
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
    │  elevenlabs
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
    │  elevenlabs
    │
    // highlight-start
    ◆  What command should be run?
    │  uvx elevenlabs-mcp
    // highlight-end
    └ 
  ```  

  5. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
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
    │  elevenlabs
    │
    ◇  What command should be run?
    │  uvx elevenlabs-mcp
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
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  elevenlabs
    │
    ◇  What command should be run?
    │  uvx elevenlabs-mcp
    │
    ◆  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◇  Would you like to add a description?
    │  No
    // highlight-end
    └ 
  ```

  7. Obtain an [ElevenLabs API Key](https://elevenlabs.io/app/settings/api-keys) and paste it in.

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
    │  elevenlabs
    │
    ◇  What command should be run?
    │  uvx elevenlabs-mcp
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │  
    ◇  Would you like to add a description?
    │  No   
    |   
    // highlight-start
    ◆  Would you like to add environment variables?
    │  Yes 
    │
    ◇  Environment variable name:
    │  ELEVENLABS_API_KEY
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  No 
    // highlight-end
    └  Added elevenlabs extension
  ```   

  </TabItem>
</Tabs>

## Example Usage

In this example, I’ll show you how to use Goose with the ElevenLabs Extension to create AI-generated voiceovers for a YouTube Short. Goose will take a sample script I provided, generate a narrated version using different AI voices, and seamlessly switch tones mid-script to match the content flow.

By connecting to the ElevenLabs MCP server, Goose can transform plain text into natural-sounding speech, offering multiple voice styles and character options — all without any manual recording or editing.

### Goose Prompt

> Hey Goose, create a script for me for my youtube short video, I want there to be two different voices. The first voice should cut me off and be a human narrator style and then switch to a cassual AI tone after I read the prompt. Here's an example of a YT short script I've done in the past: 

Wait… Within Seconds, Goose performed Security Audits Across Multiple Projects?! 🔥

Lets, plug & play to find out how 

Let’s provide Goose with the command it needs to connect to the Filesystem MCP server extension…

Now lets play 
propmt:  "Hey Goose, I need to perform a security audit across multiple projects. Let's check for…🔹 Hardcoded Credentials – API keys, passwords, and secrets left in the code.🔹 SQL Injection Risks – Unsafe queries that could expose data.🔹 Insecure Cryptographic Practices – Weak encryption methods that put data at risk.AND🔹 Exposed Config Files – Sensitive information that shouldn't be public.🔹 Outdated Dependencies – Security vulnerabilities in third-party libraries."

Go Goose, go Goose!

✅ Goose scanned the entire codebase across 3 different projects, identified security risks, generated a detailed report with fixes and provided me with step by step instructions on how I can test and verify these code fixes! 

If that’s not amazing idk what is …

🚀 to get started visit block.github.io/goose_


### Goose Output

:::note Desktop

I'll create your YouTube script for you using the given script as reference. 

───────────── Text To Speech ─────────────

───────────── Text To Speech ─────────────

───────────── Text To Speech ─────────────

───────────── Text To Speech ─────────────

───────────── Text To Speech ─────────────

───────────── Play Audio  ────────────────

───────────── Play Audio  ────────────────

The script has been created and read aloud using the specified voices and style. The audio files have been saved to your desktop.

Press play and hear it for yourself! 🔊

<audio controls>
  <source src="/goose/audio/elevenlabs-mcp-demo.mp3" type="audio/mpeg" />
  Your browser does not support the audio element.
</audio>

:::