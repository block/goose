export function formatMessageTimestamp(timestamp: number): string {
  // Convert from Unix timestamp (seconds) to milliseconds
  const date = new Date(timestamp * 1000);
  const now = new Date();

  // Format time as HH:MM AM/PM
  const timeStr = date.toLocaleTimeString('en-US', {
    hour: 'numeric',
    minute: '2-digit',
    hour12: true,
  });

  // Check if the message is from today
  if (
    date.getDate() === now.getDate() &&
    date.getMonth() === now.getMonth() &&
    date.getFullYear() === now.getFullYear()
  ) {
    return timeStr;
  }

  // If not today, format as MM/DD/YYYY HH:MM AM/PM
  const dateStr = date.toLocaleDateString('en-US', {
    month: '2-digit',
    day: '2-digit',
    year: 'numeric',
  });

  return `${dateStr} ${timeStr}`;
}
