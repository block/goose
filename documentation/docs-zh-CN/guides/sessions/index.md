---
title: 会话管理
hide_title: true
description: 管理 goose 的会话生命周期和持续中的交互
---

import Card from "@site/src/components/Card";
import styles from "@site/src/components/Card/styles.module.css";

<h1 className={styles.pageTitle}>会话管理</h1>
<p className={styles.pageDescription}>
  会话是你和 goose 的连续交互。每个会话都会保留上下文和对话历史，让 goose 能理解你正在推进的工作，并提供更相关的帮助。
</p>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 文档与指南</h2>
  <div className={styles.cardGrid}>
    <Card
      title="Session Management"
      description="了解如何开始、恢复或搜索会话，以及执行其它会话管理操作。"
      link="/zh-CN/docs/guides/sessions/session-management"
    />
    <Card
      title="In-Session Actions"
      description="查看你可以在会话中用来共享信息、与 goose 协作的功能。"
      link="/zh-CN/docs/guides/sessions/in-session-actions"
    />
    <Card
      title="Smart Context Management"
      description="了解帮助你管理上下文和对话限制的能力，从而维持高效率会话。"
      link="/zh-CN/docs/guides/sessions/smart-context-management"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📝 精选博客文章</h2>
  <div className={styles.cardGrid}>
    <Card
      title="6 Essential Tips for Working with goose"
      description="了解聚焦式会话、分步骤引导以及持续优化提示词，如何帮助你获得更高效的会话体验。"
      link="/blog/2025/03/06/goose-tips"
    />
    <Card
      title="AI Prompting 101: How to Get the Best Responses from Your AI Agent"
      description="通过给提示词增加结构，让你的会话更有效率。"
      link="/blog/2025/03/19/better-ai-prompting"
    />
    <Card
      title="The AI Skeptic’s Guide to Context Windows"
      description="了解 context window、token，以及 goose 如何帮助你管理记忆和长对话。"
      link="/blog/2025/08/18/understanding-context-windows"
    />
  </div>
</div>
