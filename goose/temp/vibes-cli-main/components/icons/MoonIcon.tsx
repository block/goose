import React from "react";

interface MoonIconProps {
  fill?: string;
  width?: number;
  height?: number;
}

export function MoonIcon({
  fill = "currentColor",
  width = 24,
  height = 24,
}: MoonIconProps) {
  return (
    <svg
      width={width}
      height={height}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M21.752 15.002A9.718 9.718 0 0112.478 3.5a10.013 10.013 0 00-7.715 4.754A10.003 10.003 0 0012 22c3.516 0 6.63-1.817 8.426-4.564a9.724 9.724 0 01.326-.434z"
        fill={fill}
      />
    </svg>
  );
}
