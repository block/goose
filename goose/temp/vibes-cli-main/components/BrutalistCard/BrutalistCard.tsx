import React from "react";
import { getBrutalistCardStyle } from "./BrutalistCard.styles.js";
import type {
  BrutalistCardVariant,
  BrutalistCardSize,
} from "./BrutalistCard.styles.js";

export interface BrutalistCardProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
  variant?: BrutalistCardVariant;
  size?: BrutalistCardSize;
  messageType?: "user" | "ai";
}

export const BrutalistCard = React.forwardRef<
  HTMLDivElement,
  BrutalistCardProps
>(
  (
    {
      children,
      variant = "default",
      size = "md",
      messageType,
      style,
      className,
      ...divProps
    }: BrutalistCardProps,
    ref,
  ) => {
    const cardStyle = {
      ...getBrutalistCardStyle(variant, size, messageType),
      background: "var(--vibes-card-bg)",
      color: "var(--vibes-card-text)",
      border: "3px solid var(--vibes-card-border)",
      ...style,
    } as React.CSSProperties;

    return (
      <div ref={ref} style={cardStyle} className={className} {...divProps}>
        {children}
      </div>
    );
  },
);

BrutalistCard.displayName = "BrutalistCard";
