---
title: 实验功能
hide_title: true
description: 仍在快速迭代中的实验性能力与项目
---

import Card from "@site/src/components/Card";
import styles from "@site/src/components/Card/styles.module.css";

<h1 className={styles.pageTitle}>实验功能</h1>
<p className={styles.pageDescription}>
  goose 是一个持续演进的开源项目。这里收录的实验性能力和项目仍在开发中，可能还不够稳定，也不一定适合直接用于生产环境，但它们展示了 goose 在 AI 自动化上的一些前沿方向。
</p>

:::note
实验功能列表会随着 goose 的开发进展持续变化。有些能力会逐步转正，有些则可能继续调整甚至移除。
:::

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>🧪 实验性能力</h2>
  <div className={styles.cardGrid}>
    <Card
      title="Ollama Tool Shim"
      description="为原生不支持 tool calling 的模型启用工具调用能力。"
      link="/zh-CN/docs/experimental/ollama"
    />
    <Card
      title="通过安全隧道实现移动访问"
      description="让移动设备通过安全隧道远程访问 goose。"
      link="/zh-CN/docs/experimental/mobile-access"
    />
    <Card
      title="goose for VS Code 扩展"
      description="通过 ACP 在 VS Code 中直接与 goose 交互。"
      link="/zh-CN/docs/experimental/vs-code-extension"
    />
    <Card
      title="goose Mobile"
      description="了解 goose Mobile 相关的实验性方向和配套能力。"
      link="/zh-CN/docs/experimental/goose-mobile"
    />
    <Card
      title="Using goose in ACP Clients"
      description="在 Zed 等 ACP-compatible 客户端中原生使用 goose。"
      link="/docs/guides/acp-clients"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📝 精选博客文章</h2>
  <div className={styles.cardGrid}>
    <Card
      title="Finetuning Toolshim Models for Tool Calling"
      description="了解原生不支持 tool calling 的模型为何需要 toolshim，以及这类模型调优的设计背景。"
      link="/blog/2025/04/11/finetuning-toolshim"
    />
    <Card
      title="AI, But Make It Local With goose and Ollama"
      description="了解如何把 goose 与 Ollama 结合，构建更本地化的 AI 工作流。"
      link="/blog/2025/03/14/goose-ollama"
    />
    <Card
      title="Community-Inspired Benchmarking: The goose Vibe Check"
      description="查看开源模型在 goose benchmark 中的表现，以及 toolshim 的效果。"
      link="/blog/2025/03/31/goose-benchmark"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>💬 反馈与支持</h2>
  <div className={styles.cardGrid}>
    <Card
      title="GitHub Issues"
      description="报告问题、提交功能建议，或参与实验性能力的开发讨论。"
      link="https://github.com/block/goose/issues"
    />
    <Card
      title="Discord Community"
      description="加入社区讨论实验性能力，反馈体验，并与其他用户交流。"
      link="https://discord.gg/goose-oss"
    />
  </div>
</div>
