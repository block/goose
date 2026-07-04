---
title: Managing Tools
hide_title: true
description: Control and configure the tools and extensions that power your gosling workflows
---

import Card from '@site/src/components/Card';
import styles from '@site/src/components/Card/styles.module.css';

<h1 className={styles.pageTitle}>Managing Tools</h1>
<p className={styles.pageDescription}>
  Tools are specific functions within <a href="/docs/getting-started/using-extensions">extensions</a> that give gosling its capabilities. Learn to control and customize how these tools work for you.
</p>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 Documentation & Guides</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="gosling Permissions"
      description="Choose how much autonomy gosling has when using tools, editing files, and taking action in a session."
      link="/docs/guides/managing-tools/gosling-permissions"
    />
    <Card 
      title="Tool Permissions"
      description="Configure fine-grained permissions to control which tools gosling can use and when, ensuring secure and controlled automation."
      link="/docs/guides/managing-tools/tool-permissions"
    />
    <Card 
      title="Adjust Tool Output"
      description="Customize how tool interactions are displayed, from detailed verbose output to clean concise summaries."
      link="/docs/guides/managing-tools/adjust-tool-output"
    />
    <Card 
      title="Code Mode"
      description="Programmatic approach that discovers and calls MCP tools on demand."
      link="/docs/guides/managing-tools/code-mode"
    />
    <Card 
      title="Ollama Tool Shim"
      description="Enable tool calling for models that don't natively support it using an experimental local interpreter model setup."
      link="/docs/experimental/ollama"
    />
  </div>
</div>
