---
title: Managing Tools
hide_title: true
description: Control and configure the tools and extensions that power your Goose workflows
---

import Card from '@site/src/components/Card';
import styles from '@site/src/components/Card/styles.module.css';

<h1 className={styles.pageTitle}>Managing Tools</h1>
<p className={styles.pageDescription}>
  Tools are the building blocks that give Goose its capabilities. Learn how to control permissions, configure behavior, and optimize tool performance for secure and efficient workflows.
</p>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 Documentation & Guides</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="Tool Permissions"
      description="Configure fine-grained permissions to control which tools Goose can use and when, ensuring secure and controlled automation."
      link="/docs/guides/managing-tools/tool-permissions"
    />
    <Card 
      title="Tool Router"
      description="Optimize tool selection with dynamic routing that loads only the tools you need, reducing context overhead and improving performance."
      link="/docs/guides/managing-tools/tool-router"
    />
    <Card 
      title="Adjust Tool Output"
      description="Customize how tool interactions are displayed, from detailed verbose output to clean concise summaries."
      link="/docs/guides/managing-tools/adjust-tool-output"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📝 Featured Blog Posts</h2>
  <div className={styles.cardGrid}>
    <Card
      title="Agentic AI and the MCP Ecosystem"
      description="A 101 introduction to AI agents, tool calling, and how tools work with LLMs to enable powerful automation."
      link="/blog/2025/02/17/agentic-ai-mcp"
    />
    <Card
      title="A Visual Guide To MCP Ecosystem"
      description="Visual breakdown of MCP: How your AI agent, tools, and models work together, explained with diagrams and analogies."
      link="/blog/2025/04/10/visual-guide-mcp"
    />
  </div>
</div>
