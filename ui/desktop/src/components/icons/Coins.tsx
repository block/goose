export default function Coins({ className = '' }) {
  return (
    <svg
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      aria-hidden="true"
      className={className}
    >
      <g fill="currentColor">
        {/* First coin (back) */}
        <ellipse cx="8" cy="12" rx="6" ry="2.5" opacity="0.5" />
        <path
          d="M2 12v4c0 1.38 2.69 2.5 6 2.5s6-1.12 6-2.5v-4c0 1.38-2.69 2.5-6 2.5S2 13.38 2 12Z"
          opacity="0.5"
        />

        {/* Second coin (front) */}
        <ellipse cx="16" cy="8" rx="6" ry="2.5" />
        <path d="M10 8v4c0 1.38 2.69 2.5 6 2.5s6-1.12 6-2.5V8c0 1.38-2.69 2.5-6 2.5s-6-1.12-6-2.5Z" />
        <path d="M10 12v4c0 1.38 2.69 2.5 6 2.5s6-1.12 6-2.5v-4c0 1.38-2.69 2.5-6 2.5s-6-1.12-6-2.5Z" />
      </g>
    </svg>
  );
}
