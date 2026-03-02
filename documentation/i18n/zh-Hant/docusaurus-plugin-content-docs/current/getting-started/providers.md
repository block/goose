---
title: Provider
description: 設定 goose 使用的 LLM Provider
---

# 設定 Provider

goose 透過 LLM Provider 取得推理能力。你可選擇 OpenAI、Anthropic、Tetrate、OpenRouter 等。

## 建議流程

1. 選擇 Provider
2. 填寫憑證（API Key 或 OAuth）
3. 選擇模型
4. 儲存並測試

## 常見變數

- `GOOSE_PROVIDER`
- `GOOSE_MODEL`
- 各 Provider 的 API Key（例如 `OPENAI_API_KEY`）

更多參數請見[環境變數](/docs/guides/environment-variables)。

## 可用 Provider {#available-providers}

本頁聚焦快速設定，完整 Provider 清單與能力差異可參考英文原文頁面。

## 設定 Provider 與模型 {#configure-provider-and-model}

在桌面版或 CLI 完成 Provider 驗證後，選擇可用模型並儲存即可開始工作階段。

## 本地 LLM {#local-llms}

本地模型可透過相容 API 的 Provider 接入，建議先驗證模型可用性與上下文視窗配置。
