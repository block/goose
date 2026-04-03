import type { ReactNode } from "react";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import Layout from "@theme/Layout";
import HomepageFeatures from "@site/src/components/HomepageFeatures";

import styles from "./index.module.css";
import { GooseLogo } from "../components/GooseLogo";
import HeroInstall from "../components/HeroInstall";
import ResourcesSection from "../components/ResourcesSection";

function HomepageHeader() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={styles.header}>
      <div className={styles.heroContainer}>
        <div className={styles.heroContent}>
          <div className={styles.logoWrapper}>
            <GooseLogo className={styles.logo} />
          </div>
          <h1 className={styles.title}>
            Your open source AI agent, automating engineering tasks seamlessly
          </h1>
          <p className={styles.subtitle}>
            Free to use with any models of your choice
          </p>

          <HeroInstall />
        </div>
      </div>
    </header>
  );
}


export default function Home(): ReactNode {
  return (
    <Layout 
      title="goose - open source AI agent"
      description="your open source AI agent, automating engineering tasks seamlessly">
      
      <HomepageHeader />
      <main>
        <ResourcesSection />
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
