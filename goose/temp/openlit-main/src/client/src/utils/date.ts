export function formatDate(dateString: string, options?: { time?: boolean }) {
	const date = new Date(dateString);
	return date.toLocaleDateString(undefined, {
		year: "numeric",
		month: "short",
		day: "numeric",
		...(options?.time && {
			hour: "2-digit",
			minute: "2-digit",
			second: "2-digit",
		}),
	});
}