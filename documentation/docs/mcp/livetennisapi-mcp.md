---
title: Live Tennis Extension
description: Add Live Tennis MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Live Tennis MCP Server](https://github.com/livetennisapi/livetennisapi-mcp) as a goose extension to get real-time tennis scores, player rankings and fixtures for ATP, WTA, Challenger and ITF matches.

:::tip Quick Install

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=livetennisapi-mcp&id=livetennisapi-mcp&name=Live%20Tennis&description=Real-time%20tennis%20scores%2C%20players%20and%20fixtures%20for%20ATP%2C%20WTA%2C%20Challenger%20and%20ITF&env=LIVETENNISAPI_KEY%3DLive%20Tennis%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y livetennisapi-mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  LIVETENNISAPI_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="livetennisapi-mcp"
    extensionName="Live Tennis"
    description="Real-time tennis scores, players and fixtures for ATP, WTA, Challenger and ITF"
    command="npx"
    args={["-y", "livetennisapi-mcp"]}
    envVars={[
      { name: "LIVETENNISAPI_KEY", label: "Live Tennis API Key" }
    ]}
    apiKeyLink="https://livetennisapi.com/subscribe/free"
    apiKeyLinkText="LIVETENNISAPI_KEY"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="livetennis"
      description="Real-time tennis scores, players and fixtures for ATP, WTA, Challenger and ITF"
      command="npx -y livetennisapi-mcp"
      envVars={[
        { key: "LIVETENNISAPI_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          Get a free <a href="https://livetennisapi.com/subscribe/free" target="_blank" rel="noopener noreferrer">Live Tennis API key</a> (no card required) and paste it in.
        </>
      }
    />

  </TabItem>
</Tabs>

The server exposes 12 read-only tools: `get_live_matches`, `get_upcoming_matches`, `get_recent_results`, `get_match`, `get_match_score`, `get_fixtures`, `search_players`, `get_player`, `get_match_odds`, `get_match_events`, `get_match_analysis`, and `check_api_status`. Match-winner odds, event streams and model analysis need a paid tier — on the free tier those tools explain the upgrade instead of failing.

## Example Usage

Let's use the Live Tennis extension to check on today's live matches and look up a player.

### goose Prompt

```
What tennis matches are live right now? Pick the closest one and give me
the full score, then tell me the world ranking of whoever is winning it.
```

### goose Output

```
I'll check the live matches for you.

─── get_live_matches | livetennis ──────────────────────────

There are 2 matches live right now, both at the M15 Bali (ITF) event:

1. Ryuki Matsuda vs Yuta Ichikawa — Quarter-finals
2. Thantub Suksumrarn vs Kentaro Taniguchi — Quarter-finals

The Matsuda–Ichikawa match is the tighter one. Let me pull the full score.

─── get_match_score | livetennis ──────────────────────────
match_id: 22213

Ryuki Matsuda vs Yuta Ichikawa — M15 Bali, Quarter-finals (clay)

  Set 1:  6-4 (Matsuda)
  Set 2:  3-4, currently 40-40 with Matsuda serving

Matsuda leads by a set but Ichikawa is up a break in the second —
this one is genuinely close. Matsuda is ahead overall, so let me
look up his ranking.

─── search_players | livetennis ──────────────────────────
query: Ryuki Matsuda

Ryuki Matsuda (JPN) — ITF ranking in the men's World Tennis Tour;
right-handed, plays mainly the ITF Men's circuit. At this level
players are ranked on the ITF points table rather than the ATP
top-100 list, and his profile shows him inside the ITF top 300.

Summary: Matsuda leads Ichikawa 6-4, 3-4 (40-40, serving) at the
M15 Bali quarter-finals. Want me to keep an eye on the match and
tell you when it finishes?
```
