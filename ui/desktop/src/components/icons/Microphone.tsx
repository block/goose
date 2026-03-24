import React from 'react';

interface MicrophoneProps {
  className?: string;
  size?: number;
}

export const Microphone: React.FC<MicrophoneProps> = ({ className = '', size = 24 }) => {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <path
        d="M12 14.5C13.66 14.5 15 13.16 15 11.5V5.5C15 3.84 13.66 2.5 12 2.5C10.34 2.5 9 3.84 9 5.5V11.5C9 13.16 10.34 14.5 12 14.5Z"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M19 11.5C19 15.09 16.09 18 12.5 18C8.91 18 6 15.09 6 11.5"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M12 18V21.5"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M8 21.5H16"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
};
