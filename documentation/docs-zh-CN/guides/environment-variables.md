---
title: "环境变量"
description: "汇总 goose 可用环境变量、用途、默认值与配置示例。"
sidebar_position: 95
---

# 环境变量

goose 支持大量环境变量，可用于调整模型、会话、工具、安全、网络和可观测性行为。本文按功能分组列出当前可用变量，并给出常见配置示例。

## 模型配置

这一组变量用于控制 [语言模型](/zh-CN/docs/getting-started/providers) 及其运行方式。

### 基础 Provider 配置

这是开始使用 goose 所需的最小配置集合。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_PROVIDER` | 指定要使用的 LLM provider | [查看可用 providers](/zh-CN/docs/getting-started/providers#available-providers) | 无（必须先[配置](/zh-CN/docs/getting-started/providers#configure-provider-and-model)） |
| `GOOSE_MODEL` | 指定 provider 下要使用的模型 | 模型名，例如 `"gpt-4"`、`"claude-sonnet-4-5-20250929"` | 无（必须先[配置](/zh-CN/docs/getting-started/providers#configure-provider-and-model)） |
| `GOOSE_TEMPERATURE` | 设置模型回答的 [temperature](https://medium.com/@kelseyywang/a-comprehensive-guide-to-llm-temperature-%EF%B8%8F-363a40bbc91f) | `0.0` 到 `1.0` 的浮点数 | 由模型决定 |
| `GOOSE_MAX_TOKENS` | 设置单次模型响应的最大 token 数，超出会截断 | 正整数，例如 `4096`、`8192` | 由模型决定 |

**示例**

```bash
# 基础模型配置
export GOOSE_PROVIDER="anthropic"
export GOOSE_MODEL="claude-sonnet-4-5-20250929"
export GOOSE_TEMPERATURE=0.7

# 更短的输出
export GOOSE_MAX_TOKENS=4096

# 更长的输出，例如代码生成
export GOOSE_MAX_TOKENS=16000
```

### 高级 Provider 配置 {#advanced-provider-configuration}

当你使用自定义 endpoint、企业部署或特定 provider 实现时，会用到这一组变量。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_PROVIDER__TYPE` | 指定 provider 的具体类型/实现 | [查看可用 providers](/zh-CN/docs/getting-started/providers#available-providers) | 从 `GOOSE_PROVIDER` 推导 |
| `GOOSE_PROVIDER__HOST` | 为 provider 指定自定义 API endpoint | URL，例如 `"https://api.openai.com"` | provider 自带默认值 |
| `GOOSE_PROVIDER__API_KEY` | provider 的认证密钥 | API key 字符串 | 无 |
| `GEMINI3_THINKING_LEVEL` | 全局设置 Gemini 3 的 [thinking level](/zh-CN/docs/getting-started/providers#gemini-3-thinking-levels) | `low`、`high` | `low` |

**示例**

```bash
# 高级 provider 配置
export GOOSE_PROVIDER__TYPE="anthropic"
export GOOSE_PROVIDER__HOST="https://api.anthropic.com"
export GOOSE_PROVIDER__API_KEY="your-api-key-here"
```

### 自定义模型定义

你可以使用 `GOOSE_PREDEFINED_MODELS` 定义自定义模型配置，包括 provider 特定参数和上下文窗口限制。这在开启 provider beta 能力、使用超长上下文窗口、或接入内部模型时尤其有用。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_PREDEFINED_MODELS` | 定义自定义模型配置 | 模型对象组成的 JSON 数组 | 无 |

**模型配置字段**

| 字段 | 必填 | 类型 | 说明 |
|-------|----------|------|-------------|
| `id` | 否 | number | 可选数字 ID |
| `name` | 是 | string | 模型名称，配置时用它引用 |
| `provider` | 是 | string | provider 名称，例如 `"databricks"`、`"openai"`、`"anthropic"` |
| `alias` | 否 | string | 展示名称 |
| `subtext` | 否 | string | 附加说明文字 |
| `context_limit` | 否 | number | 覆盖默认上下文窗口大小（token） |
| `request_params` | 否 | object | 请求里附带的 provider 特定参数 |

:::info
当前 `id`、`alias` 和 `subtext` 字段还没有实际被使用。
:::

如果自定义模型配置里指定了 `context_limit`，它会优先于 goose 的模型模式匹配，但仍然可以被显式环境变量（例如 [`GOOSE_CONTEXT_LIMIT`](#model-context-limit-overrides)）覆盖。

**示例**

```bash
# 使用 Anthropic beta header 开启 1M 上下文
export GOOSE_PREDEFINED_MODELS='[
  {
    "id": 1,
    "name": "claude-sonnet-4-1m",
    "provider": "anthropic",
    "alias": "Claude Sonnet 4 (1M context)",
    "subtext": "Anthropic",
    "context_limit": 1000000,
    "request_params": {
      "anthropic_beta": ["context-1m-2025-08-07"]
    }
  }
]'

# 定义多个自定义模型
export GOOSE_PREDEFINED_MODELS='[
  {
    "id": 1,
    "name": "gpt-4-custom",
    "provider": "openai",
    "alias": "GPT-4 (200k)",
    "context_limit": 200000
  },
  {
    "id": 2,
    "name": "internal-model",
    "provider": "databricks",
    "alias": "Internal Model (500k)",
    "context_limit": 500000
  }
]'

# Gemini 3 高 thinking level
export GOOSE_PREDEFINED_MODELS='[
  {
    "name": "gemini-3-pro",
    "provider": "google",
    "request_params": {"thinking_level": "high"}
  }
]'
```

使用这些模型时，自定义上下文限制和请求参数会自动生效。自定义上下文限制也会显示在 goose CLI 的 [token 使用指示器](/zh-CN/docs/guides/sessions/smart-context-management#token-usage) 中。

### Lead/Worker 模型配置 {#leadworker-model-configuration}

这一组变量用于配置 [lead/worker 模型模式](/zh-CN/docs/tutorials/lead-worker)：先用更强的 lead 模型完成规划和复杂推理，再自动切到更快或更便宜的 worker 模型执行。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_LEAD_MODEL` | **启用 lead 模式所必需。**指定 lead 模型名称 | 模型名，例如 `"gpt-4o"`、`"claude-sonnet-4-20250514"` | 无 |
| `GOOSE_LEAD_PROVIDER` | lead 模型使用的 provider | [查看可用 providers](/zh-CN/docs/getting-started/providers#available-providers) | 回退到 `GOOSE_PROVIDER` |
| `GOOSE_LEAD_TURNS` | 前多少轮使用 lead 模型，之后切到 worker 模型 | 整数 | 3 |
| `GOOSE_LEAD_FAILURE_THRESHOLD` | worker 模型连续失败多少次后回退到 lead 模型 | 整数 | 2 |
| `GOOSE_LEAD_FALLBACK_TURNS` | 进入 fallback 后，lead 模型继续使用多少轮 | 整数 | 2 |

这里的一个 _turn_ 指一整次 prompt-response 交互。默认配置下：

- 前 3 轮使用 lead 模型
- 第 4 轮开始使用 worker 模型
- 如果 worker 连续 2 轮表现不佳，则回退到 lead 模型
- lead 模型再处理 2 轮，然后重新切回 worker

goose CLI 会在会话开头显示当前的 lead 与 worker 模型。如果你没有为会话显式导出 `GOOSE_MODEL`，worker 模型会回退到 [配置文件](/zh-CN/docs/guides/config-files) 里的 `GOOSE_MODEL`。

**示例**

```bash
# 基础 lead/worker 配置
export GOOSE_LEAD_MODEL="o4"

# 高级 lead/worker 配置
export GOOSE_LEAD_MODEL="claude4-opus"
export GOOSE_LEAD_PROVIDER="anthropic"
export GOOSE_LEAD_TURNS=5
export GOOSE_LEAD_FAILURE_THRESHOLD=3
export GOOSE_LEAD_FALLBACK_TURNS=2
```

### Claude Extended Thinking

这一组变量用于控制 Claude 的 [extended thinking](https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking) 能力。它允许模型在生成正式响应前先进行内部推理。目前支持 Anthropic 和 Databricks provider。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `CLAUDE_THINKING_ENABLED` | 为 Claude 模型启用 extended thinking | 设为任意值即可启用 | 关闭 |
| `CLAUDE_THINKING_BUDGET` | Claude 内部推理过程允许使用的最大 token 数 | 正整数，最小 `1024` | `16000` |

**示例**

```bash
# 使用默认预算（16000）启用 extended thinking
export CLAUDE_THINKING_ENABLED=1

# 复杂任务使用更大预算
export CLAUDE_THINKING_ENABLED=1
export CLAUDE_THINKING_BUDGET=32000

# 更快返回结果，使用更小预算
export CLAUDE_THINKING_ENABLED=1
export CLAUDE_THINKING_BUDGET=8000
```

:::tip 查看 thinking 输出
如果你希望在 **CLI** 中看到 Claude 的 thinking 内容，还需要设置 `GOOSE_CLI_SHOW_THINKING=1`。在 **goose Desktop** 中，thinking 默认会显示在可折叠的 “Show reasoning” 区域里。
:::

### Planning Mode 配置

这一组变量控制 goose 的 [plan 功能](/zh-CN/docs/guides/creating-plans)。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_PLANNER_PROVIDER` | 规划模式使用的 provider | [查看可用 providers](/zh-CN/docs/getting-started/providers#available-providers) | 回退到 `GOOSE_PROVIDER` |
| `GOOSE_PLANNER_MODEL` | 规划模式使用的模型 | 模型名，例如 `"gpt-4"`、`"claude-sonnet-4-20250514"` | 回退到 `GOOSE_MODEL` |

**示例**

```bash
# 规划模式使用不同模型
export GOOSE_PLANNER_PROVIDER="openai"
export GOOSE_PLANNER_MODEL="gpt-4"
```

### Provider 重试配置

可为不同 provider 配置重试行为。

#### AWS Bedrock

| 变量 | 用途 | 默认值 |
|---------------------|-------------|---------|
| `BEDROCK_MAX_RETRIES` | 放弃前最多重试次数 | 6 |
| `BEDROCK_INITIAL_RETRY_INTERVAL_MS` | 首次重试前的等待时间（毫秒） | 2000 |
| `BEDROCK_BACKOFF_MULTIPLIER` | 每次重试后延迟增长倍数 | 2 |
| `BEDROCK_MAX_RETRY_INTERVAL_MS` | 最大重试等待时间（毫秒） | 120000 |

**示例**

```bash
export BEDROCK_MAX_RETRIES=10
export BEDROCK_INITIAL_RETRY_INTERVAL_MS=1000
export BEDROCK_BACKOFF_MULTIPLIER=3
export BEDROCK_MAX_RETRY_INTERVAL_MS=300000
```

#### Databricks

| 变量 | 用途 | 默认值 |
|---------------------|-------------|---------|
| `DATABRICKS_MAX_RETRIES` | 放弃前最多重试次数 | 3 |
| `DATABRICKS_INITIAL_RETRY_INTERVAL_MS` | 首次重试前的等待时间（毫秒） | 1000 |
| `DATABRICKS_BACKOFF_MULTIPLIER` | 每次重试后延迟增长倍数 | 2 |
| `DATABRICKS_MAX_RETRY_INTERVAL_MS` | 最大重试等待时间（毫秒） | 30000 |

**示例**

```bash
export DATABRICKS_MAX_RETRIES=5
export DATABRICKS_INITIAL_RETRY_INTERVAL_MS=500
export DATABRICKS_BACKOFF_MULTIPLIER=2
export DATABRICKS_MAX_RETRY_INTERVAL_MS=60000
```

## 会话管理 {#session-management}

这一组变量控制 goose 如何管理会话和上下文。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_CONTEXT_STRATEGY` | 控制上下文超限时如何处理 | `"summarize"`、`"truncate"`、`"clear"`、`"prompt"` | 交互模式下 `"prompt"`，headless 下 `"summarize"` |
| `GOOSE_MAX_TURNS` | 在没有用户输入的情况下允许的[最大轮数](/zh-CN/docs/guides/sessions/smart-context-management#maximum-turns) | 整数，例如 `10`、`50`、`100` | `1000` |
| `GOOSE_SUBAGENT_MAX_TURNS` | [subagent](/zh-CN/docs/guides/subagents) 允许的最大轮数。可被 recipe 或 subagent 调用里的 [`settings.max_turns`](/zh-CN/docs/guides/recipes/recipe-reference#settings) 覆盖 | 整数，例如 `25` | `25` |
| `CONTEXT_FILE_NAMES` | 指定自定义 [hint/context 文件名](/zh-CN/docs/guides/context-engineering/using-goosehints#custom-context-files) | 字符串 JSON 数组，例如 `["CLAUDE.md", ".goosehints"]` | `[".goosehints"]` |
| `GOOSE_DISABLE_SESSION_NAMING` | 禁用 AI 自动命名会话，避免后台模型调用，并保留默认标题 | `"1"`、`"true"`（大小写不敏感） | `false` |
| `GOOSE_PROMPT_EDITOR` | 使用的[外部编辑器](/zh-CN/docs/guides/goose-cli-commands#external-editor-mode) | 编辑器命令，例如 `"vim"`、`"code --wait"` | 未设置 |
| `GOOSE_CLI_THEME` | CLI Markdown 输出的[主题](/zh-CN/docs/guides/goose-cli-commands#themes) | `"light"`、`"dark"`、`"ansi"` | `"dark"` |
| `GOOSE_CLI_LIGHT_THEME` | light 模式下使用的 [bat theme](https://github.com/sharkdp/bat#adding-new-themes) | bat theme 名称 | `"GitHub"` |
| `GOOSE_CLI_DARK_THEME` | dark 模式下使用的 [bat theme](https://github.com/sharkdp/bat#adding-new-themes) | bat theme 名称 | `"zenburn"` |
| `GOOSE_CLI_NEWLINE_KEY` | 自定义 [CLI 输入换行快捷键](/zh-CN/docs/guides/goose-cli-commands#keyboard-shortcuts) | 单个字符，例如 `"n"`、`"m"` | `"j"`（即 `Ctrl+J`） |
| `GOOSE_CLI_SHOW_THINKING` | 在 CLI 中显示模型的 reasoning/thinking 输出 | 设为任意值即可启用 | 关闭 |
| `GOOSE_RANDOM_THINKING_MESSAGES` | 是否显示随机趣味等待提示 | `"true"`、`"false"` | `"true"` |
| `GOOSE_CLI_SHOW_COST` | 是否显示模型成本估算 | `"1"`、`"true"`（大小写不敏感） | `false` |
| `GOOSE_AUTO_COMPACT_THRESHOLD` | 当上下文使用到某个百分比时自动[压缩总结会话](/zh-CN/docs/guides/sessions/smart-context-management#automatic-compaction) | `0.0` 到 `1.0` 的浮点数；`0.0` 表示禁用 | `0.8` |
| `GOOSE_TOOL_CALL_CUTOFF` | 在总结旧工具输出前，保留多少次完整 tool call | 整数，例如 `5`、`10`、`20` | `10` |
| `GOOSE_MOIM_MESSAGE_TEXT` | 每一轮都注入到 goose [working memory](/zh-CN/docs/guides/using-persistent-instructions) 的固定文本 | 任意字符串 | 未设置 |
| `GOOSE_MOIM_MESSAGE_FILE` | 每一轮都注入到 goose [working memory](/zh-CN/docs/guides/using-persistent-instructions) 的文件内容，支持 `~/`，单文件最大 64 KB | 文件路径 | 未设置 |

**示例**

```bash
# 上下文超限时自动总结
export GOOSE_CONTEXT_STRATEGY=summarize

# 每次都询问用户（交互模式默认）
export GOOSE_CONTEXT_STRATEGY=prompt

# 更严格地限制自动轮数
export GOOSE_MAX_TURNS=5
export GOOSE_MAX_TURNS=25
export GOOSE_MAX_TURNS=100

# 调整默认 subagent 最大轮数
export GOOSE_SUBAGENT_MAX_TURNS=50

# 使用多个上下文文件
export CONTEXT_FILE_NAMES='["CLAUDE.md", ".goosehints", ".cursorrules", "project_rules.txt"]'

# 禁用自动命名
export GOOSE_DISABLE_SESSION_NAMING=true

# 外部编辑器
export GOOSE_PROMPT_EDITOR=vim

# CLI 主题
export GOOSE_CLI_THEME=ansi
export GOOSE_CLI_LIGHT_THEME="Solarized (light)"
export GOOSE_CLI_DARK_THEME="Dracula"

# 使用 Ctrl+N 换行
export GOOSE_CLI_NEWLINE_KEY=n

# 关闭随机 thinking 文案
export GOOSE_RANDOM_THINKING_MESSAGES=false

# 显示 reasoning
export GOOSE_CLI_SHOW_THINKING=1

# 显示成本
export GOOSE_CLI_SHOW_COST=true

# 60% 时自动 compact
export GOOSE_AUTO_COMPACT_THRESHOLD=0.6

# 保留更多完整 tool call
export GOOSE_TOOL_CALL_CUTOFF=20

# 固定注入工作记忆
export GOOSE_MOIM_MESSAGE_TEXT="IMPORTANT: Always run tests before committing changes."
export GOOSE_MOIM_MESSAGE_FILE="~/.goose/guardrails.md"
```

### 模型上下文限制覆盖 {#model-context-limit-overrides}

这组变量允许你手动覆盖模型的默认上下文窗口大小。对 [LiteLLM proxies](https://docs.litellm.ai/docs/providers/litellm_proxy) 和不符合 goose 预设模式的自定义模型尤其有用。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_CONTEXT_LIMIT` | 覆盖主模型上下文窗口 | 整数（token 数） | 模型默认值，或 `128000` |
| `GOOSE_INPUT_LIMIT` | 覆盖 ollama 请求的输入上限（映射到 `num_ctx`） | 整数（token 数） | 回退到 `GOOSE_CONTEXT_LIMIT` 或模型默认值 |
| `GOOSE_LEAD_CONTEXT_LIMIT` | 覆盖 [lead/worker 模式](/zh-CN/docs/tutorials/lead-worker) 中 lead 模型的上下文窗口 | 整数（token 数） | 回退到 `GOOSE_CONTEXT_LIMIT` 或模型默认值 |
| `GOOSE_WORKER_CONTEXT_LIMIT` | 覆盖 lead/worker 模式中 worker 模型的上下文窗口 | 整数（token 数） | 回退到 `GOOSE_CONTEXT_LIMIT` 或模型默认值 |
| `GOOSE_PLANNER_CONTEXT_LIMIT` | 覆盖 [planner 模型](/zh-CN/docs/guides/creating-plans) 的上下文窗口 | 整数（token 数） | 回退到 `GOOSE_CONTEXT_LIMIT` 或模型默认值 |

**示例**

```bash
# 主模型上下文上限
export GOOSE_CONTEXT_LIMIT=200000

# 覆盖 ollama 输入提示上限
export GOOSE_INPUT_LIMIT=32000

# lead/worker 使用不同上下文窗口
export GOOSE_LEAD_CONTEXT_LIMIT=500000
export GOOSE_WORKER_CONTEXT_LIMIT=128000

# planner 使用超大上下文
export GOOSE_PLANNER_CONTEXT_LIMIT=1000000
```

更多细节见 [Model Context Limit Overrides](/zh-CN/docs/guides/sessions/smart-context-management#model-context-limit-overrides)。

## 工具配置

这组变量控制 goose 如何处理[工具执行](/zh-CN/docs/guides/goose-permissions)和[工具管理](/zh-CN/docs/guides/managing-tools/)。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_MODE` | 控制工具执行策略 | `"auto"`、`"approve"`、`"chat"`、`"smart_approve"` | `"smart_approve"` |
| `GOOSE_TOOLSHIM` | 启用/禁用 tool call interpretation | `"1"`、`"true"`（大小写不敏感） | `false` |
| `GOOSE_TOOLSHIM_OLLAMA_MODEL` | 为 [tool call interpretation](/zh-CN/docs/experimental/ollama) 指定模型 | 模型名，例如 `llama3.2`、`qwen2.5` | 系统默认 |
| `GOOSE_CLI_MIN_PRIORITY` | 控制[工具输出](/zh-CN/docs/guides/managing-tools/adjust-tool-output)的最小展示优先级 | `0.0` 到 `1.0` 的浮点数 | `0.0` |
| `GOOSE_CLI_TOOL_PARAMS_TRUNCATION_MAX_LENGTH` | CLI 输出中，tool 参数截断前允许的最大长度（非 debug 模式） | 整数 | `40` |
| `GOOSE_DEBUG` | 启用 debug 模式，显示完整 tool 参数。也可在会话里用 `/r` [slash command](/zh-CN/docs/guides/goose-cli-commands#slash-commands) 切换 | `"1"`、`"true"`（大小写不敏感） | `false` |
| `GOOSE_SEARCH_PATHS` | 执行扩展命令时额外搜索可执行文件的目录 | 路径 JSON 数组，例如 `["/usr/local/bin", "~/custom/bin"]` | 仅系统 PATH |

**示例**

```bash
# 开启 tool interpretation
export GOOSE_TOOLSHIM=true
export GOOSE_TOOLSHIM_OLLAMA_MODEL=llama3.2
export GOOSE_MODE="auto"
export GOOSE_CLI_MIN_PRIORITY=0.2
export GOOSE_CLI_TOOL_PARAMS_TRUNCATION_MAX_LENGTH=100

# 为扩展额外添加工具目录
export GOOSE_SEARCH_PATHS='["/usr/local/bin", "~/custom/tools", "/opt/homebrew/bin"]'
```

这些路径会在扩展执行命令时被 prepend 到系统 PATH 前面，因此你不需要修改全局 PATH 就能让扩展找到自定义工具。

### 增强代码编辑 {#enhanced-code-editing}

这一组变量用于配置 Developer 扩展里 `str_replace` 工具的 [AI 增强代码编辑](/zh-CN/docs/guides/enhanced-code-editing)。三个变量都必须设置且非空才会启用。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_EDITOR_API_KEY` | 代码编辑模型使用的 API key | API key 字符串 | 无 |
| `GOOSE_EDITOR_HOST` | 代码编辑模型的 API endpoint | URL，例如 `"https://api.openai.com/v1"` | 无 |
| `GOOSE_EDITOR_MODEL` | 代码编辑模型名称 | 模型名，例如 `"gpt-4o"`、`"claude-sonnet-4"` | 无 |

**示例**

该功能兼容任何 OpenAI-compatible API endpoint，例如：

```bash
# OpenAI
export GOOSE_EDITOR_API_KEY="sk-..."
export GOOSE_EDITOR_HOST="https://api.openai.com/v1"
export GOOSE_EDITOR_MODEL="gpt-4o"

# Anthropic（通过 OpenAI-compatible proxy）
export GOOSE_EDITOR_API_KEY="sk-ant-..."
export GOOSE_EDITOR_HOST="https://api.anthropic.com/v1"
export GOOSE_EDITOR_MODEL="claude-sonnet-4-20250514"

# 本地模型
export GOOSE_EDITOR_API_KEY="your-key"
export GOOSE_EDITOR_HOST="http://localhost:8000/v1"
export GOOSE_EDITOR_MODEL="your-model"
```

## 安全与隐私 {#security-and-privacy}

这组变量用于控制安全特性、凭证存储和匿名使用数据。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_ALLOWLIST` | 控制允许加载哪些扩展 | [allowlist](/zh-CN/docs/guides/allowlist) 的 URL | 未设置 |
| `GOOSE_DISABLE_KEYRING` | 禁用系统 keyring 存储 secret | 设为任意值即可，例如 `"1"`、`"true"`、`"yes"` | 未设置 |
| `SECURITY_PROMPT_ENABLED` | 启用[prompt injection detection](/zh-CN/docs/guides/security/prompt-injection-detection) | `true` / `false` | `false` |
| `SECURITY_PROMPT_THRESHOLD` | prompt injection 检测灵敏度，越高越严格 | `0.01` 到 `1.0` | `0.8` |
| `SECURITY_PROMPT_CLASSIFIER_ENABLED` | 启用基于 ML 的高级 prompt injection 检测 | `true` / `false` | `false` |
| `SECURITY_PROMPT_CLASSIFIER_ENDPOINT` | ML 检测接口地址 | URL，例如 `"https://api.example.com/classify"` | 未设置 |
| `SECURITY_PROMPT_CLASSIFIER_TOKEN` | `SECURITY_PROMPT_CLASSIFIER_ENDPOINT` 的认证 token | 字符串 | 未设置 |
| `GOOSE_TELEMETRY_ENABLED` | 启用或禁用[匿名使用数据](/zh-CN/docs/guides/usage-data) | `true` / `false` | `false` |

**示例**

```bash
# 使用默认阈值启用 prompt injection 检测
export SECURITY_PROMPT_ENABLED=true

# 自定义更严格阈值
export SECURITY_PROMPT_ENABLED=true
export SECURITY_PROMPT_THRESHOLD=0.9

# 启用 ML 检测
export SECURITY_PROMPT_ENABLED=true
export SECURITY_PROMPT_CLASSIFIER_ENABLED=true
export SECURITY_PROMPT_CLASSIFIER_ENDPOINT="https://your-endpoint.com/classify"
export SECURITY_PROMPT_CLASSIFIER_TOKEN="your-auth-token"

# 控制匿名使用数据
export GOOSE_TELEMETRY_ENABLED=false
export GOOSE_TELEMETRY_ENABLED=true
```

:::tip
如果禁用了 keyring，或者 keyring 无法访问并且 goose [自动回退到文件存储](/zh-CN/docs/troubleshooting/known-issues#keyring-cannot-be-accessed-automatic-fallback)，secret 会保存在这里：

* macOS/Linux: `~/.config/goose/secrets.yaml`
* Windows: `%APPDATA%\Block\goose\config\secrets.yaml`
:::

### goose Desktop 的 macOS Sandbox

goose Desktop 支持可选的 [macOS sandbox](/zh-CN/docs/guides/sandbox)，基于 Apple 的 `sandbox-exec` 对文件访问、网络连接和进程执行施加限制。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|--------|---------|
| `GOOSE_SANDBOX` | 使用[可自定义的安全控制](/zh-CN/docs/guides/sandbox#configuration)启用 sandbox | `true` 或 `1` | `false` |

## 网络配置 {#network-configuration}

这组变量用于配置 goose 的代理和网络设置。

### HTTP Proxy

如果你处在企业防火墙或代理服务器后面，goose 支持标准 HTTP 代理环境变量。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `HTTP_PROXY` | HTTP 连接使用的代理 | URL，例如 `http://proxy.company.com:8080` | 无 |
| `HTTPS_PROXY` | HTTPS 连接使用的代理；与 `HTTP_PROXY` 同时设置时优先生效 | URL，例如 `http://proxy.company.com:8080` | 无 |
| `NO_PROXY` | 绕过代理的主机列表 | 逗号分隔列表，例如 `localhost,127.0.0.1,.internal.com` | 无 |

**示例**

```bash
# 为所有连接配置代理
export HTTPS_PROXY="http://proxy.company.com:8080"
export NO_PROXY="localhost,127.0.0.1,.internal,.local,10.0.0.0/8"

# 带认证的代理
export HTTPS_PROXY="http://username:password@proxy.company.com:8080"
export NO_PROXY="localhost,127.0.0.1,.internal"
```

你也可以通过操作系统网络设置配置代理。如果遇到连接问题，可参考 [Corporate Proxy or Firewall Issues](/zh-CN/docs/troubleshooting/known-issues#corporate-proxy-or-firewall-issues)。

## 可观测性 {#observability}

除了 goose 自带的[日志系统](/zh-CN/docs/guides/logs)，你还可以把 telemetry 导出到外部可观测性平台，用于监控、性能分析和生产诊断。

### 可观测性配置

你可以把 goose 输出到任意兼容 [OpenTelemetry](https://opentelemetry.io/docs/) 的平台。

要启用导出，先设置 collector endpoint：

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4318"
```

你还可以用 `OTEL_{SIGNAL}_EXPORTER` 分别控制 traces、metrics、logs：

| 变量模式 | 用途 | 取值 |
|---|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | 基础 OTLP endpoint（自动补 `/v1/traces` 等） | URL |
| `OTEL_EXPORTER_OTLP_{SIGNAL}_ENDPOINT` | 为某个 signal 单独指定 endpoint | URL |
| `OTEL_{SIGNAL}_EXPORTER` | 指定某个 signal 使用的 exporter 类型 | `otlp`、`console`、`none` |
| `OTEL_SDK_DISABLED` | 完全禁用 OTel 导出 | `true` |

也支持 `OTEL_SERVICE_NAME`、`OTEL_RESOURCE_ATTRIBUTES`、`OTEL_EXPORTER_OTLP_TIMEOUT` 等变量。完整列表见 [OTel 环境变量规范][otel-env]。

**示例**

```bash
# 所有信号都发到本地 collector
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4318"

# 只导出 traces，关闭 metrics 和 logs
export OTEL_TRACES_EXPORTER="otlp"
export OTEL_METRICS_EXPORTER="none"
export OTEL_LOGS_EXPORTER="none"
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4318"

# 将 traces 打到控制台
export OTEL_TRACES_EXPORTER="console"

# 只采样 10% traces
export OTEL_TRACES_SAMPLER="parentbased_traceidratio"
export OTEL_TRACES_SAMPLER_ARG="0.1"
```

[otel-env]: https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/

### Langfuse 集成

这组变量用于配置 [Langfuse observability 集成](/zh-CN/docs/tutorials/langfuse)。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `LANGFUSE_PUBLIC_KEY` | Langfuse 公钥 | 字符串 | 无 |
| `LANGFUSE_SECRET_KEY` | Langfuse 私钥 | 字符串 | 无 |
| `LANGFUSE_URL` | 自定义 Langfuse 地址 | URL | Langfuse 默认地址 |
| `LANGFUSE_INIT_PROJECT_PUBLIC_KEY` | 备用 Langfuse 公钥 | 字符串 | 无 |
| `LANGFUSE_INIT_PROJECT_SECRET_KEY` | 备用 Langfuse 私钥 | 字符串 | 无 |

## Recipe 配置

这一组变量控制 recipe 的发现和管理方式。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_RECIPE_PATH` | 额外搜索 recipe 的目录 | Unix 用冒号分隔，Windows 用分号分隔 | 无 |
| `GOOSE_RECIPE_GITHUB_REPO` | 搜索 recipe 的 GitHub 仓库 | `"owner/repo"` 格式，例如 `"block/goose-recipes"` | 无 |
| `GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS` | recipe 成功检查命令的全局超时 | 整数（秒） | recipe 自身默认值 |
| `GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS` | recipe `on_failure` 命令的全局超时 | 整数（秒） | recipe 自身默认值 |

**示例**

```bash
# 添加自定义 recipe 目录
export GOOSE_RECIPE_PATH="/path/to/my/recipes:/path/to/team/recipes"

# 配置 GitHub recipe 仓库
export GOOSE_RECIPE_GITHUB_REPO="myorg/goose-recipes"

# 设置全局 recipe 超时
export GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS=300
export GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS=60
```

## 开发与测试

这一组变量主要用于开发、测试和调试 goose 自身。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_PATH_ROOT` | 覆盖 goose 数据、配置和状态文件的根目录 | 绝对路径 | 平台默认路径 |

**默认目录**

- macOS: `~/Library/Application Support/Block/goose/`
- Linux: `~/.local/share/goose/`
- Windows: `%APPDATA%\Block\goose\`

设置后，goose 会在该目录下创建 `config/`、`data/`、`state/` 子目录。这适合隔离测试环境、运行多套配置、或在 CI/CD 中使用。

**示例**

```bash
# 临时测试环境
export GOOSE_PATH_ROOT="/tmp/goose-test"

# 某一条命令使用隔离环境
GOOSE_PATH_ROOT="/tmp/goose-isolated" goose run --recipe my-recipe.yaml

# CI/CD
GOOSE_PATH_ROOT="$(mktemp -d)" goose run --recipe integration-test.yaml

# 与开发工具配合
GOOSE_PATH_ROOT="/tmp/goose-test" ./scripts/goose-db-helper.sh status
```

## 由 goose 自动控制的变量

这组变量会在 goose 执行命令时自动设置。

| 变量 | 用途 | 取值 | 默认值 |
|----------|---------|---------|---------|
| `GOOSE_TERMINAL` | 表示命令正在由 goose 执行，可用于[定制 shell 行为](#customizing-shell-behavior) | 设置后值为 `"1"` | 未设置 |
| `AGENT` | 跨工具兼容的通用 agent 标识符，便于脚本识别当前是否由 goose 运行 | 设置后值为 `"goose"` | 未设置 |
| `AGENT_SESSION_ID` | 当前会话 ID，可用于[按会话隔离工作流](#using-session-ids-in-workflows)；STDIO 扩展和 Developer 扩展 shell 命令会自动拿到它 | 会话 ID 字符串，例如 `20260217_5` | 未设置 |

### 自定义 Shell 行为 {#customizing-shell-behavior}

有时你希望 goose 使用与日常终端不同的 shell 行为，例如：

- 跳过昂贵的 shell 初始化
- 屏蔽会卡住 agent 的交互式命令，例如 `git commit`
- 把 `find` 这类命令引导为更适合 agent 的工具，例如 `rg`
- 让跨 agent 的工具链能识别当前由 AI agent 执行
- 与 MCP server 或 LLM gateway 做更好的集成

这一点在 goose CLI 中尤其有用，因为 shell 命令会直接在你的终端环境里执行。

**工作方式**

goose 会自动设置 `GOOSE_TERMINAL` 和 `AGENT`，你可以在 shell 配置里据此分支：

1. 当 goose 执行命令时：
   - `GOOSE_TERMINAL` 自动为 `"1"`
   - `AGENT` 自动为 `"goose"`
2. 你的 shell 配置据此修改行为，而不会影响正常终端使用

**示例**

```bash
# ~/.zshenv 或 ~/.bashrc

# 阻止 goose 运行 git commit
if [[ -n "$GOOSE_TERMINAL" ]]; then
  git() {
    if [[ "$1" == "commit" ]]; then
      echo "❌ BLOCKED: git commit is not allowed when run by goose"
      return 1
    fi
    command git "$@"
  }
fi
```

```bash
# 引导 goose 使用更适合的工具
if [[ -n "$GOOSE_TERMINAL" ]]; then
  alias find="echo 'Use rg instead: rg --files | rg <pattern> for filenames, or rg <pattern> for content search'"
fi
```

```bash
# 使用通用约定识别 AI agent
if [[ -n "$AGENT" ]]; then
  echo "Running under AI agent: $AGENT"
  if [[ "$AGENT" == "goose" ]]; then
    echo "Detected goose - applying goose-specific settings"
  fi
fi
```

### 在工作流里使用 Session ID {#using-session-ids-in-workflows}

STDIO 扩展和 Developer 扩展的 shell 命令会自动收到 `AGENT_SESSION_ID`。这使你可以：

- 通过按会话隔离的 handoff 路径协调多个 tool call
- 按 session 隔离 worktree 或临时文件
- 将产物与会话历史做关联排查

下面是一个 recipe 使用 session ID 在不同步骤之间传递信息的例子：

```bash
# 创建按会话隔离的 handoff 目录
mkdir -p ~/Desktop/${AGENT_SESSION_ID}/handoff
echo "Results from step 1" > ~/Desktop/${AGENT_SESSION_ID}/handoff/output.txt

# 后续步骤可以读取同一路径
cat ~/Desktop/${AGENT_SESSION_ID}/handoff/output.txt
```

## 环境变量透传

Developer 扩展的 `shell` 工具会继承当前会话的环境变量。因此，依赖环境配置的工作流，例如登录态 CLI、构建流程等，都可以直接在 shell 工具里使用。

详细说明见 [Developer MCP 文档](/zh-CN/docs/mcp/developer-mcp)。

## 企业环境

在企业部署中，管理员常常需要通过环境变量统一控制基础设施、安全与监控行为。常见关注点如下：

**网络与基础设施**

- [网络配置](#network-configuration)：代理与网络访问控制
- [高级 Provider 配置](#advanced-provider-configuration)：指向内部 LLM endpoint、Databricks 或自定义部署
- [模型上下文限制覆盖](#model-context-limit-overrides)：为 LiteLLM proxy 或自定义模型设定上下文窗口

**安全与隐私**

- [安全与隐私](#security-and-privacy)：扩展加载、secret 存储、使用数据收集

**合规与监控**

- [可观测性](#observability)：将 telemetry 输出到 OTLP、Langfuse 等平台

## 备注

- 环境变量优先级高于配置文件。
- 对于 API key 等敏感变量，优先考虑使用系统 keyring，而不是直接写环境变量。
- 某些变量修改后需要重启 goose 才会生效。
- 如果启用了 planning mode，但没有设置 planner 专属变量，goose 会回退到主模型配置。
