---
title: "ACP Providers"
description: "介绍如何把 Claude Code、Codex 等 ACP agents 作为 goose provider 使用，并继续保留扩展支持。"
sidebar_position: 46
---

# ACP Providers

goose 支持把 [Agent Client Protocol (ACP)](https://agentclientprotocol.com/) agent 当作 provider 使用。ACP 是一种用于和 coding agent 通信的标准协议，目前已经有一个不断扩展的 [registry](https://github.com/agentclientprotocol/registry) 收录实现了该协议的 agent。

ACP provider 会把 goose 的[扩展](/zh-CN/docs/getting-started/using-extensions)以 MCP server 的形式透传给 agent，因此 agent 可以直接调用你的扩展。

:::warning Limitations
- **暂不支持 fork 或 resume 既有 session**：你可以启动新会话，但 `goose session resume` 和 `goose session fork` 目前还不支持。
- **ACP session ID 与 goose session ID 不同**：某些 telemetry 字段在两套 ID 之间无法一一对应。
:::

## 可用的 ACP Providers

### Claude ACP

它封装的是 [claude-agent-acp](https://github.com/zed-industries/claude-agent-acp)，也就是 Anthropic Claude Code 的 ACP 适配器。它使用的是与你 `claude-code` CLI provider 相同的 Claude 订阅。

**要求：**

- Node.js 和 npm
- 有效的 Claude Code 订阅
- 已使用 Anthropic 账号完成认证（`claude` CLI 可正常工作）

### Codex ACP

它封装的是 [codex-acp](https://github.com/zed-industries/codex-acp)，也就是 OpenAI Codex 的 ACP 适配器。它使用的是与你 `codex` CLI provider 相同的 ChatGPT 订阅。Codex 默认会阻止网络访问；如果配置了 HTTP MCP server，goose 会自动开启网络访问能力。

**要求：**

- Node.js 和 npm
- 有效的 ChatGPT Plus / Pro 订阅，或 OpenAI API credits
- 已使用 OpenAI 账号完成认证（`codex` CLI 可正常工作）

## 配置说明

### Claude ACP

1. **安装 ACP 适配器**

   ```bash
   npm install -g @zed-industries/claude-agent-acp
   ```

2. **完成 Claude 认证**

   确保 Claude CLI 已完成认证且可以正常使用。

3. **配置 goose**

   设置 provider 环境变量：

   ```bash
   export GOOSE_PROVIDER=claude-acp
   ```

   或者通过 `goose configure` 在 CLI 中设置：

   ```bash
   ┌   goose-configure
   │
   ◇  What would you like to configure?
   │  Configure Providers
   │
   ◇  Which model provider should we use?
   │  Claude Code
   │
   ◇  Model fetch complete
   │
   ◇  Enter a model from that provider:
   │  default
   ```

### Codex ACP

1. **安装 ACP 适配器**

   ```bash
   npm install -g @zed-industries/codex-acp
   ```

2. **完成 OpenAI 认证**

   运行 `codex`，按提示完成认证。你可以用 ChatGPT 账号，也可以使用 API key。

3. **配置 goose**

   设置 provider 环境变量：

   ```bash
   export GOOSE_PROVIDER=codex-acp
   ```

   或者通过 `goose configure` 在 CLI 中设置：

   ```bash
   ┌   goose-configure
   │
   ◇  What would you like to configure?
   │  Configure Providers
   │
   ◇  Which model provider should we use?
   │  Codex CLI
   │
   ◇  Model fetch complete
   │
   ◇  Enter a model from that provider:
   │  gpt-5.2-codex
   ```

## 使用示例

### 基本用法

```bash
goose session
```

### 搭配扩展使用

通过 `--with-extension` 或 `--with-streamable-http-extension` 配置的扩展，会透传给 ACP agent：

```bash
GOOSE_PROVIDER=claude-acp goose run \
  --with-extension 'npx -y @modelcontextprotocol/server-everything' \
  -t 'Use the echo tool to say hello'
```

```bash
GOOSE_PROVIDER=codex-acp goose run \
  --with-streamable-http-extension 'https://mcp.kiwi.com' \
  -t 'Search for flights from BKI to SYD tomorrow'
```

## 配置项

### Claude ACP 配置

| Environment Variable | 说明 | 默认值 |
|----------------------|------|--------|
| `GOOSE_PROVIDER` | 固定为 `claude-acp` | 无 |
| `GOOSE_MODEL` | 使用的模型 | `default` |

**已知模型：**

- `default`（opus）
- `sonnet`
- `haiku`

**权限模式（`GOOSE_MODE`）：**

| 模式 | Session Mode | 行为 |
|------|--------------|------|
| `auto` | `bypassPermissions` | 跳过所有权限检查 |
| `smart-approve` | `acceptEdits` | 自动接受文件编辑，危险操作才询问 |
| `approve` | `default` | 所有需要权限的操作都询问 |
| `chat` | `plan` | 只做规划，不执行工具 |

关于 session mode 的细节，可参考 [claude-agent-acp](https://github.com/zed-industries/claude-agent-acp)。

### Codex ACP 配置

| Environment Variable | 说明 | 默认值 |
|----------------------|------|--------|
| `GOOSE_PROVIDER` | 固定为 `codex-acp` | 无 |
| `GOOSE_MODEL` | 使用的模型 | `gpt-5.2-codex` |

**已知模型：**

- `gpt-5.2-codex`
- `gpt-5.2`
- `gpt-5.1-codex-max`
- `gpt-5.1-codex-mini`

**权限模式（`GOOSE_MODE`）：**

| 模式 | Approval / Sandbox | 行为 |
|------|--------------------|------|
| `auto` | 无审批，完全访问 | 跳过所有审批和 sandbox 限制 |
| `smart-approve` | On-request，workspace-write | 工作区内可写，超出 sandbox 的操作才询问 |
| `approve` | On-request，只读 | 默认只读，所有写操作都询问 |
| `chat` | 无审批，只读 | 只读 sandbox，不执行工具 |

关于审批策略和 sandbox 细节，可参考 [codex-acp](https://github.com/zed-industries/codex-acp)。

## 错误处理

ACP provider 依赖外部 npm 包，请确认：

- ACP 适配器二进制已经安装并在 PATH 中（`claude-agent-acp` 或 `codex-acp`）
- 底层 CLI 工具已完成认证且能正常工作
- 订阅额度没有超限
- Node.js 和 npm 已安装

如果 goose 找不到对应二进制，启动 session 时就会直接失败。你可以用 `which claude-agent-acp` 或 `which codex-acp` 来确认安装状态。
