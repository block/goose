---
title: AgentQL Web Data Extension
description: Add AgentQL MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/VIDEO_ID" />

This tutorial covers how to add the [AgentQL MCP Server](https://github.com/tinyfish-io/agentql-mcp) as a Goose extension to extract and transform unstructured web content into structured data.

:::tip TLDR

**Command**
```sh
npx -y agentql-mcp
```

**Environment Variable**
```
AGENTQL_API_KEY: <YOUR_API_KEY>
```
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
    │  agentql
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
    │  agentql
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y agentql-mcp
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
    │  agentql
    │
    ◇  What command should be run?
    │  npx -y agentql-mcp
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    └ 
  ```  

  6. Obtain an AgentQL API Key and paste it in.
  :::info
  You can get your API key by signing up at [agentql.dev](https://agentql.dev) and navigating to your account settings.
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
    │  agentql
    │
    ◇  What command should be run?
    │  npx -y agentql-mcp
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◆  Would you like to add environment variables?
    │  Yes 
    │
    ◇  Environment variable name:
    │  AGENTQL_API_KEY
    │
    ◇  Environment variable value:
    │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇  Add another environment variable?
    │  No 
    // highlight-end
    └  Added agentql extension
  ```  

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=agentql-mcp&id=agentql&name=AgentQL%20Web%20Data&description=Transform%20unstructured%20web%20content%20into%20structured%20data&env=AGENTQL_API_KEY%3DAgentQL%20API%20Key)
  2. Press `Yes` to confirm the installation
  3. Obtain an [AGENTQL_API_KEY](https://agentql.dev) and paste it in
  4. Click `Save Configuration`
  5. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
</Tabs>

## Example Usage

Let's use the AgentQL extension to gather data and structure data tech conferences to help plan speaking engagements. 

:::info LLM
Anthropic's Claude 3.5 Sonnet was used for this task.
:::

### Goose Prompt

```
I'm a tech conference speaker planning my 2025-2026 submissions. Extract for major community conferences from 2022-2025:

- Conference name
- Conference dates
- CFP timeline 
To identify:
- Consistent monthly patterns
- Whether conferences stick to same months yearly
- If CFP windows are consistent year-to-year
- Any shifts in traditional timing

Structure results as JSON
```

### Goose Output

```json
{
  "conference_patterns": {
    "major_annual_conferences": {
      "Q1_conferences": {
        "typical_months": ["February", "March"],
        "notable_events": [
          "DeveloperWeek",
          "Microsoft MVP Summit"
        ],
        "cfp_patterns": {
          "typical_open": "4-5 months before",
          "typical_close": "2-3 months before",
          "consistency": "High"
        }
      },
      "Q2_conferences": {
        "typical_months": ["April", "May", "June"],
        "notable_events": [
          "Google I/O",
          "Microsoft Build",
          "PyCon US",
          "DevOpsDays (various locations)"
        ],
        "cfp_patterns": {
          "typical_open": "6 months before",
          "typical_close": "3-4 months before",
          "consistency": "Very High"
        }
      },
      "Q3_conferences": {
        "typical_months": ["July", "August", "September"],
        "notable_events": [
          "DEF CON",
          "Black Hat",
          "Strange Loop"
        ],
        "cfp_patterns": {
          "typical_open": "7-8 months before",
          "typical_close": "4-5 months before",
          "consistency": "Medium"
        }
      },
      "Q4_conferences": {
        "typical_months": ["October", "November", "December"],
        "notable_events": [
          "AWS re:Invent",
          "KubeCon + CloudNativeCon",
          "Microsoft Ignite"
        ],
        "cfp_patterns": {
          "typical_open": "6-7 months before",
          "typical_close": "4 months before",
          "consistency": "High"
        }
      }
    },
    "key_observations": {
      "timing_consistency": [
        "Most major conferences maintain consistent months year-over-year",
        "Q2 (April-June) has highest concentration of developer conferences",
        "Q4 has most cloud/infrastructure focused events"
      ],
      "cfp_trends": [
        "Larger conferences typically open CFPs 6-8 months before event",
        "Most CFPs close 3-4 months before conference date",
        "Regional/smaller events often have shorter CFP windows (2-3 months)",
        "Virtual/hybrid options remain common post-2023"
      ],
      "recent_shifts": [
        "More hybrid attendance options",
        "Longer CFP review periods",
        "Earlier announcements of conference dates",
        "More regional satellite events for major conferences"
      ]
    }
  }
}
```