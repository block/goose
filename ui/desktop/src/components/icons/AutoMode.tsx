import React from 'react';

const AutoMode = ({ className = '' }) => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 16 16"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={className}
  >
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M2.75 4.5C2.75 3.5335 3.5335 2.75 4.5 2.75C5.4665 2.75 6.25 3.5335 6.25 4.5C6.25 5.4665 5.4665 6.25 4.5 6.25C3.5335 6.25 2.75 5.4665 2.75 4.5ZM4.5 1.25C2.70507 1.25 1.25 2.70507 1.25 4.5C1.25 6.29493 2.70507 7.75 4.5 7.75C6.29493 7.75 7.75 6.29493 7.75 4.5C7.75 2.70507 6.29493 1.25 4.5 1.25ZM10.5 14C12.433 14 14 12.433 14 10.5C14 8.567 12.433 7 10.5 7C8.567 7 7 8.567 7 10.5C7 12.433 8.567 14 10.5 14Z"
      fill="currentColor"
    />
    <circle
      cx="11"
      cy="3"
      r="1"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
);

export default AutoMode;
