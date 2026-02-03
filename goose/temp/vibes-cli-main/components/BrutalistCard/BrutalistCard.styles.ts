import { CSSProperties } from "react";

export type BrutalistCardVariant = "default" | "success" | "error" | "warning";
export type BrutalistCardSize = "sm" | "md" | "lg";

function getShadowColor(variant: BrutalistCardVariant): string {
  switch (variant) {
    case "success":
      return "var(--vibes-green)";
    case "error":
      return "var(--vibes-red-accent)";
    case "warning":
      return "var(--vibes-yellow-accent)";
    case "default":
    default:
      return "var(--vibes-shadow-color)";
  }
}

function getPadding(size: BrutalistCardSize): string {
  switch (size) {
    case "sm":
      return "0.75rem 1rem";
    case "md":
      return "1rem";
    case "lg":
      return "2rem 3rem";
    default:
      return "1rem";
  }
}

function getFontSize(size: BrutalistCardSize): string {
  switch (size) {
    case "sm":
      return "0.875rem";
    case "md":
      return "1rem";
    case "lg":
      return "1rem";
    default:
      return "1rem";
  }
}

function getBoxShadow(
  size: BrutalistCardSize,
  variant: BrutalistCardVariant,
): string {
  const color = getShadowColor(variant);

  switch (size) {
    case "sm":
      return `2px 3px 0px 0px ${color}`;
    case "md":
      return `4px 5px 0px 0px ${color}`;
    case "lg":
      return `6px 6px 0px 0px ${color}`;
    default:
      return `4px 5px 0px 0px ${color}`;
  }
}

function getBorderRadius(messageType?: "user" | "ai"): string {
  switch (messageType) {
    case "user":
      return "12px 12px 0 12px";
    case "ai":
      return "12px 12px 12px 0";
    default:
      return "12px";
  }
}

export function getBrutalistCardStyle(
  variant: BrutalistCardVariant = "default",
  size: BrutalistCardSize = "md",
  messageType?: "user" | "ai",
): CSSProperties {
  return {
    borderRadius: getBorderRadius(messageType),
    padding: getPadding(size),
    fontSize: getFontSize(size),
    fontWeight: 500,
    letterSpacing: "0.02em",
    boxShadow: getBoxShadow(size, variant),
    transition: "box-shadow 0.15s ease, transform 0.15s ease",
    boxSizing: "border-box" as const,
  };
}
