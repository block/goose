import { currentLocaleTag } from '../i18n';

export function formatMessageTimestamp(timestamp?: number): string {
  const date = timestamp ? new Date(timestamp * 1000) : new Date();
  const now = new Date();
  const localeTag = currentLocaleTag();

  const timeStr = date.toLocaleTimeString(localeTag, {
    hour: 'numeric',
    minute: '2-digit',
  });

  // Check if the message is from today
  if (
    date.getDate() === now.getDate() &&
    date.getMonth() === now.getMonth() &&
    date.getFullYear() === now.getFullYear()
  ) {
    return timeStr;
  }

  const dateStr = date.toLocaleDateString(localeTag, {
    month: '2-digit',
    day: '2-digit',
    year: 'numeric',
  });

  return `${dateStr} ${timeStr}`;
}
