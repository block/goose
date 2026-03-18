---
title: "用 Remotion Skills 创建视频"
description: "介绍如何用 goose 和 Remotion agent skills 生成程序化视频。"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

# 用 Remotion Skills 创建视频

goose 可以借助 [Remotion](https://www.remotion.dev/) 创建程序化视频。Remotion 是一个基于 React 生成视频的框架。通过加载 [Remotion Agent Skills](https://www.remotion.dev/docs/ai/skills)，goose 会掌握动画、composition、文字效果和视频渲染的最佳实践。

### 前置条件

- [Node.js](https://nodejs.org/) 18 或更高版本
- 在终端中运行 `npx skills add remotion-dev/skills` 安装 [Remotion Agent Skills](https://www.remotion.dev/docs/ai/skills)

:::note Remotion License
Remotion 对个人和小团队免费，但如果公司员工数达到 3 人及以上，则需要[商业许可证](https://www.remotion.dev/license)。
:::

## 配置

启用 [Summon 扩展](/zh-CN/docs/mcp/summon-mcp)，让 goose 可以加载并使用 Agent Skills。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Summon"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. 运行 `configure` 命令：
  ```sh
  goose configure
  ```

  2. 选择 `Toggle Extensions`
  ```sh
  ┌   goose-configure
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

## 使用示例

这个例子里，我们让 goose 创建一个“终端风格”的视频，用来演示一段 git 工作流。视频里会包含打字机效果、真实终端输出，以及最后的庆祝动画。

:::info LLM
这个示例使用的是 Anthropic 的 Claude Opus 4.5。
:::

### 给 goose 的 Prompt

```text
Create a terminal-style video showing a git workflow: type out git add .,
then git commit -m 'feat: add new feature', then git push origin main.
Each command should appear with a typewriter effect, followed by realistic output.
End with 'Deployed to production 🚀'
```

### goose 的输出

```text
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
