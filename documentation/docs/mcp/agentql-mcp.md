---
title: AgentQL Extension
description: Add AgentQL MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

<!-- <YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/VIDEO_ID" /> -->

This tutorial covers how to add the [AgentQL MCP Server](https://github.com/tinyfish-io/agentql-mcp) as a Goose extension to extract and transform unstructured web content into structured data.

:::tip TLDR

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=agentql-mcp&id=agentql&name=AgentQL&description=Transform%20unstructured%20web%20content%20into%20structured%20data&env=AGENTQL_API_KEY%3DAgentQL%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  npx -y agentql-mcp
  ```
  </TabItem>
</Tabs>
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
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="agentql"
    extensionName="AgentQL"
    description="Transform unstructured web content into structured data"
    command="npx"
    args={["-y", "agentql-mcp"]}
    envVars={[
      { name: "AGENTQL_API_KEY", label: "AgentQL API Key" }
    ]}
    apiKeyLink="https://dev.agentql.com/api-keys"
    apiKeyLinkText="AGENTQL_API_KEY"
  />
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
    │  agentql
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
    │  Add Extension (Connect to a new extension) 
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
    │  agentql
    │
    ◇  What command should be run?
    │  npx -y agentql-mcp
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

  7. Obtain an [AgentQL API Key](https://dev.agentql.com/api-keys) and paste it in.

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
    │  agentql
    │
    ◇  What command should be run?
    │  npx -y agentql-mcp
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
</Tabs>

## Example Usage

Let's use the AgentQL extension to gather and structure tech conference data to help plan speaking engagements. 

:::info LLM
Anthropic's Claude 3.5 Sonnet was used for this task.
:::

### Goose Prompt

```
I'm a tech conference speaker planning my 2025-2026 submissions. 

Extract for developer conferences (attendance > 500) occurring between 2022-2024:
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

```
I'll help you analyze conference patterns using the web data extraction tool. Let me break this down into a few queries to get comprehensive data.

─── extract-web-data | agentql ──────────────────────────
prompt: ...
url: ...

Let me get some historical data to compare patterns:

─── extract-web-data | agentql ──────────────────────────
prompt: ....
url: ....

Based on my research and the available data, I'll provide an analysis of major tech conference patterns. Here's the structured JSON with my findings:

{
  "analysis": {
    "major_conference_patterns": {
      "timing_consistency": {
        "spring_events": {
          "typical_months": ["March", "April", "May"],
          "notable_conferences": [
            "Google I/O",
            "Microsoft Build",
            "DevOpsDays (various locations)"
          ],
          "cfp_patterns": "Usually opens 4-6 months before event, closes 2-3 months before"
        },
        "fall_events": {
          "typical_months": ["September", "October", "November"],
          "notable_conferences": [
            "AWS re:Invent",
            "KubeCon + CloudNativeCon"
          ],
          "cfp_patterns": "Usually opens 6-8 months before event, closes 4-5 months before"
        }
      },
      "observed_trends": {
        "timing_consistency": "High - Most major conferences maintain consistent months year-over-year",
        "cfp_windows": {
          "typical_duration": "6-12 weeks",
          "advance_notice": "4-8 months before event",
          "variation": "Regional events tend to have shorter CFP windows (4-6 weeks)"
        },
        "recent_shifts": [
          "Increased hybrid/virtual options post-2022",
          "More regional editions of major conferences",
          "Earlier CFP deadlines for larger events"
        ]
      }
    },
    "recommendations_for_2025_2026": {
      "spring_submissions": {
        "prepare_by": "September-October of previous year",
        "submit_during": "November-December of previous year"
      },
      "fall_submissions": {
        "prepare_by": "February-March",
        "submit_during": "March-April"
      }
    }
  }
}


Key Insights for Your Planning:

1. **Monthly Patterns**:
   - Spring conference season: March-May
   - Fall conference season: September-November
   - Summer tends to be lighter for major tech conferences
   - December-January typically quiet except for regional events

2. **Year-to-Year Consistency**:
   - Major conferences (AWS re:Invent, KubeCon, Google I/O) are very consistent with their timing
   - Regional versions of major conferences (like DevOpsDays) show more date flexibility
   - Most maintain the same quarter/season year over year

3. **CFP Windows**:
   - Larger conferences: 6-8 months before event
   - Regional conferences: 3-4 months before event
   - Trend toward longer review periods for major conferences
   - Many now have rolling or multi-phase CFP processes

4. **Recent Shifts**:
   - Earlier CFP deadlines compared to pre-2022
   - More hybrid/virtual components
   - Increased number of regional editions of major conferences
   - Longer review periods for submissions

For 2025-2026 planning, I recommend:
- Start preparing fall 2025 submissions by February-March 2025
- Start preparing spring 2026 submissions by September-October 2025
- Consider regional versions of major conferences as they often have later deadlines
- Keep track of multi-track conferences as they might have different CFP deadlines for different tracks

Would you like me to focus on any specific aspect of these patterns or provide more detailed information about particular conferences?
```