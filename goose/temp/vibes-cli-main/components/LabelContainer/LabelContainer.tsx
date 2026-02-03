import React from "react";
import {
  getResponsiveContainerStyle,
  getResponsiveLabelStyle,
  getResponsiveButtonWrapperStyle,
} from "./LabelContainer.styles.js";
import { useMobile } from "../mocks/use-vibes-base.js";

export interface LabelContainerProps {
  label?: string;
  children: React.ReactNode;
  style?: React.CSSProperties;
  className?: string;
  disappear?: boolean;
}

export function LabelContainer({
  label,
  children,
  style,
  className,
  disappear = false,
}: LabelContainerProps) {
  const isMobile = useMobile();

  return (
    <div
      style={{ ...getResponsiveContainerStyle(isMobile), ...style }}
      className={className}
    >
      {label && (
        <div style={getResponsiveLabelStyle(isMobile, disappear)}>{label}</div>
      )}
      <div style={getResponsiveButtonWrapperStyle(isMobile, disappear)}>
        {children}
      </div>
    </div>
  );
}
