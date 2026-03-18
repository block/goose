---
title: "管理项目"
description: "介绍如何在 goose 中切换、组织和管理项目上下文。"
sidebar_position: 15
---

# 管理项目

goose Projects 会自动追踪你的工作目录以及关联的会话，让你能够在多个代码库之间切换时保留完整上下文，并快速恢复工作。

在 goose 里，**project** 指的是你曾用 goose 工作过的某个目录记录。每次运行 goose 时，它都会自动把当前目录记录成一个 project，并保存：

- **Path**：项目目录的绝对路径
- **Last accessed**：你最近一次使用这个项目的时间
- **Last instruction**：你最近一次给 goose 的指令
- **Session ID**：与该项目关联的会话 ID，用于延续上下文

这些 project 数据会保存在 `~/.local/share/goose/projects.json`。

:::info CLI Only Feature
Projects 目前只在 goose CLI 中可用。Desktop 支持还在后续规划中。
:::

## 基本用法

**恢复最近一次工作的项目：**

```bash
goose project
```

**浏览所有项目：**

```bash
goose projects
```

:::tip
恢复项目时，你既可以继续之前的会话，也可以在该目录下开启一个全新的会话。
:::

完整命令语法和选项请参考 [CLI Commands Guide](/zh-CN/docs/guides/goose-cli-commands#project)。

## 工作流示例

下面以开发者 Sarah 一天内在多个项目之间切换为例：

### 上午：开发 API

```bash
cd ~/projects/ecommerce-api
goose session --name "api-auth-work"
```

*Sarah 让 goose 协助实现 JWT token refresh 逻辑。*

### 上午稍晚：修移动端 Bug

```bash
cd ~/projects/mobile-app
goose session
```

*Sarah 用 goose 协助排查登录页上的 iOS 崩溃问题。*

### 下午：管理后台

```bash
cd ~/projects/admin-dashboard
goose session --name "dashboard-ui"
```

*Sarah 开始编写用户管理界面组件。*

### 第二天：快速恢复

```bash
# 无论当前在哪个目录，都可以快速恢复最近一个项目
goose project
```

goose 会显示：

```text
┌ goose Project Manager
│
◆ Choose an option:
│  ○ Resume project with session: .../admin-dashboard
│    Continue with the previous session
│  ○ Resume project with fresh session: .../admin-dashboard
│    Change to the project directory but start a new session
│  ○ Start new project in current directory: /Users/sarah
│    Stay in the current directory and start a new session
└
```

### 之后：浏览所有项目

```bash
goose projects
```

goose 会显示：

```text
┌ goose Project Manager
│
◆ Select a project:
│  ○ 1  .../admin-dashboard (2025-01-07 09:15:30) [create user management interface]
│  ○ 2  .../mobile-app (2025-01-06 11:45:20) [login screen crashing on iOS]
│  ○ 3  .../ecommerce-api (2025-01-06 09:30:15) [JWT token refresh logic]
│  ○ Cancel
└
```

Sarah 可以直接看到最近项目的时间戳和上下文摘要，从而快速判断下一步该回到哪个项目继续。

## 价值

:::tip Time Savings
对于每天频繁切换代码库的开发者，Projects 能节省反复切换上下文时常见的 2 到 5 分钟损耗。
:::

- **减少上下文切换摩擦**：在多个项目之间快速来回，无需手动导航
- **保留工作上下文**：在原来的对话历史上继续工作
- **和 session 深度集成**：不同代码库之间也能保持清晰的连续性
