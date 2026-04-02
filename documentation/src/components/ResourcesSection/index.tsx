import React from 'react';
import Link from '@docusaurus/Link';
import styles from './styles.module.css';

const resources = [
  {
    title: 'Extensions',
    description: 'Browse and discover MCP extensions to enhance goose',
    icon: '🔌',
    link: '/extensions',
  },
  {
    title: 'Skills Marketplace',
    description: 'Find pre-built skills for common workflows',
    icon: '🎯',
    link: '/skills',
  },
  {
    title: 'Recipe Generator',
    description: 'Create custom automation recipes',
    icon: '🧪',
    link: '/recipe-generator',
  },
  {
    title: 'Prompt Library',
    description: 'Collection of effective prompts for goose',
    icon: '💬',
    link: '/prompt-library',
  },
  {
    title: 'Recipe Cookbook',
    description: 'Browse community-created recipes',
    icon: '📚',
    link: '/recipes',
  },
  {
    title: 'Deeplink Generator',
    description: 'Generate direct links to goose features',
    icon: '🔗',
    link: '/deeplink-generator',
  },
];

export default function ResourcesSection() {
  return (
    <section className={styles.resourcesSection}>
      <div className="container">
        <div className={styles.resourcesWrapper}>
          <div className={styles.resourcesGrid}>
            {resources.map((resource, idx) => (
              <Link key={idx} to={resource.link} className={styles.resourceCard}>
                <div className={styles.resourceIcon}>{resource.icon}</div>
                <div className={styles.resourceContent}>
                  <h3 className={styles.resourceTitle}>{resource.title}</h3>
                  <p className={styles.resourceDesc}>{resource.description}</p>
                </div>
              </Link>
            ))}
          </div>
          <div className={styles.videoWrapper}>
            <iframe
              src="https://www.youtube.com/embed/D-DpDunrbpo"
              className={styles.video}
              title="vibe coding with goose"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
              allowFullScreen
            ></iframe>
          </div>
        </div>
      </div>
    </section>
  );
}
