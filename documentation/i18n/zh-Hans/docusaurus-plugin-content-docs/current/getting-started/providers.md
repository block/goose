---
title: Provider
description: 配置 goose 使用的 LLM Provider
---

# 配置 Provider

goose 通过 LLM Provider 获得推理能力。你可以选择 OpenAI、Anthropic、Tetrate、OpenRouter 等。

## 推荐流程

1. 选择 Provider
2. 填写凭证（API Key 或 OAuth）
3. 选择模型
4. 保存并测试

## 常见变量

- `GOOSE_PROVIDER`
- `GOOSE_MODEL`
- 各 Provider 的 API Key（如 `OPENAI_API_KEY`）

更多参数请查看[环境变量](/docs/guides/environment-variables)。

## 可用 Provider {#available-providers}

本页聚焦快速配置，完整 Provider 列表与能力差异可参考英文原文页面。

## 配置 Provider 与模型 {#configure-provider-and-model}

在桌面端或 CLI 中完成 Provider 认证后，选择一个可用模型并保存即可开始会话。

## 本地 LLM {#local-llms}

本地模型可通过兼容 API 的 Provider 接入，建议先验证模型可用性与上下文窗口配置。
