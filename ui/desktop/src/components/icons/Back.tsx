import React from 'react';

export default function Back({ className = '' }) {
  return (
    <svg
      width="1.5rem"
      height="1.5rem"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      aria-hidden="true"
      className={className}
    >
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M9.56 3.94a1.5 1.5 0 0 1 0 2.12L3.622 12l5.94 5.94a1.5 1.5 0 0 1-2.122 2.12l-7-7a1.5 1.5 0 0 1 0-2.12l7-7a1.5 1.5 0 0 1 2.122 0Z"
        fill="currentColor"
      ></path>
    </svg>
  );
}
