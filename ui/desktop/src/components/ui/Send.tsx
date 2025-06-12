export default function Send({ size }: { size: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      style={{ height: size, width: size }}
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      fill="none"
    >
      <path
        d="M22 12.5L2 4.5L4 12.5L2 20.5L22 12.5ZM5.81 13.5H14.11L4.88 17.19L5.81 13.5ZM14.11 11.5H5.81L4.89 7.81L14.11 11.5Z"
        fill="#7A7EFB"
        className="dark:fill-[#4A56E2]"
      />
    </svg>
  );
}
