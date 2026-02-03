export default function WindowsSvg({ className = "" }: { className?: string }) {
	return (
		<svg role="img" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" height="24" width="24" className={`${className} external-icon-svg`}>
			<path d="M3 3h8.5v8.5H3V3zm9.5 0H21v8.5h-8.5V3zM3 12.5h8.5V21H3v-8.5zm9.5 0H21V21h-8.5v-8.5z" fill="currentColor" />
		</svg>
	);
}