interface ArrowRightProps {
  className?: string;
}

export function ArrowRight({ className = '' }: ArrowRightProps) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <path
        d="M3.33678 8.00138L12.6701 8.00555M12.6701 8.00555L8.00553 3.3368M12.6701 8.00555L8.00136 12.6701"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
