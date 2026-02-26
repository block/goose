export default function CoinIcon({ className = '', size = 16 }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      {/* Main coin circle */}
      <circle
        cx="10"
        cy="10"
        r="9"
        fill="currentColor"
        opacity="0.1"
        stroke="currentColor"
        strokeWidth="1.5"
      />

      {/* Inner circle for depth */}
      <circle
        cx="10"
        cy="10"
        r="6.5"
        fill="none"
        stroke="currentColor"
        strokeWidth="0.8"
        opacity="0.4"
      />

      {/* Scalloped edge decoration - 8 evenly spaced notches */}
      <circle cx="10" cy="1.5" r="1" fill="currentColor" opacity="0.3" />
      <circle cx="15.7" cy="4.3" r="1" fill="currentColor" opacity="0.3" />
      <circle cx="18.5" cy="10" r="1" fill="currentColor" opacity="0.3" />
      <circle cx="15.7" cy="15.7" r="1" fill="currentColor" opacity="0.3" />
      <circle cx="10" cy="18.5" r="1" fill="currentColor" opacity="0.3" />
      <circle cx="4.3" cy="15.7" r="1" fill="currentColor" opacity="0.3" />
      <circle cx="1.5" cy="10" r="1" fill="currentColor" opacity="0.3" />
      <circle cx="4.3" cy="4.3" r="1" fill="currentColor" opacity="0.3" />

      {/* Dollar sign */}
      <g fill="currentColor" stroke="none">
        {/* Vertical line */}
        <rect x="9.3" y="5" width="1.4" height="10" />
        {/* Top S curve */}
        <path
          d="M7 7.5 Q7 6.5 8 6.5 L12 6.5 Q13 6.5 13 7.5 Q13 8.5 12 8.5 L8.5 8.5"
          strokeWidth="1.2"
          stroke="currentColor"
          fill="none"
        />
        {/* Bottom S curve */}
        <path
          d="M13 12.5 Q13 13.5 12 13.5 L8 13.5 Q7 13.5 7 12.5 Q7 11.5 8 11.5 L11.5 11.5"
          strokeWidth="1.2"
          stroke="currentColor"
          fill="none"
        />
      </g>
    </svg>
  );
}
