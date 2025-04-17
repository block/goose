export function formatMessageTimestamp(timestamp: number): string {
  // Convert from Unix timestamp (seconds) to milliseconds
  const date = new Date(timestamp * 1000);

  // Format date as MM/DD/YYYY
  const dateStr = date.toLocaleDateString('en-US', {
    month: '2-digit',
    day: '2-digit',
    year: 'numeric',
  });

  // Format time as HH:MM:SS AM/PM
  const timeStr = date.toLocaleTimeString('en-US', {
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
    hour12: true,
  });

  return `${dateStr} ${timeStr}`;
}
