import { currentLocale } from '../i18n';

let use24HourClock = false;

export function initTimeFormat(use24h: boolean): void {
  use24HourClock = use24h;
}

export function setTimeFormat(use24h: boolean): void {
  use24HourClock = use24h;
}

export function formatMessageTimestamp(timestamp?: number): string {
  const date = timestamp ? new Date(timestamp * 1000) : new Date();
  const now = new Date();

  const timeStr = date.toLocaleTimeString(currentLocale, {
    hour: 'numeric',
    minute: '2-digit',
    hour12: !use24HourClock,
  });

  // Check if the message is from today
  if (
    date.getDate() === now.getDate() &&
    date.getMonth() === now.getMonth() &&
    date.getFullYear() === now.getFullYear()
  ) {
    return timeStr;
  }

  // If not today, format as localized date + time
  const dateStr = date.toLocaleDateString(currentLocale, {
    month: '2-digit',
    day: '2-digit',
    year: 'numeric',
  });

  return `${dateStr} ${timeStr}`;
}
