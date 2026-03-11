import React from "react";
import Link from "@docusaurus/Link";
import { motion } from "framer-motion";
import { GooseLogo } from "@site/src/components/GooseLogo";
import styles from "./styles.module.css";

export default function HeroSection(): React.JSX.Element {
  return (
    <motion.section
      className={styles.hero}
      initial={{ opacity: 0, y: 24 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.6, ease: "easeOut" }}
    >
      <GooseLogo className={styles.logo} />

      <p className={styles.tagline}>
        <b>goose</b> is a general-purpose AI agent created by <a href="https://opensource.block.xyz">Block</a>, and now a part of <a href="https://aaif.io">the Agentic AI Foundation (AAIF)</a>. It helps you code,
        automate tasks, and solve problems with powerful extensions.
      </p>

      <div className={styles.buttons}>
        <Link
          className={styles.primaryButton}
          to="/docs/getting-started/installation"
        >
          Get Started
        </Link>
        <a
          className={styles.secondaryButton}
          href="https://github.com/block/goose"
          target="_blank"
          rel="noopener noreferrer"
        >
          View on GitHub
        </a>
      </div>
    </motion.section>
  );
}
