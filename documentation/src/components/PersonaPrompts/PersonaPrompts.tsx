import React from "react";
import { motion } from "framer-motion";
import ScrollingMarquee from "@site/src/components/ScrollingMarquee/ScrollingMarquee";
import styles from "./styles.module.css";

const SEPARATOR = " \u2022 ";

interface Persona {
  label: string;
  prompts: string[];
  speed: number;
  direction: "left" | "right";
}

const personas: Persona[] = [
  {
    label: "Everyday",
    prompts: [
      "START RESEARCH ON CAKE RECIPES",
      "RESPOND TO CUSTOMER EMAILS",
      "SUMMARIZE MY MEETING NOTES",
      "ORGANIZE MY PHOTOS BY DATE",
    ],
    speed: 18,
    direction: "left",
  },
  {
    label: "Developers",
    prompts: [
      "REFACTOR MY REPO TO USE REACT",
      "DEBUG MY CI PIPELINE",
      "WRITE INTEGRATION TESTS",
      "SET UP A DOCKER ENVIRONMENT",
    ],
    speed: 22,
    direction: "right",
  },
  {
    label: "Designers",
    prompts: [
      "CREATE A TOKEN LIBRARY FOR DARK MODE",
      "UNIFY MY LIBRARY STYLING USING TAILWIND",
      "GENERATE A COLOR PALETTE",
      "PROTOTYPE A LANDING PAGE",
    ],
    speed: 20,
    direction: "left",
  },
  {
    label: "Builders",
    prompts: [
      "BUILD A NEW GARDENING APP",
      "SCAFFOLD A REST API",
      "CREATE A CHROME EXTENSION",
      "DEPLOY TO PRODUCTION",
    ],
    speed: 24,
    direction: "right",
  },
];

export default function PersonaPrompts() {
  return (
    <motion.section
      className={styles.section}
      initial={{ opacity: 0, y: 24 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, amount: 0.2 }}
      transition={{ duration: 0.6, ease: "easeOut" }}
    >
      {personas.map((persona) => (
        <div key={persona.label} className={styles.row}>
          <span className={styles.label}>{persona.label}</span>
          <div className={styles.marqueeWrapper}>
            <ScrollingMarquee
              speed={persona.speed}
              direction={persona.direction}
            >
              <span className={styles.promptText}>
                {persona.prompts.join(SEPARATOR)}
                {SEPARATOR}
              </span>
            </ScrollingMarquee>
          </div>
        </div>
      ))}
    </motion.section>
  );
}
