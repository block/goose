import React from 'react';

export default function Plus({ size }: { size: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      style={{ height: size, width: size }}
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      fill="none"
    >
      <path
        d="M9.75 14.25V9.75H14.25V8.25H9.75V3.75H8.25V8.25H3.75V9.75H8.25V14.25H9.75Z"
        fill="black"
        dark:fill="white"
        fillOpacity="0.25"
      />
    </svg>
  );
}
