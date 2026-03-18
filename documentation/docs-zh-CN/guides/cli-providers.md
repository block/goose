---
sidebar_position: 45
title: CLI Providers
sidebar_label: CLI Providers
description: 在 goose 中使用 Claude Code、Codex、Cursor Agent 或 Gemini CLI 订阅
---

# CLI Providers

goose 可以使用一类“透传型 provider”，把 Anthropic、OpenAI、Cursor 和 Google 提供的现有 CLI 工具接入进来。这样你就能通过 goose 的界面继续使用 Claude Code、Codex、Cursor Agent 和 Gemini CLI 的现有订阅，同时获得 goose 的会话管理、持久化和工作流集成能力。

:::warning Limitations
这些 providers 并不完整支持所有 goose 能力，某些 provider 还会受平台或功能限制影响，出现问题时也可能需要手动排查。它们被纳入 goose，主要是为了方便集成，而不是作为最完整的 goose 运行模式。
:::

## 为什么要用 CLI Providers？

CLI providers 特别适合这些场景：

- 你已经有 Claude Code、Codex、Cursor 或 Gemini CLI 订阅，希望通过 goose 继续使用，而不是改成按 token 计费
- 你需要会话持久化，以便保存、恢复和导出对话历史
- 你想把 goose recipes 和 schedule 结合起来，形成可重复执行的工作流
- 你希望不同 AI provider 都使用统一的命令和交互方式
- 你希望在同一个任务中[组合多个模型一起工作](#combining-with-other-models)

### 优势

#### 会话管理
- **持久化对话**：重启后仍可保存和恢复会话
- **导出能力**：导出对话历史与相关产物
- **会话组织**：管理多条不同的对话线程

#### 工作流集成
- **兼容 recipes**：在自动化 goose recipes 中使用 CLI providers
- **支持调度任务**：接入 schedule 和自动化工作流
- **混合配置**：可与其他 LLM providers 配合，使用 lead / worker 模式

#### 一致的接口体验
- **统一命令**：无论底层 provider 是谁，都使用 `goose session` 这一套界面
- **统一配置方式**：都纳入 goose 的配置系统管理

:::warning Extensions
CLI providers **不会提供** goose 的扩展生态（MCP servers、第三方集成等）。它们会优先使用各自 CLI 工具的内建工具，以避免冲突。如果你需要 goose 的扩展能力，请改用标准 [API providers](/zh-CN/docs/getting-started/providers#available-providers)。
:::


## 可用的 CLI Providers

### Claude Code

Claude Code provider 会对接 Anthropic 的 [Claude CLI tool](https://claude.ai/cli)，让你通过现有 Claude Code 订阅使用 Claude 模型。

**特点：**
- 使用 Claude 最新模型
- 200,000 token 上下文窗口
- 会自动把 goose 扩展信息从 system prompt 中剔除（因为 Claude Code 有自己的工具生态）
- 使用 streaming JSON（NDJSON）协议维持多轮持久会话

**要求：**
- 已安装并配置 Claude CLI
- 拥有有效的 Claude Code 订阅
- Claude CLI 已完成 Anthropic 账号认证

### OpenAI Codex

Codex provider 会对接 OpenAI 的 [Codex CLI tool](https://developers.openai.com/codex/cli)，让你通过现有 ChatGPT Plus / Pro 订阅或 OpenAI API credits 使用 OpenAI 模型。

**特点：**
- 支持 OpenAI GPT-5 系列模型（`gpt-5.2-codex`、`gpt-5.2`、`gpt-5.1-codex-max`、`gpt-5.1-codex-mini`）
- 可配置 reasoning effort（`low`、`medium`、`high`、`xhigh`；`none` 仅在非 codex 模型如 `gpt-5.2` 上支持）
- 可选 skills 支持
- 支持解析结构化 JSON 输出
- 会自动把 goose 扩展信息从 system prompt 中剔除

**要求：**
- 已安装 Codex CLI（`npm i -g @openai/codex` 或 `brew install --cask codex`）
- 有效的 ChatGPT Plus / Pro 订阅或 OpenAI API credits
- Codex CLI 已完成 OpenAI 账号认证
- 默认情况下，Codex 需要在 git 仓库中运行。如需跳过，可设置 `CODEX_SKIP_GIT_CHECK=true`

### Cursor Agent

Cursor provider 会对接 Cursor 的 [CLI agent](https://docs.cursor.com/en/cli/installation)，通过你现有订阅访问 Cursor Agent。

**特点：**

- 适合编码类任务
- 更偏向代码工作流和文件交互

**要求：**

- 已安装并配置 `cursor-agent`
- CLI 工具已完成认证

### Gemini CLI

Gemini CLI provider 会对接 Google 的 [Gemini CLI tool](https://ai.google.dev/gemini-api/docs)，通过你的 Google AI 订阅访问 Gemini 模型。

**特点：**
- 1,000,000 token 上下文窗口

**要求：**
- 已安装并配置 Gemini CLI
- CLI 已完成 Google 账号认证

## 配置步骤

### Claude Code

1. **安装 Claude CLI**
   
   按照 [Claude Code 安装说明](https://docs.anthropic.com/en/docs/claude-code/overview) 安装并配置 Claude CLI。

2. **完成 Claude 认证**
   
   确认 Claude CLI 已认证成功且可正常运行。

3. **配置 goose**
   
   设置 provider 环境变量：
   ```bash
   export GOOSE_PROVIDER=claude-code
   ```
   
   或者通过 `goose configure` 配置：

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

### OpenAI Codex

1. **安装 Codex CLI**

   使用 npm 或 Homebrew 安装：
   ```bash
   npm i -g @openai/codex
   # or
   brew install --cask codex
   ```

2. **完成 OpenAI 认证**

   运行 `codex`，按提示完成认证。你可以使用 ChatGPT 账号，也可以使用 API key。

3. **配置 goose**

   设置 provider 环境变量：
   ```bash
   export GOOSE_PROVIDER=codex
   ```

   或通过 `goose configure`：

   ```bash
   ┌   goose-configure
   │
   ◇  What would you like to configure?
   │  Configure Providers
   │
   ◇  Which model provider should we use?
   │  OpenAI Codex CLI
   │
   ◇  Model fetch complete
   │
   ◇  Enter a model from that provider:
   │  gpt-5.2-codex
   ```

### Cursor Agent

1. **安装 Cursor Agent**

   参考 [Cursor Agent 安装说明](https://docs.cursor.com/en/cli/installation) 安装并配置 Cursor Agent。

2. **完成 Cursor 认证**

   确保 Cursor Agent 已认证成功并能正常运行。

3. **配置 goose**

   设置 provider 环境变量：

   ```bash
   export GOOSE_PROVIDER=cursor-agent
   ```

   或通过 `goose configure`：

   ```bash
   ┌   goose-configure
   │
   ◇  What would you like to configure?
   │  Configure Providers
   │
   ◇  Which model provider should we use?
   │  Cursor Agent
   │
   ◇  Model fetch complete
   │
   ◇  Enter a model from that provider:
   │  default
   ```

### Gemini CLI

1. **安装 Gemini CLI**
   
   按照 [Gemini CLI 安装说明](https://blog.google/technology/developers/introducing-gemini-cli-open-source-ai-agent/) 安装并配置 Gemini CLI。

2. **完成 Google 认证**
   
   确保 Gemini CLI 已认证成功并能正常工作。

3. **配置 goose**
   
   设置 provider 环境变量：
   ```bash
   export GOOSE_PROVIDER=gemini-cli
   ```
   
   或通过 `goose configure`：

   ```bash
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Configure Providers 
   │
   ◇  Which model provider should we use?
   │  Gemini CLI 
   │
   ◇  Model fetch complete
   │
   ◇  Enter a model from that provider:
   │  default
   ```

## 使用示例

### 基本使用

配置完成后，就可以像其他 provider 一样启动 goose 会话：

```bash
goose session
```

### 与其他模型组合使用 {#combining-with-other-models}

CLI providers 可以和其他模型配合，走 goose 的 [lead/worker 模式](/zh-CN/docs/tutorials/lead-worker)：

```bash
# 用 Claude Code 做 lead 模型，GPT-4o 做 worker
export GOOSE_LEAD_PROVIDER=claude-code
export GOOSE_PROVIDER=openai
export GOOSE_MODEL=gpt-4o
export GOOSE_LEAD_MODEL=default

goose session
```

## 配置选项

### Claude Code 配置

| 环境变量 | 说明 | 默认值 |
|---------------------|------|---------|
| `GOOSE_PROVIDER` | 设置为 `claude-code` 以使用该 provider | 无 |
| `GOOSE_MODEL` | 要使用的模型（只有 `sonnet` 或 `opus` 会传给 CLI） | `claude-sonnet-4-20250514` |
| `CLAUDE_CODE_COMMAND` | Claude CLI 命令路径 | `claude` |

**已识别模型：**

以下模型会通过 `--model` 传给 Claude CLI。如果 `GOOSE_MODEL` 设置为列表之外的值，则不会附带模型参数，Claude Code 会使用自己的默认值：

- `default`（opus）
- `sonnet`
- `haiku`

**权限模式（`GOOSE_MODE`）：**

| 模式 | Claude Code Flag | 行为 |
|------|------------------|------|
| `auto` | `--dangerously-skip-permissions` | 跳过所有权限提示 |
| `smart-approve` | `--permission-prompt-tool stdio` | 通过控制协议路由权限检查（按需提示） |
| `approve` | `--permission-prompt-tool stdio` | 通过控制协议路由权限检查（按需提示） |
| `chat` | （无） | 使用 Claude Code 默认行为 |

### Cursor Agent 配置

| 环境变量 | 说明 | 默认值 |
|---------------------|------|---------|
| `GOOSE_PROVIDER` | 设置为 `cursor-agent` 以使用该 provider | 无 |
| `CURSOR_AGENT_COMMAND` | Cursor Agent 命令路径 | `cursor-agent` |

### OpenAI Codex 配置

| 环境变量 | 说明 | 默认值 |
|---------------------|------|---------|
| `GOOSE_PROVIDER` | 设置为 `codex` 以使用该 provider | 无 |
| `GOOSE_MODEL` | 要使用的模型（只有已知模型会传给 CLI） | `gpt-5.2-codex` |
| `CODEX_COMMAND` | Codex CLI 命令路径 | `codex` |
| `CODEX_REASONING_EFFORT` | reasoning effort：`low`、`medium`、`high` 或 `xhigh`（`none` 仅在非 codex 模型如 `gpt-5.2` 上支持） | `high` |
| `CODEX_ENABLE_SKILLS` | 是否启用 Codex skills：`true` / `false` | `true` |
| `CODEX_SKIP_GIT_CHECK` | 是否跳过 git 仓库要求：`true` / `false` | `false` |

**已识别模型：**

以下模型会通过 `-m` 参数传给 Codex CLI。如果 `GOOSE_MODEL` 不在列表中，则不会传模型参数，Codex 会使用自身默认值：

- `gpt-5.2-codex`（400K 上下文，支持自动 compact）
- `gpt-5.2`（400K 上下文，支持自动 compact）
- `gpt-5.1-codex-max`（256K 上下文）
- `gpt-5.1-codex-mini`（256K 上下文）

:::note Legacy Models
这些是 Codex CLI v0.77.0 默认支持的模型。如需访问旧模型或 legacy 模型，可以直接运行 `codex -m <model_name>`，或在 Codex 的 `config.toml` 中配置。详见 [Codex CLI documentation](https://developers.openai.com/codex/cli)。
:::

**权限模式（`GOOSE_MODE`）：**

| 模式 | Codex Flag | 行为 |
|------|------------|------|
| `auto` | `--yolo` | 跳过所有审批和 sandbox 限制 |
| `smart-approve` | `--full-auto` | workspace-write sandbox，仅在失败时审批 |
| `approve` | （无） | 交互式审批（Codex 默认行为） |
| `chat` | `--sandbox read-only` | 只读 sandbox 模式 |

### Gemini CLI 配置

| 环境变量 | 说明 | 默认值 |
|---------------------|------|---------|
| `GOOSE_PROVIDER` | 设置为 `gemini-cli` 以使用该 provider | 无 |
| `GEMINI_CLI_COMMAND` | Gemini CLI 命令路径 | `gemini` |

## 它是如何工作的

### System Prompt 过滤

CLI providers 会自动把 goose 扩展相关信息从 system prompt 中过滤掉，因为这些 CLI 工具本身有各自的工具生态。这样可以避免提示词冲突，也能让底层 CLI 工具获得更干净的输入。

### 消息翻译

- **Claude Code**：把 goose 消息转换成带角色前缀（Human / Assistant）的文本块，方式和 Codex / Gemini CLI 类似
- **Codex**：把消息转换成带角色前缀的纯文本 prompt，和 Gemini CLI 类似
- **Cursor Agent**：把 goose 消息转换成 Cursor 使用的 JSON 消息格式，正确处理工具调用与工具响应
- **Gemini CLI**：把消息转换成带角色前缀的纯文本 prompt

### 响应处理

- **Claude Code**：解析 streaming JSON 输出，提取文本内容与 usage 信息
- **Codex**：解析逐行 JSON 事件，提取文本内容与 usage 信息
- **Cursor Agent**：解析 JSON 响应，提取文本内容与 usage 信息
- **Gemini CLI**：处理 CLI 工具输出的纯文本响应

## 错误处理

CLI providers 依赖外部工具，因此要优先确认：

- CLI 工具已正确安装并在 PATH 中
- 认证状态仍有效
- 没有超出订阅使用限制
- 对于 Codex：你当前在 git 仓库中，或已设置 `CODEX_SKIP_GIT_CHECK=true`


---

CLI providers 的核心价值，是让你通过 goose 的界面继续复用现有 AI CLI 订阅，同时获得会话管理和工作流集成能力。对于已经有 CLI 订阅、又希望统一管理 session 和 recipes 的用户来说，它们会非常有价值。
