---
title: "Ollama Tool Shim"
description: "介绍如何为原生不支持 tool calling 的模型启用工具调用能力。"
sidebar_position: 2
---

# Ollama Tool Shim

Ollama tool shim 可以为那些**原生不支持 tool calling** 的模型补上工具调用能力，例如某些 DeepSeek 模型。

:::warning Experimental Feature
Ollama tool shim 仍是实验特性，后续行为和配置方式可能继续变化。
:::

它的工作方式是：

1. 主模型先输出它想执行的工具调用意图，格式为 JSON
2. 一个解释模型使用 Ollama 的结构化输出能力，把主模型消息转换成合法 JSON
3. goose 再把这段 JSON 转换成真正的工具调用并执行

## 如何启用 Ollama Tool Shim

1. 确保你已经安装并运行 [Ollama](https://ollama.com/download)
2. 默认的解释模型是 `mistral-nemo`，如果你要直接使用它，需要先拉取：

   ```bash
   ollama pull mistral-nemo
   ```

3. 如果你想改用别的解释模型，先拉取该模型，再通过 `GOOSE_TOOLSHIM_OLLAMA_MODEL` 指定。例如使用 `llama3.2`：

   ```bash
   ollama pull llama3.2
   export GOOSE_TOOLSHIM_OLLAMA_MODEL=llama3.2
   ```

4. 为了获得更好的效果，建议以更大的上下文窗口启动 Ollama：

   ```bash
   OLLAMA_CONTEXT_LENGTH=32768 ollama serve
   ```

5. 启用 tool shim：

   ```bash
   export GOOSE_TOOLSHIM=1
   ```

最后，用带有 tool shim 配置的新会话启动 goose：

```bash
GOOSE_TOOLSHIM=1 GOOSE_TOOLSHIM_OLLAMA_MODEL=llama3.2 cargo run --bin goose session
```
