---
title: "集成 MLflow"
description: "介绍如何用 MLflow 观察和评估 agent 表现。"
---

# 集成 MLflow

本教程介绍如何把 goose 接入 MLflow，对 session、工具调用和 agent 决策过程做 tracing 和评估。

## 什么是 MLflow

[MLflow](https://mlflow.org/) 是一个开源平台，用来管理机器学习和 AI 的完整生命周期。它的 tracing 能力可以帮助你观察：

- LLM 调用
- 工具使用
- agent 决策过程
- token 使用情况

## 为什么配合 goose

把 goose 接入 MLflow 后，你可以：

- 查看分层 trace
- 统计 token 成本
- 使用内建或自定义 scorer 做评估
- 把 prompt 和 trace 纳入统一平台管理

## 基本设置

先安装并启动 MLflow：

```bash
pip install mlflow
mlflow server --port 5000
```

然后配置 goose 把 OTLP 数据发到 MLflow：

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:5000"
export OTEL_EXPORTER_OTLP_HEADERS="x-mlflow-experiment-id=0"
```

如果你只想导出 trace，可继续设置：

```bash
export OTEL_TRACES_EXPORTER=otlp
export OTEL_METRICS_EXPORTER=none
export OTEL_LOGS_EXPORTER=none
```

## 启动方式

配置好后，正常运行：

```bash
goose session
```

再到 MLflow UI 的 Traces 页面查看 goose 的执行轨迹。

![goose 在 MLflow 中的 trace 视图](../../docs/assets/guides/mlflow-goose-tracing.png)

## 适合什么场景

MLflow 适合：

- 做实验记录和模型对比
- 追踪 agent 决策链路
- 为团队构建评估体系
- 把 goose 纳入已有的 AI / ML 平台

如果你本来就在用 MLflow 管理 AI 项目，把 goose 接进去通常是很自然的一步。
