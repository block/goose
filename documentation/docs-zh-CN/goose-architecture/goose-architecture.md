---
title: "goose 架构"
description: "介绍 goose 的核心组成、扩展互操作和交互循环。"
sidebar_position: 1
---

# goose 架构

goose 是一个开源 AI agent，建立在大语言模型的基础交互模式之上。LLM 本质上擅长处理“文本输入、文本输出”，而 goose 在这层能力之上进一步接入了工具和扩展，让 agent 可以真正执行任务。

## goose 的核心组成

goose 主要由三个部分组成：**界面（interface）**、**agent** 和已连接的[扩展](/zh-CN/docs/getting-started/using-extensions)。

- **界面**：用户实际使用的 Desktop 应用或 CLI，负责收集输入和展示输出
- **Agent**：承载 goose 核心逻辑，驱动交互循环
- **扩展**：提供具体工具和能力的组件，例如执行命令、读写文件、访问外部服务等

一次典型会话中，界面会启动一个 agent 实例，随后 agent 会同时连接一个或多个扩展。界面也可以启动多个 agent 来并行处理不同任务。

## 与扩展的互操作

[Model Context Protocol (MCP)](https://modelcontextprotocol.io/) 是一个开放标准，用于让数据源和 AI agent 之间实现互操作。goose 通过 MCP 连接各种 [MCP servers](https://github.com/modelcontextprotocol/servers?tab=readme-ov-file#model-context-protocol-servers)，在 goose 里这些 server 通常被称为扩展。

扩展通过 **tools** 向 goose 暴露能力。工具就是 agent 可以调用的函数，例如执行命令、文件操作、搜索文档等。比如 Google Drive 扩展会提供搜索文档的工具，这个工具本身就是 goose 获得对应能力的入口。

goose 内置了一批[内建扩展](/zh-CN/docs/getting-started/using-extensions#built-in-extensions)，覆盖开发、网页抓取、自动化、记忆等场景；同时也支持接入外部扩展，或者通过 [custom extensions](/zh-CN/docs/tutorials/custom-extensions) 自定义 MCP server。

## 交互循环

![交互循环示意图](../../docs/assets/guides/interactive-loop.png)

交互循环可以概括成下面几个步骤：

1. **Human Request**：你提出问题、任务或待解决的问题。
2. **Provider Chat**：goose 把你的请求和可用工具列表一起发送给当前连接的 LLM Provider。
3. **Model Extension Call**：如果模型生成了工具调用请求，goose 负责执行并收集结果。
4. **Response to Model**：执行结果会再次反馈给模型；如果还需要更多工具，流程会继续循环。
5. **Context Revision**：goose 会清理旧的或不相关的信息，让模型聚焦当前最重要的上下文。
6. **Model Response**：当工具调用完成后，模型返回最终响应，等待你的下一轮输入。

## goose 中的错误处理

goose 不会把错误简单地视为“流程终止点”。除了传统运行时错误之外，它也会捕获模型生成过程中导致的执行错误，例如无效 JSON、错误的工具名、参数不正确等。这些错误会作为工具结果回传给模型，帮助模型自行恢复并继续执行。

关于这部分的细节，可以继续阅读 [错误处理](./error-handling.md)。

## Context Revision 与 Token 管理

虽然 goose 本身是开源免费的，但实际使用时通常仍然会消耗 LLM token。对 token 的占用来自很多地方，包括消息、工具调用、资源内容、文件内容、系统指令等。

为了降低成本并保持上下文质量，goose 会做几件事：

- 使用更快、更小的模型做摘要
- 尽量带上足够上下文，而不是单纯依赖语义搜索
- 通过算法删除过旧或无关的内容
- 优先使用 find/replace，而不是重写大文件；使用 `ripgrep` 跳过系统文件；对冗长命令输出做摘要
