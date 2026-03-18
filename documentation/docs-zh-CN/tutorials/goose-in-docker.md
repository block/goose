---
title: "Docker 中使用 goose"
description: "介绍如何在 Docker 容器里运行 goose，或在现有容器里运行扩展以适配 devcontainer 工作流。"
---

# Docker 中使用 goose

这篇教程覆盖两个常见 Docker 场景：

1. **把 goose 本身运行在 Docker 中**
2. **goose 运行在宿主机，但扩展运行在容器里**

## 场景一：在 Docker 中运行 goose

你可以直接在 Docker 容器中从源码构建并运行 goose。这样做不仅能提供更强的隔离和安全性，也能提升环境一致性和可移植性。比如你平时主要在 macOS 上开发，但要排查一个只出现在 Ubuntu 上的问题，那么用 Docker 就能很轻松地复现。

开始前，你通常需要根据自己的场景调整 [`Dockerfile` 和 `docker-compose.yml`](https://github.com/block/goose/tree/main/documentation/docs/docker)：

- **必须做的**：在 `docker-compose.yml` 中把 API key、provider 和 model 以环境变量形式显式传进去，因为在 Docker 中跑 Ubuntu 时，keyring 通常不可用。官方示例里使用的是 Google Gemini。
- **可选**：把 `Dockerfile` 的基础镜像换成你想要的 Linux 发行版。官方示例是 Ubuntu，但你也可以改成 CentOS、Fedora 或 Alpine。
- **可选**：在 `docker-compose.yml` 里挂载你个人的 goose 配置和 hints 文件，这样容器里可以直接复用你自己的设置。

:::tip 自动化替代方案
如果你不想手动维护这些容器，可以直接看 [Container-Use MCP 扩展](/zh-CN/docs/mcp/container-use-mcp)，让 goose 通过对话帮你创建和管理容器。
:::

配置好凭据后，可以先构建镜像：

```bash
docker-compose -f documentation/docs/docker/docker-compose.yml build
```

然后运行容器并连接进去：

```bash
docker-compose -f documentation/docs/docker/docker-compose.yml run --rm goose-cli
```

进入容器后，执行：

```bash
goose configure
```

如果它问你是否把 API key 保存进 keyring，选择 `No`，因为你已经通过环境变量把 key 传进来了。

接着你可以再运行一次 `goose configure`，补上你需要的[扩展](/zh-CN/docs/getting-started/using-extensions)。

之后就可以启动会话：

```bash
goose session
```

这时你就已经可以在容器里使用带有扩展的 goose 了。

## 场景二：在容器中运行扩展 {#running-extensions-in-docker-containers}

如果 goose 仍然跑在本机，但你希望扩展在某个 devcontainer 或运行中容器里执行，可以使用：

```bash
goose session --container <container-id-or-name>
```

这样 `config.yaml` 里配置的扩展会在指定容器中运行。

### Requirements

- 容器里必须能找到扩展依赖和命令路径
- 如果要在容器里跑内建扩展，容器内部也要先[安装](/zh-CN/docs/getting-started/installation) goose CLI
- 如果命令找不到，通常需要改成容器内的完整路径

### 示例

```bash
# 交互式会话，使用 config.yaml 里已有扩展
goose session --container my-dev-container

# 非交互模式运行
goose run --container my-dev-container --text "your instructions here"

# 指定一个扩展在容器中运行
goose session --container 4c76a1beed85 --with-extension "uvx mcp-server-fetch"

# 如果容器里找不到命令，可以改用完整路径
goose session --container 4c76a1beed85 --with-extension "/root/.local/bin/uvx mcp-server-fetch"
```

## 什么时候适合用 Docker

- 你需要隔离依赖和环境差异
- 你在调试跨平台问题
- 你想把 goose 工作流和 devcontainer / Codespaces 结合
- 你希望扩展直接在目标运行环境中执行

如果你不想手工管理容器，也可以考虑 [Container-Use MCP 扩展](/zh-CN/docs/mcp/container-use-mcp)，让 goose 帮你自动编排容器。
