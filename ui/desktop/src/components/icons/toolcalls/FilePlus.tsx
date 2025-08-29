export const FilePlus = ({ className }: { className?: string }) => (
  <svg
    width="11"
    height="11"
    viewBox="0 0 11 11"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={className}
  >
    <rect width="11" height="11" rx="2" fill="url(#paint0_linear_6313_774)" />
    <rect x="2" y="5" width="7" height="1" rx="0.5" fill="white" />
    <rect x="6" y="2" width="7" height="1" rx="0.5" transform="rotate(90 6 2)" fill="white" />
    <defs>
      <linearGradient
        id="paint0_linear_6313_774"
        x1="5.5"
        y1="0"
        x2="5.5"
        y2="11"
        gradientUnits="userSpaceOnUse"
      >
        <stop stopColor="#FF9A00" />
        <stop offset="1" stopColor="#FFC800" />
      </linearGradient>
    </defs>
  </svg>
);
