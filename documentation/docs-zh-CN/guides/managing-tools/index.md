---
title: 工具管理
hide_title: true
description: 控制并配置为 goose 工作流提供能力的工具和扩展
---

import Link from "@docusaurus/Link";
import Card from "@site/src/components/Card";
import styles from "@site/src/components/Card/styles.module.css";

<h1 className={styles.pageTitle}>工具管理</h1>
<p className={styles.pageDescription}>
  工具是 <Link to="/zh-CN/docs/getting-started/using-extensions">扩展</Link> 内部的具体能力，它们共同赋予 goose 执行任务的能力。你可以学习如何控制这些工具，并按自己的方式进行配置。
</p>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 文档与指南</h2>
  <div className={styles.cardGrid}>
    <Card
      title="Tool Permissions"
      description="配置细粒度权限，控制 goose 在什么情况下可以使用哪些工具，从而让自动化过程更安全、可控。"
      link="/zh-CN/docs/guides/managing-tools/tool-permissions"
    />
    <Card
      title="Adjust Tool Output"
      description="自定义工具交互的展示方式，从详细的 verbose 输出到简洁的摘要都可以调整。"
      link="/zh-CN/docs/guides/managing-tools/adjust-tool-output"
    />
    <Card
      title="Code Mode"
      description="一种按需发现并调用 MCP 工具的编程式工作方式。"
      link="/zh-CN/docs/guides/managing-tools/code-mode"
    />
    <Card
      title="Ollama Tool Shim"
      description="通过实验性的本地解释器模型方案，为原生不支持工具调用的模型启用 tool calling。"
      link="/zh-CN/docs/experimental/ollama"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📝 精选博客文章</h2>
  <div className={styles.cardGrid}>
    <Card
      title="Agentic AI and the MCP Ecosystem"
      description="面向 AI agent、tool calling，以及工具如何与 LLM 协作实现强大自动化的入门介绍。"
      link="/blog/2025/02/17/agentic-ai-mcp"
    />
    <Card
      title="A Visual Guide To MCP Ecosystem"
      description="用图示和类比方式拆解 MCP：AI agent、工具和模型是如何协同工作的。"
      link="/blog/2025/04/10/visual-guide-mcp"
    />
    <Card
      title="Finetuning Toolshim Models for Tool Calling"
      description="深入了解开源模型进行 tool calling 时面临的挑战，以及 toolshim 方案背后的研究。"
      link="/blog/2025/04/11/finetuning-toolshim"
    />
  </div>
</div>
