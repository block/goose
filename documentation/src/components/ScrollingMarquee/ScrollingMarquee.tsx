import React from "react";
import styles from "./styles.module.css";

interface ScrollingMarqueeProps {
  children: React.ReactNode;
  speed?: number;
  direction?: "left" | "right";
  className?: string;
  pauseOnHover?: boolean;
}

export default function ScrollingMarquee({
  children,
  speed = 20,
  direction = "left",
  className = "",
  pauseOnHover = false,
}: ScrollingMarqueeProps) {
  const animationName =
    direction === "left" ? styles.scrollLeft : styles.scrollRight;

  const containerClasses = [
    styles.marqueeContainer,
    pauseOnHover ? styles.pauseOnHover : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  const trackStyle: React.CSSProperties = {
    animation: `${animationName} ${speed}s linear infinite`,
  };

  return (
    <div className={containerClasses}>
      <div className={styles.marqueeTrack} style={trackStyle}>
        <div>{children}</div>
        <div>{children}</div>
      </div>
    </div>
  );
}
