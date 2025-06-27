---
title: Recipes
description: Common recipes and solutions for using Goose
sidebar_position: 1
hide_title: true
---

import Card from '@site/src/components/Card';
import styles from '@site/src/components/Card/styles.module.css';

<h1 className={styles.pageTitle}>Recipes</h1>
<p className={styles.pageDescription}>
  Recipes are agentic reusable workflows that package up specific Goose configurations, making it easy to share and launch complex automation tasks. Explore everything you need to create, customize, and use recipes to supercharge your development workflow.
</p>

<div className="video-container margin-bottom--lg">
  <iframe 
    width="100%"
    height="400"
    src="https://www.youtube.com/embed/D-DpDunrbpo"
    title="Vibe coding with Goose"
    frameBorder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
    allowFullScreen
  ></iframe>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📚 Documentation & Guides</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="Session Recipes"
      description="Share a Goose session setup (including tools, goals, and instructions) as a reusable recipe that others can launch with a single click."
      link="/docs/guides/session-recipes"
    />
    <Card 
      title="Recipe Reference Guide"
      description="Complete technical reference for creating and customizing recipes in Goose via the CLI."
      link="/docs/guides/recipe-reference"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>🛠️ Tools & Generators</h2>
  <div className={styles.cardGrid}>
    <Card 
      title="Recipe Generator"
      description="Interactive tool that creates a shareable Goose recipe URL that others can use to launch a session with your predefined settings."
      link="/goose/recipe-generator"
    />
    <Card 
      title="Recipe Cookbook"
      description="Browse our collection of ready-to-use recipes. Find and adapt recipes for common development scenarios."
      link="/goose/recipes"
    />
  </div>
</div>

<div className={styles.categorySection}>
  <h2 className={styles.categoryTitle}>📝 Featured Blog Posts</h2>
  <div className={styles.cardGrid}>
    <Card
      title="Championship Driven Development"
      description="Recipes to accelerate your developer team's workflow."
      link="/blog/2025/05/09/developers-ai-playbook-for-team-efficiency"
    />
    <Card
      title="A Recipe for Success"
      description="The value of scaling agentic workflows with recipes."
      link="/blog/2025/05/06/recipe-for-success"
    />
  </div>
</div>
