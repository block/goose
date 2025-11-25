---
title: Managing Sessions
hide_title: true
description: Manage your session lifecycle and ongoing interactions with goose
---

import Card from '@site/src/components/Card';
import styles from '@site/src/components/Card/styles.module.css';
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

<h1 className={styles.pageTitle}>Managing Sessions</h1>
<p className={styles.pageDescription}>
  Sessions are your continuous interactions with goose. Each session maintains context and conversation history, enabling goose to understand your ongoing work and provide relevant assistance.
</p>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 Documentation & Guides</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="Session Management"
      description="Learn how to start, resume, or search sessions, and perform other session management tasks."
      link="/docs/guides/sessions/session-management"
    />
    <Card 
      title="In-Session Actions"
      description="Discover features you can use to share information and communicate with goose during sessions."
      link="/docs/guides/sessions/in-session-actions"
    />
    <Card 
      title="Smart Context Management"
      description="Maintain productive sessions using features that help manage context and conversation limits."
      link="/docs/guides/sessions/smart-context-management"
    />
  </div>
</div>
<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📝 Featured Blog Posts</h2>
  <div className={styles.cardGrid}>
    <Card
      title="6 Essential Tips for Working with goose"
      description="Learn how focused sessions, step-by-step guidance, and refining your prompts can lead to more productive sessions."
      link="/blog/2025/03/06/goose-tips"
    />
    <Card
      title="AI Prompting 101: How to Get the Best Responses from Your AI Agent"
      description="Make your sessions more effective by adding structure to your prompts."
      link="/blog/2025/03/19/better-ai-prompting"
    />
    <Card
      title="The AI Skeptic’s Guide to Context Windows"
      description="Learn how context windows, tokens, and goose help you manage memory and long conversations."
      link="/blog/2025/08/18/understanding-context-windows"
    />
  </div>
</div>
