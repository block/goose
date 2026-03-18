---
sidebar_position: 3
title: "Code Mode"
sidebar_label: "Code Mode"
description: "介绍 Code Mode 如何进行工具发现和工具调用。"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Code Mode 是一种以编程方式与 MCP 工具交互的方法，而不是直接逐个调用工具。当你启用了很多扩展时，Code Mode 尤其有用，因为它能更高效地管理上下文窗口的使用。

:::info
这项功能需要启用内置的 [Code Mode 扩展](/zh-CN/docs/mcp/code-mode-mcp)。
:::

Code Mode 主要控制两件事：
- 何时发现可用工具，以及何时把工具定义加载进上下文
- 以什么方式执行工具调用

它的主要特性包括：
- 仅在需要时按需发现并加载已启用扩展中的工具
- 把多个工具调用打包到一次执行中
- 支持把一个工具的中间结果传给下一个工具继续处理

## Code Mode 的工作方式

[Code Mode 扩展](/zh-CN/docs/mcp/code-mode-mcp) 本身是一个 MCP server，它通过 MCP 协议暴露 3 个基础的 meta-tool。启用 Code Mode 后，goose 会切换到这种模式。在每次请求中，LLM 都会编写一段 JavaScript，然后由 goose 使用基于 Deno 的自定义运行时 [pctx (Port of Context)](https://github.com/AdrianCole/pctx) 来执行。这个过程会：

- 在需要时发现你当前已启用扩展中的工具
- 了解当前任务真正需要用到哪些工具
- 以编程方式调用这些工具完成任务

### 传统工具调用 vs Code Mode

传统 MCP 工具调用和 Code Mode 的目标相同，都是让 goose 能访问工具；区别在于它们采用了不同的路径。

| 维度 | Traditional | Code Mode |
|--------|------------------|-----------|
| **工具发现** | 所有已启用扩展中的工具都会直接暴露出来，例如：<br/>• `developer.shell`<br/>• `developer.text_editor`<br/>• `github.list_issues`<br/>• `github.get_pull_request`<br/>• `slack.send_message`<br/>• ... *可能还有很多更多* | 只先暴露 Code Mode 扩展自己的 3 个 meta-tool：<br/>• `list_functions`<br/>• `get_function_details`<br/>• `execute`<br/><br/>然后由 LLM 按需发现其他扩展中的工具 |
| **工具调用** | • 顺序逐个调用工具<br/>• 每个结果都会回传给 LLM，再决定下一步 | • 可能先做工具发现<br/>• 多个工具调用可以批量执行<br/>• 中间结果可以在本地链式传递 |
| **上下文窗口** | 每次 LLM 调用都要携带所有已启用工具的定义 | 每次 LLM 调用只需携带这 3 个 meta-tool，以及当前会话里已经发现过的工具定义 |
| **最适合** | • 只启用了 1-3 个扩展<br/>• 使用 1-2 个工具的简单任务 | • 启用了 5 个以上扩展<br/>• 边界清晰、步骤明确的多阶段工作流 |

:::info 仅支持文本结果
Code Mode 目前只支持工具返回的文本内容。图片、二进制数据和其他非文本内容会被忽略。
:::

## 更多资源

import ContentCardCarousel from '@site/src/components/ContentCardCarousel';
import gooseCodeMode from '@site/blog/2025-12-15-code-mode-mcp/header-image.jpg';
import notMcpReplacement from '@site/blog/2025-12-21-code-mode-doesnt-replace-mcp/header-image.png';

<ContentCardCarousel
  items={[
    {
      type: 'blog',
      title: 'Code Mode for MCP',
      description: '了解基于代码执行的 MCP 工具调用方式。',
      thumbnailUrl: gooseCodeMode,
      linkUrl: '/goose/blog/2025/12/15/code-mode-mcp',
      date: '2025-12-15',
      duration: '5 min read'
    },
    {
      type: 'blog',
      title: 'Code Mode Doesn\'t Replace MCP',
      description: '理解 Code Mode 与 MCP 是如何协同工作的。',
      thumbnailUrl: notMcpReplacement,
      linkUrl: '/goose/blog/2025/12/21/code-mode-doesnt-replace-mcp',
      date: '2025-12-21',
      duration: '8 min read'
    }
  ]}
/>
