---
title: "错误处理"
description: "介绍 goose 如何区分传统错误与 agent 错误，并把可恢复错误反馈给模型。"
---

# 错误处理

错误处理是 goose 性能和稳定性的重要组成部分。由于 LLM 生成本身具有非确定性，模型可能会产生可以自我修复的错误；而 goose 的职责之一，就是把这些错误转成模型能够理解并继续处理的反馈。

## 传统错误

在 agent 运行过程中，可能会出现网络波动、底层模型不可用等传统运行时错误。这类错误会通过 agent API 返回给调用方，由调用方决定如何处理。goose 在这部分通常使用 [anyhow::Error][anyhow-error]。

## Agent 错误

还有一类错误是“系统本身没有坏，但模型生成的内容导致执行失败”。例如：

- 生成了不存在的工具名
- 传入了不正确的参数
- 工具调用结构是合法的，但工具本身执行失败

这些错误都可以回传给 LLM，让模型尝试恢复。某种意义上，错误消息本身也是 prompt 的一部分，因为它会告诉模型“哪里错了，以及应该怎么继续修正”。

goose 在这类场景中通常使用 [thiserror::Error][this-error] 来维护一组可预测、可提示给模型的错误类型。

## ToolUse 与 ToolResult

为了覆盖这些场景，`ToolUse` 和 `ToolResult` 往往会以 `Result<T, AgentError>` 的形式在 API 中传递。

- 如果 `ToolUse` 本身有错误，它会立刻转换成一个错误 `ToolResult`
- 如果 `ToolUse` 合法，但工具执行失败，也会得到错误 `ToolResult`
- 这些错误结果都会回传给 LLM，帮助模型继续处理

Provider 层随后会把这些 agent 错误转换成各自 API 规范里的合法消息结构。

[anyhow-error]: https://docs.rs/anyhow/latest/anyhow/
[this-error]: https://docs.rs/thiserror/latest/thiserror/
