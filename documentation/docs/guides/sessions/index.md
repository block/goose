---
title: Managing Sessions
hide_title: true
description: Manage your session lifecycle and ongoing interactions with gosling
---

import Card from '@site/src/components/Card';
import styles from '@site/src/components/Card/styles.module.css';
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

<h1 className={styles.pageTitle}>Managing Sessions</h1>
<p className={styles.pageDescription}>
  Sessions are your continuous interactions with gosling. Each session maintains context and conversation history, enabling gosling to understand your ongoing work and provide relevant assistance.
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
      description="Discover features you can use to share information and communicate with gosling during sessions."
      link="/docs/guides/sessions/in-session-actions"
    />
    <Card 
      title="Smart Context Management"
      description="Maintain productive sessions using features that help manage context and conversation limits."
      link="/docs/guides/sessions/smart-context-management"
    />
  </div>
</div>
