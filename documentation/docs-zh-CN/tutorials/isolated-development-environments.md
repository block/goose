---
title: "隔离开发环境"
description: "说明如何把 goose 放进隔离环境中运行，降低对本机环境的影响。"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# 隔离开发环境

本教程介绍如何通过 **[Container Use MCP](https://github.com/dagger/container-use)** 与 goose 配合，构建隔离开发环境。借助这套方式，你的开发实验会同时隔离在 git 分支和容器中，因此可以放心尝试，而不影响主系统状态。

Container Use MCP 为隔离开发提供了一种非常适合 agent 的工作方式，它依赖 Docker、copy-on-write 文件系统等机制，让环境隔离、可重置、可并行。

## 概览

**[Container Use MCP](https://github.com/dagger/container-use)** server 提供了与 goose 深度集成的容器化开发环境。你可以借此：

- 在独立 git 分支中工作
- 在容器里运行代码，而不影响本机环境
- 需要时快速清空重来
- 在不同实验和项目之间保持清晰隔离
- 并行推进多项尝试

:::info 已经在用 Devcontainers？
如果你本来就在 Docker 容器里做开发（例如 VS Code Remote-Containers），可以直接看如何[在现有容器里运行扩展](/zh-CN/docs/tutorials/goose-in-docker#running-extensions-in-docker-containers)。
:::

## 前置条件

- 已安装并运行 Docker（也可以是 [Podman](https://docs.dagger.io/ci/integrations/podman)、[NerdCtl](https://docs.dagger.io/ci/integrations/nerdctl/) 或 [Container](https://docs.dagger.io/ci/integrations/apple-container/)）
- 已安装并配置 Git
- 已安装并配置 goose

## 安装与配置

完整安装和配置步骤见 [Container Use Extension](/zh-CN/docs/mcp/container-use-mcp) 教程。

## 使用方式

启用这个扩展之后，你就可以直接在与 goose 的对话里表达“我要在隔离环境中工作”。

### 功能实验

你可以直接说：

```text
I want to experiment with adding a new feature, but I want to do it in an isolated environment so I don't affect my main codebase.
```

goose 会自动：

1. 为你的工作创建一个新的 git 分支
2. 搭起容器化环境
3. 确保所有改动都和宿主机隔离

### 尝试不同实现路线

```text
Let me try a completely different approach to this algorithm. Can you set up an isolated environment where I can experiment?
```

### 学习新技术

```text
I want to try out this new framework, but I don't want to install all its dependencies on my main system.
```

## 主要收益

- **安全性**：不会破坏本地主要开发环境
- **可复现**：容器环境更稳定
- **易清理**：实验失败后直接丢弃容器和分支
- **适合并行探索**：多个方案互不干扰
- **版本可追踪**：所有改动都落在独立 git 分支里
- **易于回滚**：失败实验可以直接丢弃

## 常见工作流

### 功能开发

1. 和 goose 讨论一个新功能
2. 请求创建隔离开发环境
3. goose 创建分支和容器
4. 在里面开发和测试
5. 成功就合并；失败就直接丢弃

### 依赖探索

1. 让 goose 帮你试一个新库或新工具
2. 在隔离容器里安装依赖和运行实验
3. 验证兼容性和可用性
4. 再决定是否把它带回主项目

### 重构

1. 为大规模重构请求隔离环境
2. 在分支和容器中安全修改
3. 完整测试后再决定是否合并
4. 如果失败，直接回滚即可

## 故障排查

### 常见问题

**Docker 没有运行**
- 确认 Docker Desktop 已安装并启动
- 使用 `docker info` 检查 daemon 状态

**权限问题**
- 确认当前用户有执行 Docker 命令的权限
- 在 Linux 上，可把用户加入 docker 组：`sudo usermod -aG docker $USER`

**Git 问题**
- 确认 Git 已配置 user name / email
- 确认当前目录本身就是一个 Git 仓库

### 获取帮助

如果仍然遇到问题：

1. 查看 **[Container Use GitHub 仓库](https://github.com/dagger/container-use)** 文档
2. 逐项确认前置条件是否都满足
3. 到 [Discord 社区](https://discord.gg/goose-oss) 寻求帮助

## 下一步

启用 container-use 后，你已经具备用更安全方式探索项目的基础。挑一个你之前一直不敢轻易试的任务，让 goose 为你搭起一个安全、隔离的实验环境。

记住：有了隔离环境，就不存在“把主代码库改坏”的失败实验，只有成本更低的学习和迭代。
