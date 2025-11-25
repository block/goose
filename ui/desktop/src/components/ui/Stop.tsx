interface StopProps {
  size?: number;
  className?: string;
}

export default function Stop({ size = 24, className }: StopProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <rect x="6" y="6" width="12" height="12" fill="currentColor" rx="1" />
    </svg>
  );
}
