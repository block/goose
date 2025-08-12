---
title: Rich Interactive Chat
hide_title: true
description: Transform text-based responses into graphical components and interactive elements
---

import Card from '@site/src/components/Card';
import styles from '@site/src/components/Card/styles.module.css';

<h1 className={styles.pageTitle}>Rich Interactive Chat</h1>
<p className={styles.pageDescription}>
  Goose Desktop supports extensions that transform text-only responses into graphical, interactive experiences. Instead of reading through lists and descriptions, you can click, explore, and interact with UI components directly in your conversations.
</p>

 <div className="video-container margin-bottom--lg">
  <iframe 
    class="aspect-ratio"
    src="https://www.youtube.com/embed/QJHGvsVXhjw"
    title="Turn Any AI Chat Into an Interactive Experience | MCP-UI"
    frameBorder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
    allowFullScreen
  ></iframe>
</div> 

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 Documentation & Guides</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="MCP-UI Extensions"
      description="Goose transforms text-based responses into engaging graphical and interactive user experiences."
      link="/docs/guides/interactive-chat/mcp-ui"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📝 Featured Blog Posts</h2>
  <div className={styles.cardGrid}>
    <Card      
      title="MCP UI: Bringing the Browser into the Agent"
      description="MCP-UI servers return content that Goose Desktop renders as rich, embeddable UI."
      link="/blog/2025/08/11/mcp-ui-post-browser-world"
    />
  </div>
</div>