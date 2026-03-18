---
title: "在 ACP 客户端中使用 goose"
description: "介绍如何在 Zed 等 ACP-compatible 客户端中使用 goose。"
sidebar_position: 105
---

# 在 ACP 客户端中使用 goose

支持 [Agent Client Protocol (ACP)](https://agentclientprotocol.com/) 的客户端应用可以原生连接 goose。这样你就能直接在客户端里无缝使用 goose，而不必切换到单独的界面。

:::warning Experimental Feature
ACP 仍是一项正在演进中的规范，用于让客户端和 goose 这样的 AI agent 通信。当前采用范围还有限，后续也可能继续变化。
:::

## 工作原理

在 ACP 客户端里把 goose 配置成 agent 之后，你就可以直接使用 goose 的核心 agent 能力，包括扩展和工具。除此之外，goose 还会自动把 ACP 客户端里已配置的 [MCP servers](#在-acp-客户端中使用-mcp-servers) 一并加载进来，与自身扩展共同暴露工具，无需额外配置。

客户端会自动管理 goose 的生命周期，包括：

- **初始化**：客户端运行 `goose acp` 建立连接
- **通信**：客户端通过 stdio 和 JSON-RPC 与 goose 通信
- **多会话**：客户端可以同时管理多个 goose 对话
- **会话隔离**：每个会话都维护独立状态，包括对话历史、agent 上下文和扩展配置，互不干扰

:::info Session Persistence
ACP 会话会保存到 goose 的会话历史中，因此你仍然可以用 goose 自己的能力访问和管理这些会话。不同 ACP 客户端对会话历史的暴露程度可能不一样。
:::

:::tip Reference Implementation
[goose for VS Code](/zh-CN/docs/experimental/vs-code-extension) 扩展就是通过 ACP 与 goose 通信的。要看实现细节，可以参考 [vscode-goose](https://github.com/block/vscode-goose) 仓库。
:::

## 在 ACP 客户端中进行配置

任何支持 ACP 的编辑器或 IDE 都可以把 goose 作为 agent server 接入。可用客户端可以查看 [官方 ACP clients 列表](https://agentclientprotocol.com/overview/clients)。

### 示例：在 Zed 中配置

ACP 最初由 [Zed](https://zed.dev/) 推动。下面以 Zed 为例：

#### 1. 前置条件

确保你已经安装：

- **Zed**：从 [zed.dev](https://zed.dev/) 下载
- **goose CLI**：参考[安装指南](/zh-CN/docs/getting-started/installation)

此外：

- ACP 支持最好使用 1.16.0 或更高版本，可通过 `goose --version` 检查
- 可以先临时运行 `goose acp` 测试 ACP 是否正常：

```text
~ goose acp
Goose ACP agent started. Listening on stdio...
```

按 `Ctrl+C` 退出测试。

#### 2. 把 goose 配置为自定义 Agent

在 Zed 设置中加入：

```json
{
  "agent_servers": {
    "goose": {
      "command": "goose",
      "args": ["acp"],
      "env": {}
    }
  },
  // more settings
}
```

完成后，你就可以直接在 Zed 中使用 goose。ACP 会话会复用你 goose 配置里启用的扩展，因此 Developer、Computer Controller 等工具在这里的工作方式与普通 goose 会话一致。

#### 3. 在 Zed 中开始使用 goose

1. **打开 Agent 面板**：点击 Zed 状态栏中的 agent 图标
2. **创建新线程**：点击 `+` 展开线程选项
3. **选择 goose**：点击 `New goose`
4. **开始对话**：直接在 agent 面板中与 goose 交互

#### 高级配置

##### 覆盖 Provider 和 Model

默认情况下，goose 会使用你在[配置文件](/zh-CN/docs/guides/config-files)中定义的 provider 和 model。你也可以在 ACP 客户端的配置里通过 `GOOSE_PROVIDER` 和 `GOOSE_MODEL` 环境变量覆盖默认值。

下面这个 Zed 配置示例定义了两个 goose agent 实例，适合：

- 对比不同模型在同一任务上的表现
- 简单任务用更便宜的模型，复杂任务用更强的模型

```json
{
  "agent_servers": {
    "goose": {
      "command": "goose",
      "args": ["acp"],
      "env": {}
    },
    "goose (GPT-4o)": {
      "command": "goose",
      "args": ["acp"],
      "env": {
        "GOOSE_PROVIDER": "openai",
        "GOOSE_MODEL": "gpt-4o"
      }
    }
  },
  // more settings
}
```

## 在 ACP 客户端中使用 MCP servers

ACP 客户端 `context_servers` 中配置的 MCP server 会自动对 goose 可用。这样一来，这些 server 既能服务于客户端原生功能，也能被 goose agent 直接调用。

**示例（Zed）：**

```json
{
  "context_servers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "/path/to/allowed/dir"
      ]
    }
  },
  "agent_servers": {
    "goose": {
      "command": "goose",
      "args": ["acp"],
      "env": {}
    }
  },
  // more settings
}
```

如果你想知道当前有哪些工具可用，最简单的方式就是直接在客户端里问 goose。

:::info
只要 `context_servers` 中的 MCP server 使用的是 stdio（命令行启动）或 HTTP 传输，它们就会自动对 goose 可用。goose 不支持已废弃的 SSE 传输。

如果 `context_servers` 里某个 server 的名字与 goose 自己的扩展重名，goose 会优先使用它自己的[配置](/zh-CN/docs/guides/config-files)。
:::
