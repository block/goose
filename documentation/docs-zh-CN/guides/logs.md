---
title: "goose 日志系统"
description: "介绍如何查看、理解和使用 goose 的日志系统。"
sidebar_position: 65
---

# goose 日志系统

goose 使用一套统一的存储系统来保存对话和交互记录。所有会话和交互（包括 CLI 与 Desktop）都会**本地存储**在下列位置：

| **类型** | **Unix-like（macOS、Linux）** | **Windows** |
|----------|-------------------------------|-------------|
| **命令历史** | `~/.config/goose/history.txt` | `%APPDATA%\\Block\\goose\\data\\history.txt` |
| **会话记录** | `~/.local/share/goose/sessions/sessions.db` | `%APPDATA%\\Block\\goose\\data\\sessions\\sessions.db` |
| **系统日志** | `~/.local/state/goose/logs/` | `%APPDATA%\\Block\\goose\\data\\logs\\` |

:::info Privacy
goose 是本地应用，所有 goose 日志文件都存储在本机，不会被发送到外部服务器或第三方，因此 goose 数据仍由你自己掌控。需要注意的是，goose 代表你调用的 LLM 和工具，可能有它们自己的日志和隐私策略。
:::

## 命令历史 {#command-history}

goose 会跨聊天会话持久化保存命令历史，方便回顾以前用过的命令。

命令历史位于：

- Unix-like：`~/.config/goose/history.txt`
- Windows：`%APPDATA%\\Block\\goose\\data\\history.txt`

## 会话记录 {#session-records}

goose 会维护每个会话的记录，用来追踪对话历史和交互细节。

会话数据保存在 SQLite 数据库中：

- **Unix-like**：`~/.local/share/goose/sessions/sessions.db`
- **Windows**：`%APPDATA%\\Block\\goose\\data\\sessions\\sessions.db`

:::info Session Storage Migration
在 1.10.0 之前，goose 会把会话记录存成 `~/.local/share/goose/sessions/` 下的独立 `.jsonl` 文件。升级到 v1.10.0 或更高版本后，原有会话会自动导入数据库。旧的 `.jsonl` 文件仍会保留在磁盘上，但 goose 不再管理它们。
:::

这个数据库中包含所有已保存的会话数据，包括：

- 会话元数据（ID、名称、工作目录、时间戳）
- 对话消息（用户命令、助手回复、角色信息）
- 工具调用与结果（ID、参数、响应、成功 / 失败状态）
- token 使用统计
- 扩展数据与配置

会话 ID 采用 `YYYYMMDD_<COUNT>` 格式，例如 `20250310_2`。goose CLI 会在每个会话开始时输出当前会话 ID。要查看所有会话 ID，可以使用 [`goose session list` 命令](/zh-CN/docs/guides/goose-cli-commands#session-list-options)。

另外也可以参考[会话管理](/zh-CN/docs/guides/sessions/session-management)，了解如何查找和管理会话。

## 系统日志 {#system-logs}

goose 会为不同组件保存日志。CLI 和 server 日志会按日期分目录存储，并在两周后自动清理，防止占用过多磁盘空间。

如果启用了[提示注入检测](/zh-CN/docs/guides/security/prompt-injection-detection)，CLI 和 server 日志还会包含：

- 带唯一 ID 的安全发现（格式为 `SEC-{uuid}`）
- 与发现 ID 关联的用户决策（允许 / 拒绝）

:::info
扩展也可以选择在 `~/.local/state/goose/logs/` 下写入自己的子目录。具体结构取决于各扩展的实现。
:::

### Desktop 应用日志 {#desktop-application-log}

Desktop 应用本身会维护一份独立日志：

- macOS：`~/Library/Application Support/Goose/logs/main.log`
- Windows：`%APPDATA%\\Block\\goose\\logs\\main.log`

Desktop 遵循平台惯例保存自身运行日志和状态数据，但真正的会话内容仍使用标准的 [会话记录](#session-records)。这意味着无论你通过哪种界面使用 goose，你的会话历史都在同一套存储里。

### CLI 日志

CLI 日志位于：

- Unix-like：`~/.local/state/goose/logs/cli/`
- Windows：`%APPDATA%\\Block\\goose\\data\\logs\\cli\\`

日志按日期分子目录存放，例如 `cli/2025-11-13/`；超过两周的旧目录会被自动删除。

CLI 会话日志会包含：

- 工具调用与响应
- 命令执行细节
- 会话标识符
- 时间戳

CLI 日志还会记录扩展相关活动，例如：

- 工具初始化
- 工具能力与 schema
- 扩展特定操作
- 命令执行结果
- 错误消息和调试信息
- 扩展配置状态
- 扩展协议层信息

### Server 日志

Server 日志位于：

- Unix-like：`~/.local/state/goose/logs/server/`
- Windows：`%APPDATA%\\Block\\goose\\data\\logs\\server\\`

同样会按日期分目录，例如 `server/2025-11-13/`，并自动清理两周前的旧目录。

Server 日志记录的是 goose daemon（`goosed`）相关信息。它是运行在你本机上的本地服务进程，负责协调 CLI、扩展和 LLM 之间的通信。

Server 日志通常包含：

- Server 初始化细节
- JSON-RPC 通信日志
- Server capabilities
- 协议版本信息
- 客户端和服务端交互
- 扩展加载与初始化
- 工具定义和 schema
- 扩展指令与能力
- 调试级 transport 信息
- 系统能力与配置
- 操作系统信息
- 工作目录信息
- 传输层通信细节
- 消息解析与处理过程
- 请求 / 响应周期
- 错误状态和处理结果
- 扩展初始化序列

### LLM 请求日志

LLM 请求日志保存了发往模型 provider 的原始请求和响应数据：

- Unix-like：`~/.local/state/goose/logs/llm_request.*.jsonl`
- Windows：`%APPDATA%\\Block\\goose\\data\\logs\\llm_request.*.jsonl`

这些日志使用编号轮换机制，保留最近 10 个已完成请求（`llm_request.0.jsonl` 到 `llm_request.9.jsonl`）。每条记录都包含模型配置、输入 payload、响应数据和 token 使用信息。
