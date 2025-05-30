export default function VertDots({ size }: { size: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      fill="none"
      className="text-gray-600 dark:text-prev-goose-text-dark"
    >
      <path
        d="M10.4976 4.5C10.4976 5.32843 9.82599 6 8.99756 6C8.16913 6 7.49756 5.32843 7.49756 4.5C7.49756 3.67157 8.16913 3 8.99756 3C9.82599 3 10.4976 3.67157 10.4976 4.5Z"
        fill="currentColor"
      />
      <path
        d="M10.4976 9C10.4976 9.82843 9.82599 10.5 8.99756 10.5C8.16913 10.5 7.49756 9.82843 7.49756 9C7.49756 8.17157 8.16913 7.5 8.99756 7.5C9.82599 7.5 10.4976 8.17157 10.4976 9Z"
        fill="currentColor"
      />
      <path
        d="M8.99756 15C9.82599 15 10.4976 14.3284 10.4976 13.5C10.4976 12.6716 9.82599 12 8.99756 12C8.16913 12 7.49756 12.6716 7.49756 13.5C7.49756 14.3284 8.16913 15 8.99756 15Z"
        fill="currentColor"
      />
    </svg>
  );
}
