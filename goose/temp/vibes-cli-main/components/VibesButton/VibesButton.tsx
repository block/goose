import React, { useEffect, useState } from "react";
import {
  getButtonStyle,
  getMergedButtonStyle,
  getIconContainerStyle,
  getIconStyle,
  getContentWrapperStyle,
  bounceKeyframes,
} from "./VibesButton.styles.js";
import {
  LoginIcon,
  RemixIcon,
  InviteIcon,
  SettingsIcon,
  BackIcon,
  GoogleIcon,
  GitHubIcon,
} from "../icons/index.js";
import { useMobile } from "../mocks/use-vibes-base.js";

export const BLUE = "blue" as const;
export const RED = "red" as const;
export const YELLOW = "yellow" as const;
export const GRAY = "gray" as const;

type ButtonVariant = "blue" | "red" | "yellow" | "gray";
type ButtonType = "square" | "flat" | "flat-rounded" | "form";
type IconName =
  | "login"
  | "remix"
  | "invite"
  | "settings"
  | "back"
  | "google"
  | "github";

const iconMap: Record<
  IconName,
  React.ComponentType<{
    bgFill?: string;
    fill?: string;
    width?: number;
    height?: number;
    withCircle?: boolean;
  }>
> = {
  login: LoginIcon,
  remix: RemixIcon,
  invite: InviteIcon,
  settings: SettingsIcon,
  back: BackIcon,
  google: GoogleIcon,
  github: GitHubIcon,
};

export interface MenuButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  buttonType?: ButtonType;
  children: React.ReactNode;
  onHover?: () => void;
  onUnhover?: () => void;
  icon?: IconName;
  ignoreDarkMode?: boolean;
  formColor?: string;
}

export function VibesButton({
  variant = "blue",
  buttonType = "square",
  children,
  onHover,
  onUnhover,
  icon,
  style: customStyle,
  className = "",
  ignoreDarkMode = false,
  formColor,
  ...props
}: MenuButtonProps) {
  const buttonVariant = variant;
  const [isHovered, setHovered] = useState(false);
  const [isActive, setActive] = useState(false);
  const isMobile = useMobile();

  useEffect(() => {
    if (isHovered) {
      onHover?.();
    } else {
      onUnhover?.();
    }
  }, [isHovered, onHover, onUnhover]);

  const IconComponent = icon ? iconMap[icon] : undefined;

  const baseStyle = getButtonStyle(
    buttonVariant,
    isHovered,
    isActive,
    isMobile,
    !!IconComponent,
    buttonType,
    formColor,
  );
  const mergedStyle = getMergedButtonStyle(
    baseStyle,
    ignoreDarkMode,
    customStyle,
    buttonType,
  );
  const iconContainerStyle = getIconContainerStyle(
    buttonVariant,
    isMobile,
    !!IconComponent,
    buttonType,
  );
  const iconStyle = getIconStyle(isMobile, isHovered, isActive);
  const contentWrapperStyle = getContentWrapperStyle(
    isMobile,
    !!IconComponent,
    buttonType,
  );

  return (
    <>
      <style>{bounceKeyframes}</style>
      <button
        {...props}
        className={className}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => {
          setHovered(false);
          setActive(false);
        }}
        onMouseDown={() => setActive(true)}
        onMouseUp={() => setActive(false)}
        style={mergedStyle}
      >
        {IconComponent ? (
          <div style={contentWrapperStyle}>
            <div style={iconContainerStyle}>
              <div style={iconStyle}>
                <IconComponent
                  bgFill="var(--vibes-button-icon-bg)"
                  fill={icon === "google" || icon === "github" ? "#000" : "var(--vibes-button-icon-fill)"}
                  width={
                    buttonType === "flat-rounded" ? 28 : isMobile ? 28 : 45
                  }
                  height={
                    buttonType === "flat-rounded" ? 28 : isMobile ? 28 : 45
                  }
                  withCircle={icon === "back"}
                />
              </div>
            </div>
            <span>{children}</span>
          </div>
        ) : (
          children
        )}
      </button>
    </>
  );
}
