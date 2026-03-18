---
title: "Lead/Worker 多模型协作"
description: "介绍如何用两个 LLM 组成 lead/worker 模式来完成任务。"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Lead/Worker 多模型协作

goose 支持一种 lead/worker 多模型配置，让两个不同模型协作完成任务：一个擅长思考，另一个擅长快速执行。这种方式解决了一个非常典型的痛点：强模型能力好但昂贵，便宜模型速度快但复杂任务容易失手。lead/worker 模式的目标，就是把两者的优点组合起来。

<details>
  <summary>Lead/Worker 模式演示</summary>
  <iframe
    class="aspect-ratio"
    src="https://youtube.com/embed/ZyhUTsChFUw"
    title="Lead/Worker 模式配置与设置讲解"
    frameBorder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
    allowFullScreen
  ></iframe>
</details>

## 工作机制

lead/worker 本质上是一套智能交接系统。`lead` 模型（例如 GPT-4 或 Claude Opus）负责开场阶段的规划和整体推理；当方向确定之后，goose 会把任务交给 `worker` 模型（例如 GPT-4o-mini 或 Claude Sonnet）继续执行具体步骤。

如果执行过程中出现偏差，例如 worker 模型反复犯错、方向跑偏，goose 会自动检测到异常，并把 lead 模型重新拉回来接管；等状态恢复之后，再把任务交还给 worker。

## Turn-based 切换

这里的 **turn** 指一轮完整交互：你的输入 + 模型的响应。goose 会按 turn 控制模型切换：

- **前几轮**（默认 3 轮）交给 lead 模型
- **后续轮次**切到 worker 模型
- **出现连续失败时**触发 fallback，重新切回 lead
- **恢复稳定后**再把 session 交还给 worker

## 快速示例

```bash
export GOOSE_LEAD_MODEL="gpt-4o"
export GOOSE_MODEL="gpt-4o-mini"
export GOOSE_PROVIDER="openai"
```

这样 goose 会先用 `gpt-4o` 负责前 3 轮规划，然后交给 `gpt-4o-mini` 执行。如果 worker 连续失败两次，goose 会临时切回 lead 模型做 2 轮兜底，再尝试把任务交回 worker。

## 配置方式

:::tip
请先确认你已经[把目标 LLM 配置进 goose](/zh-CN/docs/getting-started/providers)。
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   1. 点击 goose Desktop 窗口底部的模型名称
   2. 点击 **Lead/Worker Settings**
   3. 勾选 **Enable lead/worker mode**
   4. 在下拉框里分别选择 **Lead Model** 和 **Worker Model**
   5. （可选）调整 **initial lead turns**、**failure threshold** 和 **fallback turns**
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    CLI 最少只需要设置 `GOOSE_LEAD_MODEL` 这个[环境变量](/zh-CN/docs/guides/environment-variables#leadworker-model-configuration)：

    ```bash
    export GOOSE_LEAD_MODEL="gpt-4o"
    ```

    goose 会默认把常规 `GOOSE_MODEL` 当作 worker model。

    如果你想更细粒度控制，还可以配置这些可选环境变量：

    ```bash
    export GOOSE_LEAD_PROVIDER="anthropic"
    export GOOSE_LEAD_TURNS=5
    export GOOSE_LEAD_FAILURE_THRESHOLD=3
    export GOOSE_LEAD_FALLBACK_TURNS=2
    ```

    配置完成后，新的 CLI 和 Desktop session 都会使用 lead/worker 模式。
  </TabItem>
</Tabs>

## 什么算失败？

goose 判断失败并不只看 API 层报错，而是会尝试识别真正的任务失败。通常在以下情况会触发 fallback：

- 生成了损坏代码，例如语法错误、工具调用失败、文件缺失
- 遇到权限问题
- 被用户明确纠正，例如 “这不对”“重新来一次”

像 timeout、认证异常或服务抖动这类纯技术性故障，通常不会触发 fallback；goose 会自己重试。

## 为什么值得用

- **节省成本**：把便宜模型用于大量执行
- **提高速度**：复杂思考交给强模型，重复执行交给快模型
- **跨 provider 组合**：例如 Claude 做推理，OpenAI 做执行
- **适合长开发 session**：减少单一模型疲劳和性能波动

## 最佳实践

如果你刚开始用，默认设置通常已经够用。但你也可以这样调整：

- 把 `GOOSE_LEAD_TURNS` 提高到 5-7，让前期规划更充分
- 把 `GOOSE_LEAD_FAILURE_THRESHOLD` 降到 1，让 goose 更快触发纠偏
- 选择一个足够轻量、足够快的 worker 模型，例如 Claude Haiku 或 GPT-4o-mini

如果你想观察模型切换行为，可以打开下面的日志：

```bash
export RUST_LOG=goose::providers::lead_worker=info
```

## 与 Planning Mode 的关系

lead/worker 模式可以理解为 [goose CLI `/plan` 命令](/zh-CN/docs/guides/creating-plans) 的自动化替代方案。你甚至可以给 lead/worker 和 planning mode 分别指定不同模型，例如：

```bash
export GOOSE_PROVIDER="openai"
export GOOSE_MODEL="gpt-4o-mini"

export GOOSE_LEAD_MODEL="o1-preview"
export GOOSE_PLANNER_MODEL="gpt-4o"
```

如果你希望由专门的强模型先生成一份你可以审核的完整策略，再开始执行，那就用 **planning mode**。如果你更想要一种不中断的自动化开发体验，让 goose 在后台智能切换规划和执行模型，那么就用 **lead/worker 模式**。两者也完全可以结合：先用 `/plan` 做重大决策，再让 lead/worker 去完成后续实施。
