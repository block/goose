import React from "react";
import Link from "@docusaurus/Link";
import { motion, AnimatePresence } from "framer-motion";
import styles from "./styles.module.css";

interface FullScreenMenuProps {
  isOpen: boolean;
  onClose: () => void;
}

const navItems = [
  { label: "features", to: "/#features", external: false },
  { label: "extensions", to: "/extensions", external: false },
  { label: "docs", to: "/docs/category/getting-started", external: false },
  { label: "blog", to: "/blog", external: false },
  {
    label: "github",
    to: "https://github.com/block/goose",
    external: true,
  },
];

const socialItems = [
  { label: "Twitter", href: "https://x.com/goose_oss" },
  { label: "GitHub", href: "https://github.com/block/goose" },
  { label: "Discord", href: "https://discord.gg/goose-oss" },
];

const overlayVariants = {
  hidden: { opacity: 0 },
  visible: { opacity: 1, transition: { duration: 0.3, ease: "easeOut" } },
  exit: { opacity: 0, transition: { duration: 0.2, ease: "easeIn" } },
};

const linkVariants = {
  hidden: { opacity: 0, y: 20 },
  visible: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { delay: 0.1 + i * 0.05, duration: 0.3, ease: "easeOut" },
  }),
  exit: { opacity: 0, y: -10, transition: { duration: 0.15 } },
};

export default function FullScreenMenu({
  isOpen,
  onClose,
}: FullScreenMenuProps) {
  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          className={styles.overlay}
          variants={overlayVariants}
          initial="hidden"
          animate="visible"
          exit="exit"
        >
          <button
            className={styles.closeButton}
            onClick={onClose}
            aria-label="Close menu"
          >
            Close &times;
          </button>

          <nav className={styles.navLinks}>
            {navItems.map((item, i) =>
              item.external ? (
                <motion.a
                  key={item.label}
                  href={item.to}
                  target="_blank"
                  rel="noopener noreferrer"
                  className={styles.navLink}
                  variants={linkVariants}
                  custom={i}
                  initial="hidden"
                  animate="visible"
                  exit="exit"
                  onClick={onClose}
                >
                  {item.label}
                </motion.a>
              ) : (
                <motion.div
                  key={item.label}
                  variants={linkVariants}
                  custom={i}
                  initial="hidden"
                  animate="visible"
                  exit="exit"
                >
                  <Link
                    to={item.to}
                    className={styles.navLink}
                    onClick={onClose}
                  >
                    {item.label}
                  </Link>
                </motion.div>
              )
            )}
          </nav>

          <div className={styles.socialLinks}>
            {socialItems.map((item) => (
              <a
                key={item.label}
                href={item.href}
                target="_blank"
                rel="noopener noreferrer"
                className={styles.socialLink}
                onClick={onClose}
              >
                {item.label}
              </a>
            ))}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
