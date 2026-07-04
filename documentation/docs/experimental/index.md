---
title: Experimental
hide_title: true
description: Experimental and radically unstable, but lot's of fun.
---

import Card from '@site/src/components/Card';
import styles from '@site/src/components/Card/styles.module.css';

<h1 className={styles.pageTitle}>Experimental</h1>
<p className={styles.pageDescription}>
  gosling is an open source project that is constantly being improved and expanded upon. These experimental features and projects are still in development and may not be fully stable or ready for production use, but they showcase exciting possibilities for the future of AI automation.
</p>

:::note
The list of experimental features may change as gosling development progresses. Some features may be promoted to stable features, while others might be modified or removed. This section will be updated with specific experimental features as they become available.
:::

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>🧪 Experimental Features</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="Ollama Tool Shim"
      description="Enable tool calling capabilities for language models that don't natively support tool calling (like DeepSeek) using an experimental local interpreter model setup."
      link="/docs/experimental/ollama"
    />
    <Card 
      title="gosling for VS Code Extension"
      description="Interact with gosling directly from VS Code via ACP."
      link="/docs/experimental/vs-code-extension"
    />
    <Card 
      title="Using gosling in ACP Clients"
      description="Interact with gosling natively in ACP-compatible clients like Zed."
      link="/docs/guides/acp-clients"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>💬 Feedback & Support</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="GitHub Issues"
      description="Report bugs, request features, or contribute to the development of experimental features."
      link="https://github.com/repo-makeover/gosling/issues"
    />
    <Card 
      title="Discord Community"
      description="Join our community to discuss experimental features, share feedback, and connect with other users."
      link="https://discord.gg/gosling-oss"
    />
  </div>
</div>
