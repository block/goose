import { CSSProperties } from "react";

export const getOverlayStyle = (): CSSProperties => ({
  position: "fixed",
  top: 0,
  left: 0,
  right: 0,
  bottom: 0,
  display: "flex",
  justifyContent: "center",
  alignItems: "center",
  zIndex: 1000,
  overflow: "hidden",
});

export const getGridBackgroundStyle = (): CSSProperties => ({
  position: "absolute",
  top: 0,
  left: 0,
  right: 0,
  bottom: 0,
  backgroundColor: "rgba(0, 0, 0, 0.5)",
  zIndex: 0,
});

export const getBlackBorderWrapperStyle = (): CSSProperties => ({
  position: "relative",
  width: "90%",
  maxWidth: "550px",
  backgroundImage: `
    linear-gradient(to right, rgba(0, 0, 0, 0.1) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(0, 0, 0, 0.1) 1px, transparent 1px)
  `,
  backgroundSize: "40px 40px",
  backgroundColor: "#e8e4df",
  border: "3px solid #1a1a1a",
  borderRadius: "12px",
  zIndex: 1,
  overflow: "hidden",
});

export const getContainerStyle = (): CSSProperties => ({
  display: "flex",
  flexDirection: "column",
  justifyContent: "center",
  alignItems: "center",
  minHeight: "500px",
  width: "100%",
  gap: "2rem",
  padding: "3rem 0rem",
  position: "relative",
});

export const getBackgroundStyle = (isShredding: boolean): CSSProperties => ({
  position: "absolute",
  top: "1.5rem",
  left: "1.5rem",
  right: "1.5rem",
  bottom: "1.5rem",
  backgroundColor: "var(--vibes-gray-lighter, #c4c4c4)",
  border: "1px solid black",
  borderRadius: "8px",
  zIndex: 0,
  transformOrigin: "center center",
  animation: isShredding ? "collapseToLine 1.2s ease-in-out forwards" : "none",
  pointerEvents: "none",
});

export const getCardIconStyle = (isShredding: boolean): CSSProperties => ({
  marginBottom: "1rem",
  animation: isShredding ? "shredCard 0.9s ease-in forwards" : "none",
  position: "relative",
  zIndex: 1,
});

export const getCardIconAnimationStyles = (): string => `
  @keyframes shredCard {
    0% {
      clip-path: inset(0 0 0% 0);
      transform: translateY(0);
    }
    45% {
      clip-path: inset(0 0 0% 0);
      transform: translateY(0);
    }
    80% {
      clip-path: inset(0 0 100% 0);
      transform: translateY(310px);
    }
    100% {
      clip-path: inset(0 0 100% 0);
      transform: translateY(310px);
    }
  }

  @keyframes collapseToLine {
    0% {
      transform: scale(1);
      border-radius: 8px;
      background-color: var(--vibes-gray-lighter, #c4c4c4);
    }
    40% {
      transform: scaleX(0.05) scaleY(0.01);
      border-radius: 50%;
      background-color: black;
    }
    45% {
      transform: scaleX(0.05) scaleY(0.01);
      border-radius: 50%;
      background-color: black;
    }
    65% {
      transform: scaleX(0.6) scaleY(0.01);
      border-radius: 0;
    }
    80% {
      transform: scaleX(0.6) scaleY(0.01);
      border-radius: 0;
    }
    100% {
      transform: scaleX(0) scaleY(0.01);
      border-radius: 0;
      background-color: black;
    }
  }
`;

export const getButtonsContainerStyle = (): CSSProperties => ({
  display: "flex",
  flexDirection: "column",
  gap: "1rem",
  width: "100%",
  maxWidth: "400px",
  position: "relative",
  zIndex: 1,
});

export const getButtonsCenterWrapperStyle = (): CSSProperties => ({
  display: "flex",
  flexDirection: "column",
  gap: "1rem",
  width: "100%",
  position: "relative",
  paddingTop: "1rem",
  paddingBottom: "1rem",
  zIndex: 1,
});

export const getCloseButtonStyle = (): CSSProperties => ({
  position: "absolute",
  top: "1rem",
  right: "1rem",
  background: "white",
  height: "30px",
  width: "30px",
  display: "flex",
  justifyContent: "center",
  alignItems: "center",
  borderRadius: "50%",
  border: "none",
  fontSize: "1.5rem",
  cursor: "pointer",
  zIndex: 10,
  color: "#1a1a1a",
  lineHeight: 1,
  padding: "0.25rem",
});
