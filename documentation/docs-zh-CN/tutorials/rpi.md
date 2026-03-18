---
title: "Research → Plan → Implement 模式"
description: "介绍如何在复杂软件项目中使用 RPI 这种 context engineering 技术。"
---

import CodeBlock from '@theme/CodeBlock';
import researchDoc from '../../static/files/thoughts/research/2025-12-22-llm-tool-selection-strategy.raw';
import planDoc from '../../static/files/thoughts/plans/2025-12-23-remove-tool-selection-strategy.raw';

# Research → Plan → Implement 模式

大多数人使用 AI agent 时，往往一上来就直接执行任务：“重构这段代码”“删除这个功能”“加一个新特性”。这种方式对小改动或小型代码库有时确实有效，但一旦任务复杂起来，通常就会开始失控。

**[RPI（Research, Plan, Implement）](https://github.com/humanlayer/advanced-context-engineering-for-coding-agents/blob/main/ace-fca.md)** 是 HumanLayer 提出的工作方式。它用速度换来更高的清晰度、可预测性和正确性。

本教程会通过一个真实示例，演示 RPI 是怎么运作的。看完之后，你就可以在自己的代码库上复用同样的流程。

## 前置条件

<details>
<summary>1. 导入 RPI Recipes</summary>

把下面这段命令复制到终端里执行。它会下载核心 RPI recipes 及其 subrecipes，并保存到全局 recipe 目录。

```sh
mkdir -p ~/.config/goose/recipes/subrecipes

curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/rpi-research.yaml -o ~/.config/goose/recipes/rpi-research.yaml
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/rpi-plan.yaml -o ~/.config/goose/recipes/rpi-plan.yaml
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/rpi-implement.yaml -o ~/.config/goose/recipes/rpi-implement.yaml
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/rpi-iterate.yaml -o ~/.config/goose/recipes/rpi-iterate.yaml

curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/subrecipes/rpi-codebase-locator.yaml -o ~/.config/goose/recipes/subrecipes/rpi-codebase-locator.yaml
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/subrecipes/rpi-codebase-analyzer.yaml -o ~/.config/goose/recipes/subrecipes/rpi-codebase-analyzer.yaml
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/subrecipes/rpi-pattern-finder.yaml -o ~/.config/goose/recipes/subrecipes/rpi-pattern-finder.yaml
```
</details>

<details>
<summary>2. 添加自定义 Slash Commands</summary>

recipes 导入后，为了方便在 session 中快速调用，请为下面这些 recipes 配置[自定义 slash commands](/zh-CN/docs/guides/context-engineering/slash-commands)：

| Recipe | Slash Command |
|---|---|
| RPI Research Codebase | `research_codebase` |
| RPI Create Plan | `create_plan` |
| RPI Implement Plan | `implement_plan` |
| RPI Iterate | `iterate_plan` |
</details>

## RPI 工作流

在 goose 中，我们会通过 recipes 配合 slash commands，使用一套结构化 RPI 工作流来系统性地处理复杂代码库变更：

1. `research_codebase`：记录当前现状，不给意见
2. `create_plan`：设计变更方案，明确阶段和成功标准
3. `implement_plan`：逐阶段执行计划并持续验证
4. `iterate_plan`：如果需要，迭代修订现有计划

```md
┌─────────────────────────────────────────────────────────────────────────────-┐
│                           RPI WORKFLOW                                       │
├─────────────────────────────────────────────────────────────────────────────-┤
│                                                                              │
│  /research_codebase "topic"                                                  │
│       │                                                                      │
│       ├──► Spawns parallel sub-agents:                                       │
│       │    • find_files (rpi-codebase-locator)                               │
│       │    • analyze_code (rpi-codebase-analyzer)                            │
│       │    • find_patterns (rpi-pattern-finder)                              │
│       │                                                                      │
│       └──► Output: thoughts/research/YYYY-MM-DD-HHmm-topic.md                │
│                                                                              │
│  /create_plan "feature/task"                                                 │
│       │                                                                      │
│       ├──► Reads research docs                                               │
│       ├──► Asks clarifying questions                                         │
│       ├──► Proposes design options                                           │
│       │                                                                      │
│       └──► Output: thoughts/plans/YYYY-MM-DD-HHmm-description.md             │
│                                                                              │
│  /implement_plan "plan path"                                                 │
│       │                                                                      │
│       ├──► Executes phase by phase                                           │
│       ├──► Runs verification after each phase                                │
│       ├──► Updates checkboxes in plan                                        │
│       │                                                                      │
│       └──► Working code                                                      │
│                                                                              │
│  /iterate_plan "plan path" + feedback                                        │
│       │                                                                      │
│       ├──► Researches only what changed                                      │
│       ├──► Updates the plan surgically                                       │
│       │                                                                      │
│       └──► Updated plan                                                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────-┘
```

所有 RPI 输出物都会落在一个固定位置：

```md
thoughts/
├── research/
│   └── YYYY-MM-DD-HHmm-topic.md
└── plans/
    └── YYYY-MM-DD-HHmm-description.md
```

## 本次任务

这篇教程中的目标，是从一个大型代码库中移除一个现有功能。

这不是一个小改动。这个功能会影响：

- 核心 Rust 代码
- TypeScript
- 配置
- 测试
- 文档

这类任务里，agent 常常会出问题，不是因为它不会做，而是因为改动横跨的上下文太多，无法安全地“直接开改”。

所以，我们不直接进实现，而是先走 RPI。

## Session 1：Research

先计划后实现，已经是常见做法。但如果没有先研究现状，计划往往会建立在错误假设之上。所以在 RPI 里，我们从 research 开始。

我用 `/research_codebase` 命令，再配上一个自然语言描述的主题：

```text
/research_codebase "look through the cloned goose repo and research how the LLM Tool Discovery is implemented"
```

这个命令会调用 **[RPI Research Codebase](https://raw.githubusercontent.com/block/goose/refs/heads/main/documentation/src/pages/recipes/data/recipes/rpi-research.yaml)** recipe，它的要求非常严格：

- 只记录当前存在的内容
- 不提出修改建议
- 不批评现有实现
- 不开始规划

这个 recipe 会自动拉起 3 个并行 subagent：

- `find_files`：定位相关文件在哪里
- `analyze_code`：完整阅读这些文件，并记录它们是如何工作的
- `find_patterns`：在仓库中查找类似功能或现有约定

这些 subagent 会独立运行并汇总结果，你不需要手动调度。

:::info 中途纠偏
在 goose 开始 research 之后，我意识到它研究的是泛化意义上的 “tool discovery”，但我真正想删的是一个叫 Tool Selection Strategy 的特定功能。于是我停掉它，换了一个更准确的研究主题重新跑了一次。

这不算失败。恰恰相反，这就是 research 阶段存在的意义。要是我一开始直接让 goose “删掉 LLM Tool Discovery 功能”，它很可能把我们其他工具发现机制一起删掉。好在这种偏差在 research 阶段被发现时，代价还很低，也很好修正。
:::

`/research_codebase` 最终输出了一份结构化的研究文档：

<details>
<summary>./thoughts/research/2025-12-22-llm-tool-selection-strategy.md</summary>

<CodeBlock language="markdown">
{researchDoc}
</CodeBlock>

</details>

这是一份体量不小的结构化文档，里面包含：

- Git 元数据
- 文件与行号引用
- 流程说明
- 关键组件
- 开放问题

你可以把它理解成“该功能当前实现状态的技术地图”。这个阶段故意什么都不改，目标只有一个：形成共享理解。

作为 human in the loop，你一定要认真审阅 research 结果，因为它会直接决定后续 plan 的质量。

## Session 2：Plan

research 完成之后，就进入 planning 阶段。

:::tip Sessions
最好每个阶段都开一个新 session，让 LLM 只专注当前任务。每个 session 只做一个目标。
:::

```text
/create_plan a removal of the Tool Selection Strategy feature
```

**[RPI Create Plan](https://raw.githubusercontent.com/block/goose/refs/heads/main/documentation/src/pages/recipes/data/recipes/rpi-plan.yaml)** recipe 会先读取刚刚生成的 research 文档。

然后它会做三件关键事情：

1. **提出澄清问题**

   例如：
   - 是彻底删除，还是先废弃？
   - 配置清理应该怎么处理？
   - 是否要重新生成 OpenAPI 产物？
   - 相关测试分布在哪些位置？

2. **给出设计选项**

   在存在多个合理方案时，goose 会把它们列出来让你选。

3. **生成分阶段实施计划**

最终输出会是一份细致的计划：

<details>
<summary>thoughts/plans/2025-12-23-remove-tool-selection-strategy.md</summary>

<CodeBlock language="markdown">
{planDoc}
</CodeBlock>

</details>

这份计划里会包含：

- 10 个明确阶段
- 精确文件路径
- 明确指出要删什么的代码片段
- 自动化成功标准
- 人工验证步骤
- 可用于跟踪进度的 checkbox

到这一步，plan 就成了 single source of truth。注意这里最重要的转折是：我们从“理解现状”过渡到了“做决策”，但依然还没有碰代码。

这份计划必须清晰到别人也能执行，因为 implementation 会发生在一个全新的 session 里。也就是说，plan 本身必须携带足够上下文，才能支持后续真正落地。

同样地，你也必须在这里进行人工 review。如果 plan 有问题，不需要推倒重来，你可以直接用 **[RPI Iterate Plan](https://raw.githubusercontent.com/block/goose/refs/heads/main/documentation/src/pages/recipes/data/recipes/rpi-iterate.yaml)**（`/iterate_plan`）描述哪里不对。goose 会读取现有 plan，只针对需要重想的部分补 research、提出定向修订，然后更新原计划。

## Session 3：Implement

只有在 research 和 planning 都完成之后，才进入 implementation。此时你把 plan 文档路径交给 goose：

```text
/implement_plan thoughts/plans/2025-12-23-remove-tool-selection-strategy.md
```

**[RPI Implement Plan](https://raw.githubusercontent.com/block/goose/refs/heads/main/documentation/src/pages/recipes/data/recipes/rpi-implement.yaml)** recipe 故意设计得“很无聊”。事实上，我第一次跑它的时候，中途都睡着了。Implementation 本来就应该机械。如果它看起来还很“有创造性”，那通常意味着上游 research 或 plan 还不够扎实。只要 plan 够稳，我建议你让 goose 自己干活，除非 plan 里有必须手动参与的步骤。

它会完整读取 plan，按阶段顺序执行，在每个阶段后跑验证，并且把 plan 文件里的 checkbox 直接更新掉。

这一点非常有用，因为我的上下文窗口在执行到一半时被打满了，但 goose 通过 plan 里的状态更新，能够在 compact 之后从正确位置继续往下做。

## 最终结果

这个任务总共有 10 个阶段，跨了 32 个文件。Research 阶段花了 9 分钟，Plan 阶段花了 4 分钟，Implement 阶段花了 39 分钟，总共大约 52 分钟。这里面既包括 goose 自己执行和测试，也包括我回答澄清问题的时间。

这绝对不算快。**但是**，当我提交 [这个 PR](https://github.com/block/goose/pull/6250) 时，构建通过了，而且独立的 Code Review Agent 连一条评论都没提。这说明整个过程的质量非常高。

如果完全不用 AI，这件事大概率要花我好几个小时，因为这个功能复杂且深度耦合。相反，如果让 AI 一开始就直接开改，我几乎可以肯定它会中途漂移、改坏东西。

所以，RPI 的确比“让 AI 直接干”更慢，但质量非常高，这个取舍是值得的。

## 什么时候该用 RPI

对于简单任务，RPI 可能有点用力过猛，尤其因为它并不是一个快流程。但如果你要做的是一个横跨多个文件的复杂任务，它会是很合适的选择。

RPI 特别适合：

- 重构
- 迁移
- 新功能引入
- 大版本升级
- 事故后的系统清理
- 大规模文档重构

你可以直接拿它去试自己的代码库。
