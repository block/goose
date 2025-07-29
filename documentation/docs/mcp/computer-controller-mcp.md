---
title: Computer Controller Extension
description: Use Computer Controller MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/EuMzToNOQtw" />

The Computer Controller extension helps automate everyday computer tasks and web interactions such as searching the web, controlling system settings, processing data files, and controlling applications without needing to know how to code.

This tutorial covers enabling and using the Computer Controller MCP Server, which is a built-in Goose extension.

:::tip
Let Goose complete its tasks without interruption - avoid using your mouse or keyboard until it's done.
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Computer Controller"
    description="Automate everyday computer tasks and web interactions"
  />
  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Built-in Extension`
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◆  What type of extension would you like to add?
  // highlight-start    
  │  ● Built-in Extension (Use an extension that comes with Goose)
  // highlight-end  
  │  ○ Command-line Extension 
  │  ○ Remote Extension (SSE) 
  │  ○ Remote Extension (Streaming HTTP) 
  └  
  ```

  3. Select the `Computer Controller` extension
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Built-in Extension 
  │
  ◆  Which built-in extension would you like to enable?
  │  ○ Developer Tools 
  // highlight-start
  │  ● Computer Controller 
  // highlight-end
  │  ○ Memory 
  |  ○ JetBrains
  └  Enabled Computer Controller extension
  ```

  4. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Add Extension (Connect to a new extension) 
  │
  ◇  What type of extension would you like to add?
  │  Built-in Extension 
  │
  ◆  Which built-in extension would you like to enable?
  │  Computer Controller 
  │  
  // highlight-start
  ◆  Please set the timeout for this tool (in secs):
  │  300
  // highlight-end  
  └  Enabled Computer Controller extension
  ```  
  </TabItem>
</Tabs>

## Example Usage

In this example, I'll show you how Goose can multitask, handling everything from system controls and music playback to web research and data organization.

:::info LLM
Anthropic's Claude 3.5 Sonnet was used for this task.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
   1. Open a new session in Goose Desktop
  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  1. Open a terminal and start a new Goose session:

  ```sh
  goose session
  ```

  </TabItem>
</Tabs>

### Goose Prompt
```
Goose, I need to decompress while researching AI models for tomorrow's meeting. Can you create a relaxing environment and help me research?

1. Play classical music in Safari
2. Dim screen brightness (2 levels)
3. Research current flagship AI models (Anthropic, OpenAI, Google):
    * Verify availability
    * Collect: release date, features, pricing, context window
    * Use official sources only
4. Create "ai-models-comparison.csv" on desktop with collected data
5. Launch Numbers and open the CSV file
6. Reset brightness and close Safari
```

### Goose Output
```md
I'll help you create a relaxing research environment and gather information about AI models. I'll break this down into steps:

1. First, let's set up the relaxing environment with classical music and dimmed screen:

─── computer_control | computercontroller ──────────────────────────
script: ...



2. Now, let's research the AI models. I'll use web search and scraping to gather
 information from official sources:

─── web_search | computercontroller ──────────────────────────
query: Claude




─── web_search | computercontroller ──────────────────────────
query: GPT4




─── web_search | computercontroller ──────────────────────────
query: Gemini



3. Let's create the CSV file with the collected information:

─── automation_script | computercontroller ──────────────────────────
language: shell
script: ...



4. Now, let's open the CSV file in Numbers and reset the environment:

─── computer_control | computercontroller ──────────────────────────
script: ...

I've completed all the requested tasks:
1. ✅ Played classical music in Safari
2. ✅ Dimmed screen brightness
3. ✅ Researched current flagship AI models
4. ✅ Created the CSV file on desktop
5. ✅ Launched Numbers and opened the CSV file
6. ✅ Reset brightness and closed Safari

The CSV file has been created on your desktop with the following information for each model:
- Model name
- Company
- Release date
- Key features
- Context window size
- Pricing information

The file is now open in Numbers for your review. The environment has been reset to normal settings. Is there anything specific about any of the models you'd like me to elaborate on?
```

### Results
![Computer Controller MCP Output](../assets/guides/computer-controller-csv-result.png)