import type React from "react";

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

export function getResponsiveLabelStyle(
  isMobile: boolean,
  disappear = false,
): React.CSSProperties {
  if (isMobile) {
    if (disappear) {
      return {
        display: "none",
      };
    }
    return {
      background: "var(--vibes-card-bg)",
      border: "2px solid var(--vibes-card-border)",
      borderLeft: "2px solid var(--vibes-card-border)",
      borderBottom: "none",
      borderTopLeftRadius: "8px",
      borderTopRightRadius: "8px",
      borderBottomRightRadius: "0",
      padding: "8px 12px",
      fontWeight: 700,
      fontSize: "14px",
      letterSpacing: "1px",
      whiteSpace: "nowrap",
      color: "var(--vibes-card-text)",
      writingMode: "horizontal-tb",
      transform: "none",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      flexShrink: 0,
      width: "calc(100% - 64px)",
      margin: "0px 32px",
    };
  }
  return {
    background: "var(--vibes-card-bg)",
    border: "2px solid var(--vibes-card-border)",
    borderLeft: "none",
    borderBottom: "2px solid var(--vibes-card-border)",
    borderTopRightRadius: "8px",
    borderBottomRightRadius: "8px",
    borderTopLeftRadius: "0",
    padding: "12px 8px",
    fontWeight: 700,
    fontSize: "14px",
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
    width: "auto",
  };
}

export function getResponsiveButtonWrapperStyle(
  isMobile: boolean,
  disappear = false,
): React.CSSProperties {
  if (isMobile && disappear) {
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
  if (isMobile && !disappear) {
    return {
      background: "var(--vibes-card-bg)",
      border: "2px solid var(--vibes-card-border)",
      borderRadius: "8px",
      padding: "24px 24px 32px 24px",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      width: "100%",
    };
  }
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

export function getResponsiveContainerStyle(
  isMobile: boolean,
): React.CSSProperties {
  if (isMobile) {
    return {
      position: "relative",
      display: "inline-flex",
      alignItems: "stretch",
      flexDirection: "column",
      width: "100%",
      marginBottom: "40px",
    };
  }
  return {
    position: "relative",
    display: "inline-flex",
    alignItems: "stretch",
    flexDirection: "row",
    width: "auto",
    marginBottom: "40px",
  };
}
