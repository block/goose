/** Avocado Work mark — same path as landing logo.svg / desktop glyph.svg */
export function Avocado({ className = '' }: { className?: string }) {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 28 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      role="img"
      aria-label="Avocado"
    >
      <path
        fill="currentColor"
        fillRule="evenodd"
        d="M14 0C6.6 0 0.6 5.8 0.15 13.2C-0.3 22.6 5.6 32 14 32C22.4 32 28.3 22.6 27.85 13.2C27.4 5.8 21.4 0 14 0ZM14 22.4a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9Z"
      />
    </svg>
  );
}
