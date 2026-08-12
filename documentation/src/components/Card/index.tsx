import React from 'react';
import Link from '@docusaurus/Link';
import styles from './styles.module.css';

interface CardProps {
  title: string;
  description: string;
  link: string;
  icon?: string;
}

export default function Card({ title, description, link, icon }: CardProps): JSX.Element {
  const isInternalLink = link.startsWith('/');
  const CardWrapper = isInternalLink ? Link : 'a';
  const wrapperProps = isInternalLink ? { to: link } : { href: link };

  return (
    <CardWrapper {...wrapperProps} className={styles.card}>
      <div className={styles.cardContent}>
        {icon && (
          <div className={styles.iconWrapper}>
            <img src={icon} alt="" className={styles.icon} />
          </div>
        )}
        <h3 className={styles.cardTitle}>{title}</h3>
        <p className={styles.cardDescription}>{description}</p>
      </div>
    </CardWrapper>
  );
}
