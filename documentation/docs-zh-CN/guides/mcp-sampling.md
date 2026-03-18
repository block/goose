---
title: "MCP Sampling 扩展"
description: "介绍如何把 MCP servers 转成能利用 goose AI 进行分析和决策的智能 agents。"
sidebar_position: 54
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import ContentCardCarousel from '@site/src/components/ContentCardCarousel';
import mcpSampling from '@site/blog/2025-12-04-mcp-sampling/mcp-sampling.png';

# MCP Sampling 扩展

MCP Sampling 能把扩展从“只会提供原始数据的工具”升级成“具备分析和判断能力的智能代理”。扩展不再只是把信息原封不动返回给 goose，而是可以借助 goose 的 AI 能力给出专家级建议、做上下文分析，甚至创造新的交互方式。

这个功能在 goose 中默认启用，无需额外配置。任何支持 sampling 的 MCP server 扩展都会自动拿到 goose 当前使用的 LLM。这意味着：

- goose 用户可以获得更贴合扩展能力边界的回答
- 开发者可以在自己的 MCP server 中加入 sampling 支持，让扩展能力更强

你可以试试 [Council of Mine](/zh-CN/docs/mcp/council-of-mine-mcp) 扩展，感受一下 MCP sampling 的实际效果。

:::info
[MCP Sampling](https://modelcontextprotocol.io/specification/draft/client/sampling) 是 Model Context Protocol 的一个特性。
:::

## MCP Sampling 如何工作

MCP Sampling 允许扩展在执行任务时向 goose 的 AI 请求帮助。当扩展需要分析数据、做智能决策或理解自然语言时，它可以发起一个 sampling 请求，请 goose 的 AI 协助完成。goose 会用自身的 AI 能力处理这个请求，再把结果返回给扩展。

这样一来，扩展就能给出更有上下文的专业回答，或实现新的交互模式。下面这个例子展示了数据库扩展如何把自身的领域知识和 goose 的 AI 分析结合起来，给出更具体的诊断建议：

<Tabs>
  <TabItem value="without" label="Without Sampling">
    1. 你问 goose：“我的数据库性能出了什么问题？”

    2. goose 调用数据库工具

    3. 数据库工具把原始指标返回给 goose：
       ```
       Query times: 2.3s, 1.8s, 5.2s, 0.3s, 8.1s
       Table sizes: users (1M rows), orders (5M rows)
       Indexes: 3 on users, 1 on orders
       ```

    4. goose 给出比较泛化的建议：
       ```
       Your database seems slow. Some queries are taking over 5 seconds. You might need more indexes.
       ```

  </TabItem>
  <TabItem value="with" label="With Sampling">
    1. 你问 goose：“我的数据库性能出了什么问题？”

    2. goose 调用数据库工具

    3. 数据库工具先拿到原始指标：
       ```
       Query times: 2.3s, 1.8s, 5.2s, 0.3s, 8.1s
       Table sizes: users (1M rows), orders (5M rows)
       Indexes: 3 on users, 1 on orders
       ```

       然后它会：
       - 结合自己的领域知识（查询模式、表关系、数据库类型）去问 goose 的 AI：“结合这些指标以及这个 PostgreSQL 数据库里的 JOIN 模式，真正的问题是什么？”
       - 再把 AI 增强后的结论返回给 goose

    4. goose 最终给出更有针对性的建议：
       ```
       Your orders table is missing an index on customer_id which is causing the 5-8 second delays in your JOIN queries. The slow queries all involve customer lookups. Run: `CREATE INDEX idx_orders_customer ON orders(customer_id);`
       ```

  </TabItem>
</Tabs>

### 适用场景

MCP Sampling 非常适合这些能力：

- **智能文档工具**：结合上下文解释代码
- **智能搜索**：过滤并排序结果
- **数据库分析器**：输出明确的优化建议
- **多视角分析**：由扩展生成并综合多个 AI 视角

## 给扩展开发者

如果你想在自己的扩展里加入 MCP Sampling，可以查看 [构建自定义扩展](/zh-CN/docs/tutorials/custom-extensions) 教程，了解 MCP server 如何利用 goose 的 AI 能力。
