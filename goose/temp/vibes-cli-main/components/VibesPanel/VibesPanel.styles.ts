import type React from "react";

export function getOuterContainerStyle(
  customStyle?: React.CSSProperties,
): React.CSSProperties {
  return {
    padding: "12px",
    display: "flex",
    flexDirection: "column",
    justifyContent: "center",
    alignItems: "center",
    gap: "12px",
    ...customStyle,
  };
}

export function getContainerStyle(): React.CSSProperties {
  return {
    position: "relative",
    display: "inline-flex",
    alignItems: "stretch",
    width: "auto",
    marginBottom: "40px",
  };
}

export function getLabelStyle(): React.CSSProperties {
  return {
    background: "var(--vibes-card-bg)",
    border: "2px solid var(--vibes-card-border)",
    borderLeft: "none",
    borderTopRightRadius: "8px",
    borderBottomRightRadius: "8px",
    padding: "12px 8px",
    fontWeight: 700,
    fontSize: "14px",
    textTransform: "uppercase",
    letterSpacing: "1px",
    whiteSpace: "nowrap",
    color: "var(--vibes-card-text)",
    writingMode: "vertical-rl",
    transform: "rotate(180deg)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    flexShrink: 0,
    margin: "32px 0px",
  };
}

export function getButtonWrapperStyle(): React.CSSProperties {
  return {
    background: "var(--vibes-card-bg)",
    border: "2px solid var(--vibes-card-border)",
    borderRadius: "8px",
    padding: "24px 24px 32px 24px",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    width: "auto",
  };
}

export function getButtonContainerStyle(
  isMobile: boolean,
): React.CSSProperties {
  return {
    display: "flex",
    flexDirection: "row",
    justifyContent: "center",
    alignItems: "center",
    gap: isMobile ? "24px" : "65px",
    flexWrap: "wrap",
    maxWidth: "100%",
    width: isMobile ? "100%" : "auto",
    padding: isMobile ? "0px" : "0px 65px",
  };
}

export function getInviteFormStyle(isMobile: boolean): React.CSSProperties {
  return {
    width: isMobile ? "calc(100% - 40px)" : "300px",
    display: "flex",
    padding: "20px",
    flexDirection: "column",
    gap: "12px",
    borderRadius: "20px",
    background: "var(--vibes-button-bg)",
  };
}

export function getInviteLabelStyle(): React.CSSProperties {
  return {
    alignSelf: "flex-start",
    fontWeight: 600,
    color: "#231F20",
  };
}

export function getInviteInputStyle(): React.CSSProperties {
  return {
    width: "calc(100% - 20px)",
    background: "transparent",
    color: "var(--vibes-text-primary)",
    fontSize: "inherit",
    fontWeight: "inherit",
    letterSpacing: "inherit",
    padding: "8px 10px",
    border: "2px solid var(--vibes-card-border)",
    borderRadius: "20px",
  };
}

export function getInviteStatusStyle(): React.CSSProperties {
  return {
    textAlign: "center",
  };
}

export function getResponsiveLabelStyle(
  isMobile: boolean,
): React.CSSProperties {
  if (isMobile) {
    return {
      display: "none",
    };
  }
  return getLabelStyle();
}

export function getResponsiveButtonWrapperStyle(
  isMobile: boolean,
): React.CSSProperties {
  if (isMobile) {
    return {
      background: "transparent",
      border: "none",
      borderRadius: "0",
      padding: "0",
      paddingBottom: "24px",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      width: "auto",
    };
  }
  return getButtonWrapperStyle();
}

export function getResponsiveContainerStyle(
  isMobile: boolean,
): React.CSSProperties {
  if (isMobile) {
    return {
      ...getContainerStyle(),
      flexDirection: "column",
      width: "100%",
    };
  }
  return getContainerStyle();
}

export function getInviteRowStyle(isMobile: boolean): React.CSSProperties {
  return {
    display: "flex",
    flexDirection: isMobile ? "column" : "row",
    gap: isMobile ? "24px" : "65px",
    width: isMobile ? "100%" : "auto",
  };
}

export function getButtonAnimationStyle(
  isVisible: boolean,
  delay = 0,
): React.CSSProperties {
  return {
    opacity: isVisible ? 1 : 0,
    visibility: isVisible ? "visible" : "hidden",
    transition: `opacity 0.3s ease-in-out ${delay}ms, visibility 0s ${isVisible ? "0ms" : "300ms"}`,
    pointerEvents: isVisible ? "auto" : "none",
  };
}

export function getInviteFormContainerStyle(
  isVisible: boolean,
): React.CSSProperties {
  return {
    display: "flex",
    flexDirection: "row",
    gap: "12px",
    gridColumn: "1 / -1",
    opacity: isVisible ? 1 : 0,
    transform: isVisible ? "translateX(0)" : "translateX(-20px)",
    transition: "opacity 0.4s ease-out 0.2s, transform 0.4s ease-out 0.2s",
    pointerEvents: isVisible ? "auto" : "none",
  };
}

export function getAnimatedButtonContainerStyle(): React.CSSProperties {
  return {
    display: "grid",
    gridTemplateColumns: "repeat(4, 1fr)",
    gap: "24px",
    width: "100%",
    position: "relative",
  };
}
