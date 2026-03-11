import React from "react";
import { motion } from "framer-motion";
import ScrollingMarquee from "@site/src/components/ScrollingMarquee/ScrollingMarquee";
import styles from "./styles.module.css";

const cardVariants = {
  hidden: { opacity: 0, y: 24 },
  visible: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { duration: 0.5, ease: "easeOut", delay: i * 0.1 },
  }),
};

const props = [
  {
    icon: "🌐",
    title: "Open Source",
    description:
      "Started at Block, built by the open source community for everyone. Fully transparent, community-driven development. Permissively licensed (ASLv2).",
  },
  {
    icon: "🔀",
    title: "Multi-Model",
    description:
      "Works with various LLMs. Choose the model that fits your needs. Or different models per sub-agent.",
  },
  {
    icon: "⚡",
    title: "Agentic AI",
    description: "Foundation for autonomous task execution.",
    ticker: false,
  },
];

export default function ValueProps(): React.JSX.Element {
  return (
    <section className={styles.section}>
      <div className={styles.grid}>
        {props.map((prop, i) => (
          <motion.div
            key={prop.title}
            className={styles.card}
            variants={cardVariants}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, amount: 0.2 }}
            custom={i}
          >
            <div className={styles.icon}>{prop.icon}</div>
            <h3 className={styles.cardTitle}>{prop.title}</h3>
            <p className={styles.cardDescription}>{prop.description}</p>
            {prop.ticker && (
              <div className={styles.ticker}>
                <ScrollingMarquee speed={15}>
                  DeepSeek &bull; GPT-4 &bull; Claude &bull; Llama &bull;
                  Gemini &bull; Mistral &bull;&nbsp;
                </ScrollingMarquee>
              </div>
            )}
          </motion.div>
        ))}
      </div>
    </section>
  );
}
