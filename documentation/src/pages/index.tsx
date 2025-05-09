import type { ReactNode } from "react";
import Link from "@docusaurus/Link";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import Layout from "@theme/Layout";
import HomepageFeatures from "@site/src/components/HomepageFeatures";

import styles from "./index.module.css";
import { GooseWordmark } from "../components/gooseWordmark";

function HomepageHeader() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={styles.header}>
      <div className={styles.wrapper}>
        <div className={styles.textColumn}>
          <div className="hero--logo">
            <GooseWordmark />
          </div>
          <p className={styles.subtitle}>{siteConfig.tagline}</p>
          <Link className="button button--primary button--lg" to="docs/getting-started/installation">
            install goose
          </Link>
        </div>

        <div className={styles.videoColumn}>
          <iframe
            src="https://www.youtube.com/embed/D-DpDunrbpo"
            className="aspect-ratio"
            title="vibe coding with goose"
            allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
            allowFullScreen
          ></iframe>
        </div>
      </div>
    </header>
  );
}


export default function Home(): ReactNode {
  const { siteConfig } = useDocusaurusContext();
  return (
    <Layout title={`${siteConfig.title}`} description="Your open source AI agent, automating engineering tasks seamlessly.">
      <HomepageHeader />
      <main>
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
