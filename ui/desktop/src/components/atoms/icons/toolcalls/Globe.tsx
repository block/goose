export const Globe = ({ className }: { className?: string }) => (
  <svg
    width="11"
    height="11"
    viewBox="0 0 11 11"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={className}
  >
    <g clipPath="url(#clip0_6313_720)">
      <rect width="11" height="11" rx="2" fill="white" />
      <path
        d="M10.3125 5.5C10.3125 8.15787 8.15787 10.3125 5.5 10.3125C2.84213 10.3125 0.6875 8.15787 0.6875 5.5C0.6875 2.84213 2.84213 0.6875 5.5 0.6875C8.15787 0.6875 10.3125 2.84213 10.3125 5.5Z"
        fill="url(#paint0_linear_6313_720)"
      />
    </g>
    <defs>
      <linearGradient
        id="paint0_linear_6313_720"
        x1="5.5"
        y1="0.6875"
        x2="5.5"
        y2="10.3125"
        gradientUnits="userSpaceOnUse"
      >
        <stop stopColor="#00CAF7" />
        <stop offset="1" stopColor="#0B54DE" />
      </linearGradient>
      <clipPath id="clip0_6313_720">
        <rect width="11" height="11" rx="2" fill="white" />
      </clipPath>
    </defs>
  </svg>
);
