export default function X({ size }: { size: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      fill="none"
    >
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M5.97237 6.00001L3.82593 3.85356L4.53303 3.14645L6.67948 5.2929L8.82593 3.14645L9.53303 3.85356L7.38659 6.00001L9.53303 8.14645L8.82593 8.85356L6.67948 6.70711L4.53303 8.85356L3.82593 8.14645L5.97237 6.00001Z"
        fill="black"
        className="dark:fill-white"
        fillOpacity="0.6"
      />
    </svg>
  );
}
