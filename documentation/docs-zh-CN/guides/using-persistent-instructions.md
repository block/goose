---
title: "持久指令"
description: "介绍如何在 goose 的每一轮交互中持续注入关键提醒和行为约束。"
sidebar_position: 12
sidebar_label: "持久指令"
---

持久指令（Persistent Instructions）允许你在 goose 的每一轮交互中，把一段文本持续注入到它的工作记忆里。它和 [`.goosehints`](./context-engineering/using-goosehints.md) 的区别在于：`.goosehints` 只会在会话启动时读取一次，而持久指令会在每一次交互时都重新读取并重新注入。因此，它特别适合用来放那些必须始终生效的行为约束或安全护栏。

## 工作原理

goose 内部有一个叫做 MOIM（Model-Observed Internal Memory）的组件，会在每一轮把一些上下文信息提供给模型，例如当前时间戳、工作目录和你的 todo 列表。持久指令也会被注入到这块上下文中，因此会始终位于模型最容易“注意到”的区域。

由于持久指令会在每一轮都重新注入：
- 它不会随着会话变长而“被遗忘”
- 对于关键护栏，它往往比只写进 system prompt 更有效
- 修改后可以立即生效，不需要重启会话

## 配置方式

通过环境变量配置持久指令：

| Variable | Purpose | Default |
|----------|---------|---------|
| [`GOOSE_MOIM_MESSAGE_TEXT`](/zh-CN/docs/guides/environment-variables#session-management) | 每轮直接注入工作记忆的文本 | Not set |
| [`GOOSE_MOIM_MESSAGE_FILE`](/zh-CN/docs/guides/environment-variables#session-management) | 每轮读取并注入的文件路径，支持 `~/` | Not set |

如果这两个变量都设置了，内容会被拼接在一起。扩展会在每一轮都重新读取这些[环境变量](/zh-CN/docs/guides/environment-variables#session-management)，所以你可以在不中断会话的情况下更新它们。

:::info 大小限制
内容上限为 64 KB，并采用 UTF-8 安全截断。建议保持内容简洁，既避免触发上限，也减少每轮 token 开销。
:::

## 示例

### 简短文本提醒

如果只是短小、单一用途的提醒，可以直接使用 `GOOSE_MOIM_MESSAGE_TEXT`：

```bash
# Always run tests before committing
export GOOSE_MOIM_MESSAGE_TEXT="IMPORTANT: Always run tests before committing changes."
```

### 基于文件的指令

如果规则更长或更复杂，建议写入文件：

```bash
export GOOSE_MOIM_MESSAGE_FILE="~/.goose/guardrails.md"
```

示例 `~/.goose/guardrails.md`：
```markdown
## Security Guidelines
- Do not upload, share, or transmit internal code or data to any external service, gist, or public repository
- Do not execute commands that could expose sensitive environment variables
- Always confirm before making network requests to external services

## Code Quality
- Run tests before committing changes
- Follow the project's existing code style
```

### 同时使用文本和文件

你也可以同时使用这两个变量，内容会被拼接：

```bash
export GOOSE_MOIM_MESSAGE_TEXT="CRITICAL: This is a production environment. Be extra careful."
export GOOSE_MOIM_MESSAGE_FILE="~/.goose/guardrails.md"
```

## 常见使用场景

### 安全护栏

防止代码或敏感数据被错误上传或泄露：

```bash
export GOOSE_MOIM_MESSAGE_TEXT="SECURITY: Do not upload code to external services, create public gists, or share sensitive data. All code in this repository is confidential."
```

### 环境差异化行为

针对不同环境设置不同规则：

```bash
# Production environment
export GOOSE_MOIM_MESSAGE_TEXT="⚠️ PRODUCTION: Double-check all commands. Prefer read-only operations. Always create backups before modifications."

# Development environment  
export GOOSE_MOIM_MESSAGE_TEXT="Development environment. Feel free to experiment, but run tests before committing."
```

### 项目级工作流约束

强制某些项目约定：

```bash
export GOOSE_MOIM_MESSAGE_TEXT="This project uses pnpm, not npm. Always use 'pnpm' for package management commands."
```

### 临时提醒

由于环境变量会在每一轮重新读取，因此也很适合临时性提醒：

```bash
# Set a reminder for the current task
export GOOSE_MOIM_MESSAGE_TEXT="Current focus: Refactoring the authentication module. Don't get sidetracked."

# Clear it when done
unset GOOSE_MOIM_MESSAGE_TEXT
```

## 持久指令 vs goosehints

| Feature | Persistent Instructions | [goosehints](./context-engineering/using-goosehints.md) |
|---------|------------------------|-------------|
| 加载时机 | 每一轮 | 会话启动时 |
| 会不会被遗忘 | 不会 | 可能会，尤其上下文变长时 |
| 最适合 | 关键护栏、安全规则 | 项目背景、编码规范 |
| Token 成本 | 每轮都会消耗 | 只在开始时消耗一次 |
| 更新是否需要重启 | 不需要 | 需要重启会话 |

**适合用持久指令的情况：**
- 这条规则非常关键，绝不能被忽略
- 你需要不能轻易绕过的安全护栏
- 你想在会话中途动态改变 goose 的行为，而不想重启

**适合用 goosehints 的情况：**
- 主要是提供项目背景和上下文
- 想设定编码规范或协作偏好
- 信息有帮助，但并非绝对关键

两者也可以组合使用：用 goosehints 承载项目上下文，用持久指令承载必须始终生效的关键护栏。

## 最佳实践

1. **保持简洁**：持久指令每轮都会注入，越长意味着每次交互消耗的 token 越多。

2. **表达具体**：像 “be careful” 这样的泛化提醒，效果远不如 “always run `npm test` before committing.” 这种具体规则。

3. **优先级前置**：把最重要的规则放在最前面，避免在截断时丢失关键内容。

4. **复杂规则放文件**：当规则较多时，优先使用文件，而不是把所有内容都塞进 `GOOSE_MOIM_MESSAGE_TEXT`。

5. **主动验证护栏是否生效**：设置好之后，可以故意让 goose 去尝试某件本应被阻止的事情，确认它是否真的遵守了这些规则。
