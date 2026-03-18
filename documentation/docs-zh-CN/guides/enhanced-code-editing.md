---
title: "使用 AI 模型增强代码编辑"
description: "介绍如何用 AI 模型更智能地应用代码变更。"
sidebar_position: 110
---

# 使用 AI 模型增强代码编辑

[Developer 扩展](/zh-CN/docs/mcp/developer-mcp) 支持在 `str_replace` 命令中使用 AI 模型来增强代码编辑能力。配置完成后，它不再只是做简单字符串替换，而是会借助 AI 模型更智能地应用代码修改。

使用专门擅长代码编辑的模型，可以降低主 LLM provider 的负担，同时提高准确性、质量和速度，并降低成本。这种增强方式带来的收益包括：

- **理解上下文的编辑**：AI 能理解代码结构，做出更合理的修改
- **更稳定的格式**：尽量保持现有代码风格和排版一致
- **降低错误风险**：编辑时可能顺带发现并修复潜在问题
- **灵活的模型支持**：兼容任何 OpenAI-compatible API
- **实现更干净**：配置检查走正常控制流，不依赖异常处理

## 配置

设置下面这些[环境变量](/zh-CN/docs/guides/environment-variables#enhanced-code-editing)即可启用 AI 驱动的代码编辑：

```bash
export GOOSE_EDITOR_API_KEY="your-api-key-here"
export GOOSE_EDITOR_HOST="https://api.openai.com/v1"
export GOOSE_EDITOR_MODEL="gpt-4o"
```

**这三个环境变量必须全部设置且非空，功能才会生效。**

这是一个完全向后兼容的可选能力。如果没有配置，扩展会继续使用原来的简单字符串替换逻辑。

### 支持的 Provider

任何 OpenAI-compatible API 端点理论上都可以使用，例如：

**OpenAI：**

```bash
export GOOSE_EDITOR_API_KEY="sk-..."
export GOOSE_EDITOR_HOST="https://api.openai.com/v1"
export GOOSE_EDITOR_MODEL="gpt-4o"
```

**Anthropic（通过 OpenAI-compatible 代理）：**

```bash
export GOOSE_EDITOR_API_KEY="sk-ant-..."
export GOOSE_EDITOR_HOST="https://api.anthropic.com/v1"
export GOOSE_EDITOR_MODEL="claude-3-5-sonnet-20241022"
```

**Morph：**

```bash
export GOOSE_EDITOR_API_KEY="sk-..."
export GOOSE_EDITOR_HOST="https://api.morphllm.com/v1"
export GOOSE_EDITOR_MODEL="morph-v0"
```

**Relace：**

```bash
export GOOSE_EDITOR_API_KEY="rlc-..."
export GOOSE_EDITOR_HOST="https://instantapply.endpoint.relace.run/v1/apply"
export GOOSE_EDITOR_MODEL="auto"
```

**本地 / 自定义端点：**

```bash
export GOOSE_EDITOR_API_KEY="your-key"
export GOOSE_EDITOR_HOST="http://localhost:8000/v1"
export GOOSE_EDITOR_MODEL="your-model"
```

## 工作原理

当 `str_replace` 工具用于修改代码时，流程如下：

1. **检查配置**：goose 会确认这三个环境变量是否都已正确设置且非空。
2. **启用 AI 时**：如果配置齐全，goose 会把原始代码和你要求的修改一起发送给配置好的 AI 模型处理。
3. **回退逻辑**：如果没有配置 AI API，或调用 API 失败，就退回到简单字符串替换。
4. **用户提示**：第一次在未配置 AI 的情况下使用 `str_replace` 时，你会看到一条说明消息，告诉你如何开启这个能力。
