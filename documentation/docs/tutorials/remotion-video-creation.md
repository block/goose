---
title: Create Videos with Remotion Skills
description: Use gosling with Remotion agent skills to generate programmatic videos with React
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GoslingBuiltinInstaller from '@site/src/components/GoslingBuiltinInstaller';

gosling can create programmatic videos using [Remotion](https://www.remotion.dev/), a framework for creating videos with React. By loading [Remotion Agent Skills](https://www.remotion.dev/docs/ai/skills), gosling gains knowledge of best practices for animations, compositions, text effects, and video rendering.

### Prerequisites

- [Node.js](https://nodejs.org/) 18 or later
- Install [Remotion Agent Skills](https://www.remotion.dev/docs/ai/skills) by running `npx skills add remotion-dev/skills` in your terminal

:::note Remotion License
Remotion is free for individuals and small teams, but requires a [commercial license](https://www.remotion.dev/license) for companies with 3+ employees.
:::

## Configuration

Enable the [Summon extension](/docs/mcp/summon-mcp) to allow gosling to load and use Agent Skills.

<Tabs groupId="interface">
  <TabItem value="ui" label="gosling Desktop" default>
  <GoslingBuiltinInstaller
    extensionName="Summon"
  />
  </TabItem>
  <TabItem value="cli" label="gosling CLI">

  1. Run the `configure` command:
  ```sh
  gosling configure
  ```

  2. Choose to `Toggle Extensions`
  ```sh
  ┌   gosling-configure 
  │
  ◇  What would you like to configure?
  │  Toggle Extensions 
  │
  ◆  Enable extensions: (use "space" to toggle and "enter" to submit)
  // highlight-start    
  │  ● summon
  // highlight-end
  |
  └  Extension settings updated successfully
  ```
  </TabItem>
</Tabs>

## Example Usage

In this example, we'll ask gosling to create a terminal-style video that demonstrates a git workflow. The video will feature typewriter effects, realistic terminal output, and a celebratory animation at the end.

:::info LLM
Anthropic's Claude Opus 4.5 was used for this example.
:::

### gosling Prompt

```
Create a terminal-style video showing a git workflow: type out git add ., 
then git commit -m 'feat: add new feature', then git push origin main. 
Each command should appear with a typewriter effect, followed by realistic output. 
End with 'Deployed to production 🚀'
```

### gosling Output

```
─── load | summon ───────────────────────────────────────
source: remotion-best-practices

reading ~/.agents/skills/remotion-best-practices/rules/animations.md

─── shell | developer ─────────────────────────────────────────
mkdir -p git-terminal-video && cd git-terminal-video && npm init -y

─── shell | developer ─────────────────────────────────────────
npm install remotion @remotion/cli react react-dom typescript @types/react

─── text_editor | developer ───────────────────────────────────
writing git-terminal-video/src/GitTerminal.tsx

─── text_editor | developer ───────────────────────────────────
writing git-terminal-video/src/Root.tsx

─── text_editor | developer ───────────────────────────────────
writing git-terminal-video/remotion.config.ts

─── shell | developer ─────────────────────────────────────────
npx remotion render GitTerminal out/git-workflow.mp4

✓ Video rendered successfully

✅ Git Terminal Video Created
Location: git-terminal-video/out/git-workflow.mp4

Video Details
├── Duration: 14 seconds (420 frames @ 30fps)
├── Resolution: 1280x720 (HD)
├── Size: ~875 KB
└── Codec: H.264

What the Video Shows
├── Terminal Window - macOS-style with traffic light buttons
├── Command Sequence with Typewriter Effect:
│   ├── ~/my-project ❯ git add .
│   ├── ~/my-project ❯ git commit -m 'feat: add new feature'
│   │   └── Shows realistic output with files changed
│   └── ~/my-project ❯ git push origin main
│       └── Shows push output (enumerating, compressing, writing)
└── Final Message - "Deployed to production 🚀" with bounce animation

Features
├── Blinking cursor while typing
├── Staggered output reveal (each line appears with delay)
├── Color-coded syntax (prompts, commands, output)
├── Spring animations for success message
└── Animated rocket that floats and bounces
```


