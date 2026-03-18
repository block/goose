---
title: "集成 Laminar"
description: "介绍如何把 goose 接入 Laminar 做可观测性分析。"
---

# 集成 Laminar

本教程介绍如何把 goose 接入 Laminar，对 session、LLM 调用和工具执行做 tracing 与可观测性分析。

## 什么是 Laminar

[Laminar](https://laminar.sh/) 是面向 AI agents 的开源 observability 平台，能跟踪：

- LLM 调用
- 工具执行
- 自定义函数
- agent 行为链路

## 为什么要和 goose 一起用

把 goose 接入 Laminar 后，你可以：

- 查看高信号 trace
- 回放某个 span，比较 prompt 或模型差异
- 从真实运行轨迹中提炼评估数据集
- 分析复杂 agent 行为模式

## 基本接入方式

goose 通过 OTLP/HTTP 导出 OpenTelemetry 数据，因此只需要把 exporter 指向 Laminar。

### Laminar Cloud

```bash
export LMNR_PROJECT_API_KEY=lmnr_proj_...
export OTEL_EXPORTER_OTLP_ENDPOINT="https://api.lmnr.ai"
export OTEL_EXPORTER_OTLP_HEADERS="authorization=Bearer ${LMNR_PROJECT_API_KEY}"
export OTEL_EXPORTER_OTLP_TIMEOUT=10000
```

### 自托管 Laminar

```bash
export LMNR_PROJECT_API_KEY=lmnr_proj_...
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:8000"
export OTEL_EXPORTER_OTLP_HEADERS="authorization=Bearer ${LMNR_PROJECT_API_KEY}"
```

如果你的自托管实例不要求认证，可以省略 `OTEL_EXPORTER_OTLP_HEADERS`。

## 启动方式

配置完环境变量后，正常启动 goose 即可。Laminar 会开始自动采集：

- session trace
- tool execution
- 模型调用链路

![goose 在 Laminar 中的 trace 视图](../../docs/assets/guides/laminar.png)

## 什么时候值得接入

如果你在这些场景里工作，Laminar 很有帮助：

- 排查 agent 为什么做出某个决策
- 对比不同 prompt / model 的效果
- 观察工具调用成本和路径
- 构建团队级 agent 评估体系

它更像是给 goose 增加了一套“可审计的运行轨迹”，让问题不再只能靠猜。
