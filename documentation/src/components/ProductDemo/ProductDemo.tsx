import React from "react";
import { motion } from "framer-motion";
import useBaseUrl from "@docusaurus/useBaseUrl";
import ScrollingMarquee from "@site/src/components/ScrollingMarquee/ScrollingMarquee";
import styles from "./styles.module.css";

export default function ProductDemo(): React.JSX.Element {
  const demoSrc = useBaseUrl("/img/goose-demo.svg");

  return (
    <motion.section
      className={styles.section}
      initial={{ opacity: 0 }}
      whileInView={{ opacity: 1 }}
      viewport={{ once: true, amount: 0.2 }}
      transition={{ duration: 0.6 }}
    >
      <div className={styles.demoContainer}>
        <img
          src={demoSrc}
          alt="Goose desktop application"
          className={styles.demoImage}
          loading="lazy"
        />
        <div className={styles.marqueeOverlay}>
          <ScrollingMarquee speed={30}>
            <span className={styles.marqueeText}>
              SOFTWARE BUILT BY THE PEOPLE &bull; MAKE YOUR DREAMS COME TRUE
              &bull; ALWAYS CUSTOM &bull; ALWAYS FREE &bull;&nbsp;
            </span>
          </ScrollingMarquee>
        </div>
      </div>
    </motion.section>
  );
}
