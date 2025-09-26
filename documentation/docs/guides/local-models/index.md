---
title: Local Models
hide_title: true
description: Use local models to enhance your Goose experience with tools like Ollama, llama.cpp, and more
---

import Card from '@site/src/components/Card';
import styles from '@site/src/components/Card/styles.module.css';

<h1 className={styles.pageTitle}>Local Models</h1>
<p className={styles.pageDescription}>
  Local models allow you to run and manage machine learning models directly on your device, providing enhanced privacy.
</p>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 Ollama Guides</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="Recommended Models"
      description="Community-drive list of local models known to work with Goose"
      link="/docs/guides/local-models/recommended-models"
    />
    <Card 
      title="Ollama Setup Notes"
      description="Using Ollama with Goose"
      link="/docs/guides/local-models/ollama-notes"
    />
    <Card 
      title="llama.cpp Setup Notes"
      description="Using llama.cpp with Goose"
      link="/docs/guides/local-models/llama-cpp-notes"
    />
    <Card 
      title="Mac Hardware Notes"
      description="Recommendations for Mac hardware setup for local models with Goose"
      link="/docs/guides/local-models/mac-hardware"
    />
    <Card 
      title="Windows Hardware Notes"
      description="Recommendations for Windows hardware setup for local models with Goose"
      link="/docs/guides/local-models/windows-hardware"
    />
    <Card 
      title="Linux Hardware Notes"
      description="Recommendations for Linux hardware setup for local models with Goose"
      link="/docs/guides/local-models/linux-hardware"
    />
  </div>
</div>
