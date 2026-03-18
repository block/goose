---
title: "自定义发行版"
description: "介绍如何构建和分发定制化的 goose 发行版。"
sidebar_position: 60
---

# 自定义发行版

goose 从设计上就支持 fork 和定制。你可以基于它制作自己的“发行版（distro）”，预先配置特定 provider、打包内置扩展、替换品牌元素，或者为你的组织和用户群体定制专属工作流。

## 你可以自定义什么

| 目标 | 复杂度 |
|------|--------|
| 预配置模型 / Provider | 低 |
| 添加自定义 AI Provider（声明式 JSON，无需代码） | 低 |
| 打包自定义 MCP 扩展 | 中 |
| 修改 system prompt | 低 |
| 自定义 Desktop 品牌元素（图标、名称、颜色） | 中 |
| 基于 REST API 或 ACP 构建全新 UI | 高 |
| 用 recipes 构建引导式工作流 | 低 |

## 从哪里开始

完整指南放在仓库根目录，因为构建自定义发行版通常需要直接在代码层面操作：

👉 **[CUSTOM_DISTROS.md](https://github.com/block/goose/blob/main/CUSTOM_DISTROS.md)**

这份指南涵盖：

- **架构概览**：说明 goose 的各层（UI → server → core）如何协同工作
- **只改配置的定制方式**：环境变量、`config.yaml`、`init-config.yaml`
- **扩展打包**：把 MCP server 作为内置扩展或通过 recipe 分发
- **品牌定制**：替换图标、应用名称、system prompt
- **新界面开发**：通过 REST API 或 Agent Client Protocol（ACP）集成
- **自定义 AI Provider**：声明式 JSON provider 或自行实现 `Provider` trait
- **Recipes 与 subagents**：分发预配置工作流
- **许可证与贡献说明**：确保符合 Apache 2.0 许可要求

## 快速示例：预装本地模型的 goose

最简单的自定义发行版，往往只是设置一组默认环境变量：

```bash
export GOOSE_PROVIDER=ollama
export GOOSE_MODEL=qwen3-coder:latest
```

或者创建一个在首次运行时应用的 `init-config.yaml`：

```yaml
GOOSE_PROVIDER: ollama
GOOSE_MODEL: qwen3-coder:latest
```

如果你要覆盖更多场景，例如企业内部 API key 分发、面向特定用户群体的发行版，或者自定义 UI，请直接查看 [完整指南](https://github.com/block/goose/blob/main/CUSTOM_DISTROS.md)。
