export const Brain = ({ className }: { className?: string }) => (
  <svg
    width="11"
    height="11"
    viewBox="0 0 11 11"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={className}
  >
    <rect width="11" height="11" rx="2" fill="#3E3E3E" />
    <rect width="11" height="11" rx="2" fill="url(#paint0_linear_6313_782)" />
    <path d="M5.5 2.0625L1.375 5.5L5.5 8.9375L9.625 5.5L5.5 2.0625Z" fill="#E74786" />
    <defs>
      <linearGradient
        id="paint0_linear_6313_782"
        x1="5.5"
        y1="0"
        x2="5.5"
        y2="11"
        gradientUnits="userSpaceOnUse"
      >
        <stop />
        <stop offset="1" stopColor="#323232" />
      </linearGradient>
    </defs>
  </svg>
);
