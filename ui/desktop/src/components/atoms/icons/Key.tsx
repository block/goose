interface KeyProps {
  className?: string;
}

export function Key({ className = '' }: KeyProps) {
  return (
    <svg
      width="16"
      height="17"
      viewBox="0 0 16 17"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <g clipPath="url(#clip0_986_19974)">
        <path
          d="M10.3382 5.68265L11.8709 7.21667C11.9954 7.33888 12.1629 7.40737 12.3374 7.40745C12.5119 7.40753 12.6795 7.33918 12.8042 7.21709L14.2048 5.81771C14.327 5.69315 14.3955 5.52563 14.3956 5.35113C14.3957 5.17663 14.3273 5.00905 14.2052 4.88438L12.6726 3.35036M14.0064 2.01762L7.60358 8.41476M8.66915 11.0152C8.66825 13.0403 7.02589 14.6812 5.00085 14.6803C2.9758 14.6794 1.33492 13.037 1.33582 11.012C1.33673 8.98692 2.97908 7.34603 5.00413 7.34693C7.02917 7.34784 8.67006 8.9902 8.66915 11.0152Z"
          stroke="currentColor"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </g>
      <defs>
        <clipPath id="clip0_986_19974">
          <rect
            width="16"
            height="16"
            fill="white"
            transform="translate(0.00708008 0.677979) rotate(0.0256089)"
          />
        </clipPath>
      </defs>
    </svg>
  );
}
