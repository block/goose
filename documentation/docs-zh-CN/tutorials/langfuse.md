---
title: "集成 Langfuse"
description: "介绍如何把 goose 接入 Langfuse 观察执行表现。"
---

# 集成 Langfuse

本教程介绍如何把 goose 接入 Langfuse，用来观察请求轨迹、调试 agent 行为和分析性能。

## 什么是 Langfuse

[Langfuse](https://langfuse.com/) 是一个开源的 LLM engineering 平台，适合团队一起做：

- 监控
- 评估
- 调试
- prompt 与 trace 分析

## 接入前准备

你可以：

- 注册 Langfuse Cloud
- 或自行部署 Langfuse

之后拿到项目级 public key / secret key。

## 配置 goose

通过环境变量让 goose 能把 tracing 数据送到 Langfuse：

```bash
export LANGFUSE_INIT_PROJECT_PUBLIC_KEY=pk-lf-...
export LANGFUSE_INIT_PROJECT_SECRET_KEY=sk-lf-...
export LANGFUSE_URL=https://cloud.langfuse.com
```

如果你使用美国区域或自托管实例，则把 `LANGFUSE_URL` 换成对应地址。

## 使用方式

配置好之后，正常运行 goose 即可。Langfuse 会开始记录：

- goose 的请求轨迹
- agent 的动作链路
- 关键调用上下文

![goose 在 Langfuse 中的 trace 视图](https://langfuse.com//images/docs/goose-integration/goose-example-trace.png)

## 适合什么场景

Langfuse 特别适合：

- 团队协作排查 agent 问题
- 观察不同 prompt 的效果
- 分析工具和模型调用行为
- 做面向产品或平台层的 LLM 可观测性建设

如果你已经在用 Langfuse 管理其它 LLM 应用，把 goose 也接进去，通常能让整个 AI 工程体系更统一。
