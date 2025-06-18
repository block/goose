export default function ChevronUp({ className = '' }) {
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
        d="M17.293 9.293a1 1 0 0 1 1.414 0l5 5a1 1 0 0 1-1.414 1.414L18 11.414l-4.293 4.293a1 1 0 0 1-1.414-1.414l5-5Z"
        fill="currentColor"
      ></path>
    </svg>
  );
}
