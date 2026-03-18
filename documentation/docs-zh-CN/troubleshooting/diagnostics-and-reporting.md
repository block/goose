---
title: Diagnostics and Reporting
sidebar_label: Diagnostics and Reporting
description: 使用内置诊断、报告 bug，并通过 goose 的集成支持能力请求新功能。
---

import Tabs from "@theme/Tabs";
import TabItem from "@theme/TabItem";
import { PanelLeft, Bug } from "lucide-react";

goose 内置了几项能力，帮助你收集排障信息、提交问题以及提出新功能需求。这一页主要覆盖诊断包、bug 报告和功能请求的使用方式。

| 功能 | 用途 | 位置 | 输出 |
| --- | --- | --- | --- |
| **Diagnostics** | 生成排障数据 | 聊天输入框工具栏 | 包含系统信息、日志和会话数据的 ZIP 文件 |
| **Report a Bug** | 提交 bug | 聊天输入框工具栏，或 `Settings → App → Help & feedback` | 打开 GitHub issue 模板 |
| **Request a Feature** | 提交功能建议 | `Settings → App → Help & feedback` | 打开 GitHub issue 模板 |

## Diagnostics 系统 {#diagnostics-system}

诊断功能会生成一个排障包，里面包含系统信息、会话数据、配置文件和近期日志。遇到难以定位的问题，或者需要给维护者和社区提供上下文时，这个包非常有用。

### 生成诊断包

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 在一个活动会话中，找到底部工具栏里的 <Bug className="inline" size={16} /> 图标。
    2. 点击诊断按钮。
    3. 在弹窗里确认会被收集的数据内容。
    4. 点击 `Download` 生成并保存诊断包。
    5. 文件会保存为 `diagnostics_{session_id}.zip`。

    :::tip
    只有在存在活动会话时才会显示诊断按钮，因为生成诊断包需要当前会话 ID。
    :::
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    可以使用 session diagnostics 命令生成排障包。完整命令说明见 [CLI 命令指南](/zh-CN/docs/guides/goose-cli-commands)。

    ```sh
    # 为指定会话生成诊断包
    goose session diagnostics --session-id <session_id>

    # 交互式选择会话
    goose session diagnostics

    # 输出到指定位置
    goose session diagnostics --session-id <session_id> --output /path/to/diagnostics.zip
    ```

    如果你还不知道会话 ID，可以先列出可用会话：

    ```sh
    goose session list
    ```
  </TabItem>
</Tabs>

### 诊断包里有什么

诊断 ZIP 文件通常会包含以下内容：

```text
diagnostics_abc123def.zip
├── logs/
│   ├── goose-2024-01-15.jsonl
│   ├── goose-2024-01-14.jsonl
│   └── ...
├── session.json
├── config.yaml
└── system.txt
```

**适合生成诊断包的时机：**

- 遇到崩溃或异常行为
- 遇到看不懂的错误提示
- 性能异常或响应变慢
- 提交 bug 前，希望附带技术信息

**诊断包通常会包含：**

- **系统信息**：应用版本、操作系统、架构和生成时间
- **会话数据**：当前对话及相关历史
- **配置文件**：你的[配置文件](/zh-CN/docs/guides/config-files)（如果存在）
- **日志文件**：近期应用日志，便于定位问题

:::warning 隐私提醒
诊断包会包含你的会话消息和系统信息。如果会话中含有 API Key、个人信息或专有代码，分享前请先自行检查内容。
:::

## Bug 报告

bug 报告功能会直接打开结构化的 GitHub issue 模板，帮助你一次性提供必要信息。

### 提交 bug

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 在活动会话中找到底部工具栏的 <Bug className="inline" size={16} /> 图标。
    2. 点击诊断按钮。
    3. 选择 `File Bug on GitHub`。
    4. 浏览器会打开 GitHub，并自动带上 bug 模板。
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    直接访问下面的 GitHub 地址即可：

    ```text
    https://github.com/block/goose/issues/new?template=bug_report.md
    ```
  </TabItem>
</Tabs>

## 功能请求

如果你想建议新能力或改进现有行为，可以使用功能请求入口。

### 提交功能建议

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏。
    2. 点击 `Settings`。
    3. 进入 `App` 标签。
    4. 下拉到 `Help & feedback` 区域。
    5. 点击 `Request a Feature`。
    6. 浏览器会打开 GitHub，并带上功能请求模板。
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    直接访问：

    ```text
    https://github.com/block/goose/issues/new?template=feature_request.md
    ```
  </TabItem>
</Tabs>

## 通过 “Ask goose” 辅助恢复错误

在 goose Desktop 中，某些错误发生时（例如扩展激活失败），通知里会出现 `Ask goose` 按钮。你可以用它把错误详情直接发给 goose，快速获得诊断建议：

1. 当错误出现时，点击通知里的 `Ask goose`
2. 错误详情会自动带入聊天输入
3. goose 会给出排查建议和可能的修复方向

## 更多调试手段

如果 Diagnostics 还不足以定位问题，可以继续看：

- [会话和系统日志](/zh-CN/docs/guides/logs)：查看更细粒度的调试日志
- [环境变量与可观测性配置](/zh-CN/docs/guides/environment-variables)：配置遥测输出，用于性能分析和监控
