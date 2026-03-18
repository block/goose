---
title: "goose for VS Code 扩展"
description: "介绍如何通过 ACP 在 VS Code 中直接与 goose 交互。"
sidebar_position: 4
---

import IconGoose from "@site/src/components/icons/goose";

# goose for VS Code 扩展

[goose for VS Code extension](https://marketplace.visualstudio.com/items?itemName=block.vscode-goose) 让你可以直接在 VS Code 内部与 goose 交互。

:::warning Experimental Feature
goose for VS Code 仍在活跃开发中，后续行为和配置方式可能继续变化。
:::

这个扩展通过 [Agent Client Protocol (ACP)](/zh-CN/docs/guides/acp-clients) 与 goose 通信。

## 安装扩展

:::info Prerequisites
- [VS Code](https://code.visualstudio.com/) 1.95.0 及以上
- [goose CLI](/zh-CN/docs/getting-started/installation) 1.16.0 及以上
:::

1. 打开 [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=block.vscode-goose)
2. 点击 `Install`
3. 在弹出的窗口中点击 `Open Visual Studio Code`
4. 在 VS Code 中点击 `Install`
5. 点击 Activity Bar 里的 <IconGoose className="inline" size={16} /> 图标，打开侧边栏并开始和 goose 聊天

## 如何给 goose 提供上下文

### 发送选中代码

1. 在编辑器中选中你想讨论的代码
2. 把选中内容发送给 goose：
   - macOS：`Cmd+Shift+G`
   - Windows/Linux：`Ctrl+Shift+G`
   - 或右键选择 `Send to goose`

选中的代码会作为 context chip 出现在聊天输入框上方。

### 用 `@` 附加文件

1. 在聊天框输入 `@`，并开始输入文件名
2. 从候选列表中选择文件：
   - 用方向键上下选择后按 `Enter`
   - 或直接用鼠标点击

附件会作为 context chip 显示在输入框上方。

## 示例用法

这个扩展让 goose 直接连接你的编辑器，用 AI 协助处理文件、项目和开发工作流。主要能力包括：

- 流式响应的聊天 UI
- 访问 goose 当前启用的能力和扩展
- session 管理和历史回放
- 文件上下文共享和可视化 context chips
- 便于复制代码片段和回复内容的剪贴板工具

### goose Prompt

```
Update the contributing guide at /goose/CONTRIBUTING.md with instructions on how to start docusaurus dev server
```

### goose Output

```

I'll help you update the CONTRIBUTING.md file with instructions for starting the Docusaurus dev server. First, let me check the current content of the file.

─── developer__text_editor | view ──────────────────────────

I'll now update the CONTRIBUTING.md file to include instructions for starting the Docusaurus dev server. I'll add this information in a new section under the Node section.

─── developer__text_editor | str_replace ──────────────────────────

I've successfully updated the CONTRIBUTING.md file to include instructions for starting the Docusaurus dev server. I added a new "Documentation" section right after the Node section, which provides clear step-by-step instructions for:

Navigating to the docs directory
Installing dependencies
Starting the development server
I also included helpful information about the hot-reloading feature and the local server URL. The formatting is consistent with the rest of the document, using proper markdown syntax for code blocks and headers.

Is there anything else you'd like me to add or modify in the contributing guide?

```
