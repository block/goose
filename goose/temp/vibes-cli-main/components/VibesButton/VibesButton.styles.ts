import type React from "react";

const variantColors: Record<string, string> = {
  blue: "var(--vibes-variant-blue)",
  red: "var(--vibes-variant-red)",
  yellow: "var(--vibes-variant-yellow)",
  gray: "var(--vibes-variant-gray)",
};

function getVariantColor(variant: string): string {
  return variantColors[variant] || variant;
}

export const bounceKeyframes = `
  @keyframes vibes-button-bounce {
    0%, 100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-8px);
    }
  }
`;

export function getFormButtonStyle(variant: string, formColor?: string): React.CSSProperties {
  const cssColor = formColor || getVariantColor(variant);

  return {
    width: "100%",
    padding: "3px",
    backgroundColor: cssColor,
    border: "1px solid var(--vibes-button-border)",
    color: "var(--vibes-button-text)",
    fontSize: "24px",
    fontWeight: "bold",
    letterSpacing: "2px",
    cursor: "pointer",
    transition: "0.2s",
    borderRadius: "20px",
    textTransform: "none" as const,
  };
}

export function getButtonStyle(
  variant: string,
  isHovered: boolean,
  isActive: boolean,
  isMobile = false,
  hasIcon: boolean,
  buttonType: string,
  formColor?: string,
): React.CSSProperties {
  if (buttonType === "form") {
    return getFormButtonStyle(variant, formColor);
  }
  const cssColor = getVariantColor(variant);
  let transform = "translate(0px, 0px)";
  let boxShadow = buttonType
    ? `7px 8px 0px 0px ${cssColor}, 7px 8px 0px 2px var(--vibes-button-border)`
    : `8px 10px 0px 0px ${cssColor}, 8px 10px 0px 2px var(--vibes-button-border)`;

  if (isHovered && !isActive) {
    transform = "translate(2px, 2px)";
    boxShadow = `2px 3px 0px 0px ${cssColor}, 2px 3px 0px 2px var(--vibes-button-border)`;
  }

  if (isActive) {
    transform = "translate(4px, 5px)";
    boxShadow = "none";
  }

  return {
    width:
      buttonType === "flat-rounded"
        ? "100%"
        : !hasIcon
          ? "auto"
          : isMobile
            ? "100%"
            : "130px",
    height:
      buttonType === "flat-rounded"
        ? "auto"
        : !hasIcon
          ? "auto"
          : isMobile
            ? "auto"
            : "135px",
    minHeight: isMobile ? "60px" : undefined,
    padding:
      buttonType === "flat-rounded"
        ? "0.5rem 0.75rem"
        : isMobile
          ? buttonType
            ? "none"
            : "0.75rem 1.5rem"
          : "1rem 2rem",
    borderRadius: "12px",
    fontSize: "1rem",
    fontWeight: 700,
    letterSpacing: "0.05em",
    cursor: "pointer",
    transition: "all 0.15s ease",
    position: "relative" as const,
    transform,
    boxShadow,
  };
}

export function getMergedButtonStyle(
  baseStyle: React.CSSProperties,
  ignoreDarkMode: boolean,
  customStyle?: React.CSSProperties,
  buttonType?: "square" | "flat" | "flat-rounded" | "form",
): React.CSSProperties {
  if (buttonType === "form") {
    return {
      ...baseStyle,
      ...customStyle,
    };
  }

  const style: React.CSSProperties = {
    ...baseStyle,
    background: ignoreDarkMode
      ? "var(--vibes-button-bg)"
      : "var(--vibes-button-bg-dark-aware)",
    color: ignoreDarkMode
      ? "var(--vibes-button-text)"
      : "var(--vibes-button-text-dark-aware)",
    border: ignoreDarkMode
      ? "2px solid var(--vibes-button-border)"
      : "2px solid var(--vibes-button-border-dark-aware)",
  };

  if (buttonType === "flat-rounded") {
    style.borderRadius = "50px";
  }

  return {
    ...style,
    ...customStyle,
  };
}

export function getIconContainerStyle(
  variant: string,
  isMobile: boolean,
  hasIcon: boolean,
  buttonType: string,
): React.CSSProperties {
  if (!hasIcon) return {};

  const cssColor = getVariantColor(variant);

  return {
    width: buttonType === "flat-rounded" ? "28px" : isMobile ? "48px" : "80px",
    height: buttonType === "flat-rounded" ? "28px" : isMobile ? "48px" : "80px",
    backgroundColor: buttonType === "flat-rounded" ? "transparent" : cssColor,
    borderRadius: buttonType === "flat-rounded" ? "0" : "8px",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    flexShrink: 0,
    border:
      buttonType === "flat-rounded" ? "none" : "2px solid var(--vibes-black)",
  };
}

export function getIconStyle(
  isMobile: boolean,
  isHovered: boolean,
  isActive: boolean,
): React.CSSProperties {
  return {
    width: isMobile ? "28px" : "50px",
    height: isMobile ? "28px" : "50px",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    animation:
      isHovered && !isActive
        ? "vibes-button-bounce 0.8s ease-in-out infinite"
        : "none",
  };
}

export function getContentWrapperStyle(
  isMobile: boolean,
  hasIcon: boolean,
  buttonType: string,
): React.CSSProperties {
  if (!hasIcon) return {};

  return {
    display: "flex",
    alignItems: "center",
    gap: buttonType === "flat-rounded" ? "0.5rem" : isMobile ? "16px" : "6px",
    flexDirection:
      buttonType === "flat-rounded"
        ? ("row" as const)
        : isMobile
          ? ("row" as const)
          : ("column" as const),
    justifyContent:
      buttonType === "flat-rounded"
        ? ("flex-start" as const)
        : isMobile
          ? ("flex-start" as const)
          : ("center" as const),
    width: "100%",
  };
}
