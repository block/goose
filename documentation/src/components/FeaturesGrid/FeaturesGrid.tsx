import React from "react";
import { motion } from "framer-motion";
import styles from "./styles.module.css";

const cardVariants = {
  hidden: { opacity: 0, y: 24 },
  visible: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { duration: 0.5, ease: "easeOut", delay: i * 0.1 },
  }),
};

function TerminalVisual() {
  return (
    <div className={styles.terminal}>
      <div className={styles.terminalLine}>
        <span className="prompt">$</span> kubectl apply -f deploy.yaml
      </div>
      <div className={styles.terminalLine}>
        <span className="prompt">$</span> npm install goose
      </div>
    </div>
  );
}

const extensions = [
  { icon: "🛠", label: "Developer" },
  { icon: "📊", label: "Analytics" },
  { icon: "🌐", label: "Browser" },
  { icon: "🧠", label: "Memory" },
];

function ExtensionsVisual() {
  return (
    <div className={styles.chips}>
      {extensions.map((ext) => (
        <span key={ext.label} className={styles.chip}>
          <span className={styles.chipIcon}>{ext.icon}</span>
          {ext.label}
        </span>
      ))}
    </div>
  );
}

const tasks = [
  { label: "Set up CI/CD pipeline", done: true },
  { label: "Write integration tests", done: true },
  { label: "Deploy to staging", done: false },
];

function ChecklistVisual() {
  return (
    <div className={styles.checklist}>
      {tasks.map((task) => (
        <div key={task.label} className={styles.checkItem}>
          <span
            className={`${styles.checkbox} ${task.done ? styles.checked : ""}`}
          >
            {task.done ? "✓" : ""}
          </span>
          {task.label}
        </div>
      ))}
    </div>
  );
}

function ChatVisual() {
  return (
    <div className={styles.chatBubbles}>
      <div className={`${styles.bubble} ${styles.bubbleLeft}`}>
        How does the auth flow work?
      </div>
      <div className={`${styles.bubble} ${styles.bubbleRight}`}>
        Based on our last session, you're using OAuth 2.0 with PKCE…
      </div>
      <div className={`${styles.bubble} ${styles.bubbleLeft}`}>
        Right — can you update the refresh logic?
      </div>
    </div>
  );
}

const cards = [
  {
    title: "Developer Tools",
    subtitle: "Code editing and shell commands",
    Visual: TerminalVisual,
  },
  {
    title: "Extensions",
    subtitle: "Dynamic plugin system",
    Visual: ExtensionsVisual,
  },
  {
    title: "Task Management",
    subtitle: "Break down complex problems into manageable steps",
    Visual: ChecklistVisual,
  },
  {
    title: "Smart Memory",
    subtitle: "Context-aware conversations that remember what matters",
    Visual: ChatVisual,
  },
];

export default function FeaturesGrid(): React.JSX.Element {
  return (
    <section className={styles.section}>
      <div className={styles.grid}>
        {cards.map((card, i) => (
          <motion.div
            key={card.title}
            className={styles.card}
            variants={cardVariants}
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, amount: 0.2 }}
            custom={i}
          >
            <h3 className={styles.cardTitle}>{card.title}</h3>
            <p className={styles.cardSubtitle}>{card.subtitle}</p>
            <card.Visual />
          </motion.div>
        ))}
      </div>
    </section>
  );
}
