---
title: 设置 LLM 速率限制
sidebar_label: LLM 速率限制
sidebar_position: 60
---

速率限制是指在特定时间窗口内，限制用户或应用向 LLM API 发送请求的数量。LLM Provider 会设置这类限制，用于管理资源并防止滥用。

由于 goose 执行任务的速度很快，你可能会遇到 Provider 施加的速率限制。如果你经常触发限制，可以考虑升级当前 LLM 套餐以获得更高配额，或者选择 [内置速率限制能力的 Provider](/zh-CN/docs/getting-started/providers#configure-provider-and-model)：

:::info
goose 对这两类 Provider 都支持自动配置流程，会引导你完成 OAuth 账号创建和安全 API Key 生成。
:::

- **Tetrate Agent Router**：面向 AI 模型的统一 API 网关，覆盖 Claude、Gemini、GPT 以及开源权重模型等。它提供企业级路由、内置速率限制和自动故障切换，是开发者接入多模型的捷径。

  你可以在 [router.tetrate.ai](https://router.tetrate.ai/dashboard) 管理账号。

- **OpenRouter**：为 LLM 提供统一接口，让你可以在一个计费方案下自动选择和切换不同 Provider。使用 OpenRouter 时，你可以直接使用免费模型，或为付费模型购买额度。

  你可以在 [openrouter.ai](https://openrouter.ai) 管理账号。

当 goose 通过这些 Provider 发送请求时，Provider 会在需要时自动切换模型，以尽量避免因速率限制造成中断。
