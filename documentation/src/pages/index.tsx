import type { ReactNode } from "react";
import Link from "@docusaurus/Link";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import Layout from "@theme/Layout";
import HomepageFeatures from "@site/src/components/HomepageFeatures";

import styles from "./index.module.css";
import { GooseLogo } from "../components/GooseLogo";

function HomepageHeader() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={styles.header}>
      <div className={styles.wrapper}>
        <div className={styles.textColumn}>
          <div className="hero--logo">
            <GooseLogo />
          </div>
          <p className={styles.subtitle}>{siteConfig.tagline}</p>
          <div className={styles.installBlock}>
            <div className={styles.installTabs}>
              <code className={styles.installCommand}>
                brew install block/tap/goose
              </code>
            </div>
            <p className={styles.installNote}>
              Free and open source. Works with ChatGPT, Claude, Gemini, and local models.
            </p>
          </div>
          <div className={styles.heroButtons}>
            <Link className="button button--primary button--lg" to="docs/getting-started/installation">
              Get Started
            </Link>
            <Link className="button button--outline button--lg" to="docs/quickstart">
              Quickstart Guide
            </Link>
          </div>
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
  return (
    <Layout description="Goose is an open-source AI agent by Block. Connect any LLM to your tools via MCP. Desktop app, CLI, scheduled agents, and 100+ extensions. Free to start.">
      <HomepageHeader />
      <main>
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
