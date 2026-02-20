---
title: MongoDB Storage
hide_title: true
description: Use MongoDB as an alternative session storage backend for goose
---

import Card from '@site/src/components/Card';
import styles from '@site/src/components/Card/styles.module.css';

<h1 className={styles.pageTitle}>MongoDB Storage</h1>
<p className={styles.pageDescription}>
  goose stores session data in SQLite by default. The MongoDB storage backend is an alternative that persists sessions and messages to MongoDB, enabling centralized storage for multi-node deployments.
</p>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>Documentation</h2>
  <div className={styles.cardGrid}>
    <Card
      title="Pluggable Session Storage"
      description="How the session storage abstraction works and how to configure goose to use MongoDB."
      link="/docs/mongodb-storage/pluggable-session-storage"
    />
    <Card
      title="Testing"
      description="How to run the MongoDB storage tests against a Docker MongoDB instance."
      link="/docs/mongodb-storage/testing"
    />
  </div>
</div>
