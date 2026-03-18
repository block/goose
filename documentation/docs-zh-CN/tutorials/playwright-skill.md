---
title: "使用 Playwright CLI Skill 做 Agentic Testing"
description: "介绍如何配合 Playwright CLI，用自然语言生成自动化测试。"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

# 使用 Playwright CLI Skill 做 Agentic Testing

<iframe
  class="aspect-ratio"
  src="https://www.youtube.com/embed/_MpbmD_unnU?si=dpHvuLVkbONN_0Hk"
  title="使用 Playwright CLI skill 做 agentic testing 的教程视频"
  frameBorder="0"
  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
  referrerPolicy="strict-origin-when-cross-origin"
  allowFullScreen
></iframe>

通过 [Playwright CLI](https://github.com/microsoft/playwright-cli)，goose 可以直接打开网页、点击按钮、填写表单，并把这些交互过程转换成 Playwright 自动化测试。和 Playwright MCP 不同，Playwright CLI 不会在每次请求时都把整个页面结构发给 LLM，而是把可访问性树存放在本地。这意味着响应更快、成本更低，也不会轻易被大页面拖垮。

## 为什么要用 Skill？

LLM 不一定原生了解 Playwright CLI 的命令格式。如果直接让 agent 自己猜，常见后果就是幻觉命令、写错参数、浪费 token。[Playwright CLI Skill](https://github.com/microsoft/playwright-cli/blob/main/skills/playwright-cli/SKILL.md) 的作用，就是教 goose 在什么场景下该如何调用 CLI，以及应该用哪些具体命令。

## 前置条件

- [Node.js](https://nodejs.org/) 18 或更高版本
- 全局安装 Playwright CLI：
  ```bash
  npm install -g @playwright/cli@latest
  ```
- 如果你还想直接运行生成的测试，可选安装 [Playwright](https://playwright.dev/)（例如 `npm init playwright@latest`）

## 配置步骤

### 1. 安装 Skill

1. 在命令行里进入你的项目目录，安装 Playwright skill：

```bash
npx skills add https://github.com/microsoft/playwright-cli --skill playwright-cli
```

2. 当提示是否安装 skills package 时，输入 `y`

3. 当提示安装给哪个 agent 时，选择 `goose`

4. 当提示作用域时，选择：
   - `Global`：所有项目都可用
   - `Local`：仅当前工作目录可用

5. 当提示安装方式时，选择 `Symlink`，这样所有 agent 都能复用同一份 skill

6. 看到安装确认后，选择 `Yes` 继续

### 2. 启用 Summon 扩展

在 goose 中启用 [Summon extension](/zh-CN/docs/mcp/summon-mcp)，这样 session 才能加载 skills。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseBuiltinInstaller extensionName="Summon" />
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

## 生成带视频和 Trace 的测试

给 goose 一个完整 prompt，描述你要测试什么：

```text
Using the Playwright CLI skill, open block.github.io/goose, click on the Docs menu, click on Context Engineering, then click on Using Skills and generate a test with video and traces
```

### 它是怎么工作的

每条 `playwright-cli` 命令都会自动输出对应的 Playwright 代码。例如这条命令：

```bash
playwright-cli click e11
```

实际对应的 Playwright 代码就是：

```ts
await page.getByRole('link', { name: 'Docs' }).click();
```

### goose 会帮你做什么

通常流程包括：

1. 打开浏览器：`playwright-cli open block.github.io/goose`
2. 启动录制：`playwright-cli video-start` 和 `playwright-cli tracing-start`
3. 获取页面快照查找元素：`playwright-cli snapshot`
4. 执行点击：`playwright-cli click <ref>`
5. 结束录制：`playwright-cli video-stop` 和 `playwright-cli tracing-stop`
6. 把生成的代码组装成完整测试文件

### 生成的文件

| 文件 | 说明 |
|---|---|
| `tests/using-skills-navigation.spec.ts` | 生成的 Playwright 测试 |
| `.playwright-cli/video-*.webm` | 交互过程的视频录制 |
| `.playwright-cli/traces/*.trace` | 用于调试的 Trace 文件 |

### 生成的测试代码

最终生成的测试大致会是这样：

```typescript
import { test, expect } from '@playwright/test';

test('navigate to Using Skills guide via docs menu', async ({ page }) => {
  await page.goto('https://block.github.io/goose');
  await expect(page).toHaveTitle(/goose/);

  // Click on Docs in the navigation
  await page.getByRole('link', { name: 'Docs' }).click();

  // Expand Context Engineering category
  await page.getByRole('button', { name: 'Expand sidebar category \'Context Engineering\'' }).click();

  // Click on Using Skills
  await page.getByRole('link', { name: 'Using Skills' }).click();

  // Verify navigation
  await expect(page).toHaveURL(/using-skills/);
  await expect(page.getByRole('heading', { level: 1 })).toContainText('Using Skills');
});
```

### 运行测试

如果 Playwright 已经配置好，goose 甚至可以直接帮你把测试跑起来，确认生成结果可用。如果还没装好，goose 也可以先帮你安装 Playwright，再执行测试。

## 查看视频

如果你想回看整个过程，可以直接让 goose：

```text
Show me the video
```

goose 会通过 CLI 打开录制好的视频，让你看到整个 session 里到底发生了什么。

## 查看 Trace

如果你需要调试或复盘过程，可以让 goose：

```text
Open the trace
```

Trace viewer 会显示：

- 所有动作的时间线
- 每一步动作前后的截图
- Console logs 和错误
- 网络请求
- 使用过的元素定位器

## 多会话时的可视化 Dashboard

当 goose 同时跑多个浏览器任务时，单靠文本很难看清楚每个 session 的状态。可视化 dashboard 可以给你一个总览界面，让你实时看到所有活跃浏览器会话的进展，必要时还可以接管操作。

```text
Show playwright dashboard
```

在这里你可以看到 goose 正在控制的每个浏览器的实时预览。点开任意 session 就能放大查看，必要时也可以临时接管鼠标和键盘。如果你手动接管了，完成后按 `Escape`，goose 就会接着往下执行。

## 完整能力

如果你想知道这个 skill 还能做什么，可以直接问 goose：

```text
What else can you do with the Playwright skill?
```

| Category | Capabilities |
|---|---|
| **Browser Control** | open、goto、click、fill、close |
| **Capture & Debug** | screenshot、snapshot、video、trace |
| **Tab Management** | 打开、切换、关闭标签页 |
| **Storage & Auth** | 保存/恢复 cookies、处理登录状态 |
| **Network** | Mock API、拦截请求 |
| **Input** | 输入文本、按键、鼠标操作 |

### 常见使用场景

- ✅ 用自然语言测试 Web 应用
- ✅ 自动填写表单
- ✅ 从网站抓取数据
- ✅ 通过视频录制调试问题
- ✅ 测试认证流程
- ✅ 为文档录制演示
- ✅ 在隔离测试中 Mock API

## 总结

Playwright CLI agent skill 的上手门槛很低，但能力非常强。无论你是想生成测试、通过视频和 trace 调试，还是自动化执行复杂交互，它都提供了一种更省 token、也更贴近真实浏览器操作的方式，让 goose 能直接发挥 Playwright 的能力。

## 资源

- [Summon Extension 文档](/zh-CN/docs/mcp/summon-mcp)
- [Using Skills 指南](/zh-CN/docs/guides/context-engineering/using-skills)
- [Playwright CLI GitHub](https://github.com/microsoft/playwright-cli)
