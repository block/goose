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
  <h2 className={styles.categoryTitle}>📚 Recommended Models</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="Recommended Models"
      description="Community-drive list of local models known to work with Goose"
      link="/docs/guides/local-models/recommended-models"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 Ollama Guides</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="Using Ollama with Goose - Start Here"
      description="Using Ollama with Goose - Start Here"
      link="/docs/guides/local-models/ollama-notes"
    />
    <!-- <Card 
      title="MacOS Notes"
      description="Recommendations for MacOS for local models with Goose"
      link="/docs/guides/local-models/macos"
    /> -->
    <Card 
      title="Ollama on MacOS"
      description="Recommendations for Ollama on MacOS for local models with Goose"
      link="/docs/guides/local-models/macos-ollama"
    />
    <Card 
      title="Ollama on Windows"
      description="Recommendations for Ollama on Windows for local models with Goose"
      link="/docs/guides/local-models/windows-ollama"
    />
    <Card 
      title="Ollama on Linux"
      description="Recommendations for Ollama on Linux for local models with Goose"
      link="/docs/guides/local-models/linux-ollama"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 llama.cpp Guides</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="llama.cpp Setup Notes"
      description="Using llama.cpp with Goose - Start Here"
      link="/docs/guides/local-models/llama-cpp-notes"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 Operating System Specifics</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="MacOS"
      description="Operating System Specifics - MacOS - all Providers"
      link="/docs/guides/local-models/macos"
      slug="MacOS"
    />

  </div>
</div>
