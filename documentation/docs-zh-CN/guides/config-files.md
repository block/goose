---
title: "配置文件"
description: "说明 goose 的配置文件结构、存储位置和常见修改方式。"
sidebar_position: 85
---

# 配置概览

goose 使用 YAML [配置文件](#configuration-files) 来管理设置和扩展。主配置文件位于：

- macOS/Linux：`~/.config/goose/config.yaml`
- Windows：`%APPDATA%\\Block\\goose\\config\\config.yaml`

这些配置文件可以用来设置默认行为、配置语言模型、指定工具权限以及管理扩展。虽然许多设置也能通过[环境变量](/zh-CN/docs/guides/environment-variables)完成，但配置文件更适合做持久化偏好设置。

## 配置文件 {#configuration-files}

- **`config.yaml`**：provider、model、extensions 和通用设置
- **`permission.yaml`**：通过 `goose configure` 配置的工具权限级别
- **`secrets.yaml`**：当 goose 使用[文件型密钥存储](#security-considerations)时，保存 API key 和其他密钥
- **`permissions/tool_permissions.json`**：运行时权限决策（自动维护）
- **`prompts/`**：自定义 [prompt templates](/zh-CN/docs/guides/prompt-templates)

除了直接编辑配置文件外，许多设置也能在 goose Desktop 和 goose CLI 中管理：

- **goose Desktop**：在 `Settings` 页面和底部工具栏中调整
- **goose CLI**：运行 `goose configure` 命令

## 全局设置

下面这些设置可以直接放在 `config.yaml` 根级别：

| Setting | Purpose | Values | Default | Required |
|---|---|---|---|---|
| `GOOSE_PROVIDER` | 主 [LLM provider](/zh-CN/docs/getting-started/providers) | `"anthropic"`、`"openai"` 等 | 无 | 是 |
| `GOOSE_MODEL` | 默认模型 | 模型名称，例如 `"claude-3.5-sonnet"`、`"gpt-4"` | 无 | 是 |
| `GOOSE_TEMPERATURE` | 模型输出随机性 | `0.0` 到 `1.0` 的浮点数 | 取决于模型 | 否 |
| `GOOSE_MAX_TOKENS` | 每次模型响应允许的最大 token 数 | 正整数 | 取决于模型 | 否 |
| `GOOSE_MODE` | [工具执行行为](/zh-CN/docs/guides/goose-permissions) | `"auto"`、`"approve"`、`"chat"`、`"smart_approve"` | `"auto"` | 否 |
| `GOOSE_MAX_TURNS` | 无用户输入时允许的[最大轮数](/zh-CN/docs/guides/sessions/smart-context-management#maximum-turns) | 整数，例如 `10`、`50`、`100` | `1000` | 否 |
| `GOOSE_LEAD_PROVIDER` | [lead/worker 模式](/zh-CN/docs/guides/environment-variables)下的 lead model provider | 与 `GOOSE_PROVIDER` 相同 | 回退到 `GOOSE_PROVIDER` | 否 |
| `GOOSE_LEAD_MODEL` | lead/worker 模式的 lead model | 模型名称 | 无 | 否 |
| `GOOSE_PLANNER_PROVIDER` | [planning mode](/zh-CN/docs/guides/creating-plans) 的 provider | 与 `GOOSE_PROVIDER` 相同 | 回退到 `GOOSE_PROVIDER` | 否 |
| `GOOSE_PLANNER_MODEL` | planning mode 使用的模型 | 模型名称 | 回退到 `GOOSE_MODEL` | 否 |
| `GOOSE_TOOLSHIM` | 启用工具解释 | `true/false` | `false` | 否 |
| `GOOSE_TOOLSHIM_OLLAMA_MODEL` | 工具解释使用的模型 | 模型名称，例如 `"llama3.2"` | 系统默认值 | 否 |
| `GOOSE_CLI_MIN_PRIORITY` | CLI 工具输出详细程度 | `0.0` 到 `1.0` 的浮点数 | `0.0` | 否 |
| `GOOSE_CLI_THEME` | CLI Markdown 响应的[主题](/zh-CN/docs/guides/goose-cli-commands#themes) | `"light"`、`"dark"`、`"ansi"` | `"dark"` | 否 |
| `GOOSE_CLI_LIGHT_THEME` | 浅色模式下的自定义高亮主题 | [bat theme 名称](https://github.com/sharkdp/bat#adding-new-themes) | `"GitHub"` | 否 |
| `GOOSE_CLI_DARK_THEME` | 深色模式下的自定义高亮主题 | [bat theme 名称](https://github.com/sharkdp/bat#adding-new-themes) | `"zenburn"` | 否 |
| `GOOSE_CLI_SHOW_COST` | 在 CLI 中显示估算 token 成本 | `true/false` | `false` | 否 |
| `GOOSE_ALLOWLIST` | 允许扩展的 URL | 合法 URL | 无 | 否 |
| `GOOSE_RECIPE_GITHUB_REPO` | Recipes 的 GitHub 仓库 | `"org/repo"` 格式 | 无 | 否 |
| `GOOSE_AUTO_COMPACT_THRESHOLD` | 自动压缩会话的阈值百分比 | `0.0` 到 `1.0` 的浮点数 | `0.8` | 否 |
| `SECURITY_PROMPT_ENABLED` | 启用[提示注入检测](/zh-CN/docs/guides/security/prompt-injection-detection) | `true/false` | `false` | 否 |
| `SECURITY_PROMPT_THRESHOLD` | 提示注入检测的灵敏度阈值 | `0.01` 到 `1.0` 的浮点数 | `0.8` | 否 |
| `SECURITY_PROMPT_CLASSIFIER_ENABLED` | 启用基于 ML 的提示注入检测 | `true/false` | `false` | 否 |
| `SECURITY_PROMPT_CLASSIFIER_ENDPOINT` | ML 检测服务的分类接口 URL | URL | 无 | 否 |
| `SECURITY_PROMPT_CLASSIFIER_TOKEN` | `SECURITY_PROMPT_CLASSIFIER_ENDPOINT` 的认证 token | 字符串 | 无 | 否 |
| `GOOSE_TELEMETRY_ENABLED` | 启用[匿名使用数据](/zh-CN/docs/guides/usage-data)收集 | `true/false` | `false` | 否 |

其他[环境变量](/zh-CN/docs/guides/environment-variables)在 `config.yaml` 中通常也能使用。

## 配置示例

```yaml
# 模型配置
GOOSE_PROVIDER: "anthropic"
GOOSE_MODEL: "claude-4.5-sonnet"
GOOSE_TEMPERATURE: 0.7

# 规划配置
GOOSE_PLANNER_PROVIDER: "openai"
GOOSE_PLANNER_MODEL: "gpt-4"

GOOSE_MODE: "smart_approve"
GOOSE_TOOLSHIM: true
GOOSE_CLI_MIN_PRIORITY: 0.2

# Recipe 配置
GOOSE_RECIPE_GITHUB_REPO: "block/goose-recipes"

# Search Path 配置
GOOSE_SEARCH_PATHS:
  - "/usr/local/bin"
  - "~/custom/tools"
  - "/opt/homebrew/bin"

# 安全配置
SECURITY_PROMPT_ENABLED: true

extensions:
  developer:
    bundled: true
    enabled: true
    name: developer
    timeout: 300
    type: builtin

  memory:
    bundled: true
    enabled: true
    name: memory
    timeout: 300
    type: builtin
```

## 扩展配置

扩展配置在 `extensions` 键下。每个扩展都可以拥有以下字段：

```yaml
extensions:
  extension_name:
    bundled: true/false
    display_name: "Name"
    enabled: true/false
    name: "extension_name"
    timeout: 300
    type: "builtin"/"stdio"

    # stdio 扩展的附加配置
    cmd: "command"
    args: ["arg1", "arg2"]
    description: "text"
    env_keys: []
    envs: {}
```

## Search Path 配置

扩展有时需要执行系统里的外部命令。默认情况下，goose 使用系统自己的 `PATH` 环境变量。你也可以在配置文件中额外补充搜索目录：

```yaml
GOOSE_SEARCH_PATHS:
  - "/usr/local/bin"
  - "~/custom/tools"
  - "/opt/homebrew/bin"
```

这些路径会在运行扩展命令时被追加到系统 PATH 前面，因此无需修改全局 PATH，也能让 goose 找到你的自定义工具。

## 可观测性配置

你可以把 goose 的 telemetry 导出到兼容 [OpenTelemetry](https://opentelemetry.io/docs/) 的平台。环境变量会覆盖这些配置，并且还支持按 signal 维度做更细粒度的控制。更多细节见[环境变量指南](/zh-CN/docs/guides/environment-variables)。

| Setting | Purpose | Values | Default |
|---|---|---|---|
| `otel_exporter_otlp_endpoint` | OTLP endpoint URL | URL，例如 `http://localhost:4318` | 无 |
| `otel_exporter_otlp_timeout` | 导出超时时间（毫秒） | 整数（毫秒） | `10000` |

```yaml
otel_exporter_otlp_endpoint: "http://localhost:4318"
otel_exporter_otlp_timeout: 20000
```

## Recipe 命令配置

你也可以配置[自定义 slash commands](/zh-CN/docs/guides/context-engineering/slash-commands)，让它们直接运行你创建的 recipe。这里填写命令名（不带前导 `/`）以及 recipe 文件路径：

```yaml
slash_commands:
  - command: "run-tests"
    recipe_path: "/path/to/recipe.yaml"
  - command: "daily-standup"
    recipe_path: "/Users/me/.local/share/goose/recipes/standup.yaml"
```

## 配置优先级

设置的优先级从高到低如下：

1. 环境变量
2. 配置文件
3. 默认值

## 安全注意事项 {#security-considerations}

- 不要把敏感信息（API key、token 等）直接写进配置文件
- 优先使用系统 keyring（macOS 上是 keychain）来保存密钥；只要系统支持，这就是推荐方案
- 如果 goose 退回到文件型密钥存储，密钥会被单独写入明文 `secrets.yaml`。这通常发生在：
  - 你的环境没有桌面 keyring 服务，例如 headless server、CI/CD 或容器
  - 你显式关闭了 keyring（见 [GOOSE_DISABLE_KEYRING](/zh-CN/docs/guides/environment-variables#security-and-privacy)）
  - goose 无法访问 keyring，因此自动回退到文件型密钥存储

如需排查 keyring 故障和自动回退行为，可参考[已知问题](/zh-CN/docs/troubleshooting/known-issues#keyring-cannot-be-accessed-automatic-fallback)。

## 检查当前配置

修改配置后，需要重启 goose 才会生效。可以用下面命令确认当前生效配置：

```bash
goose info -v
```

这个命令会显示所有当前激活的设置及其实际值。

## 继续阅读

- **[多模型配置](/zh-CN/docs/guides/multi-model/)**：了解多模型选择策略
- **[环境变量](/zh-CN/docs/guides/environment-variables)**：用环境变量管理配置
- **[使用扩展](../getting-started/using-extensions.md)**：查看扩展配置的更多细节
