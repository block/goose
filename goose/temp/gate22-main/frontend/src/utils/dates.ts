export const utcDateOffsetByDays = (days: number) => {
  const date = new Date();
  date.setUTCHours(0, 0, 0, 0);
  date.setUTCDate(date.getUTCDate() + days);
  return date;
};

export const localtimeDateOffsetByDays = (days: number) => {
  const date = new Date();
  date.setHours(0, 0, 0, 0);
  date.setDate(date.getDate() + days);
  return date;
};
export const utcDate = (localDateTime: Date) =>
  new Date(
    Date.UTC(localDateTime.getFullYear(), localDateTime.getMonth(), localDateTime.getDate()),
  );

export const setBeginningOfDay = (date: Date) => {
  date.setHours(0, 0, 0, 0);
  return date;
};

export const setEndOfDay = (date: Date) => {
  date.setHours(23, 59, 59, 999);
  return date;
};

export const intervalInSeconds = (start: Date, end: Date | null) =>
  end ? (end.getTime() - start.getTime()) / 1000 : 0;

export const formatIntervalSeconds = (seconds: number, scale: number = 2) => {
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);
  const pad = (num: number) => `00${num}`.slice(2);

  if (hrs > 0) return `${hrs}h ${pad(mins)}m ${pad(secs)}s`;
  if (mins > 0) return `${mins}m ${pad(secs)}s`;
  return `${seconds.toFixed(scale)}s`;
};

export const getShortLocalTimezone = () => {
  return new Date().toLocaleTimeString("en-us", { timeZoneName: "short" }).split(" ")[2];
};

export const getTimezoneDetails = () => {
  const longLocalTz = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const location = longLocalTz.replace(/_/g, " ");
  const utcDifference = -(new Date().getTimezoneOffset() / 60); // negative because TZ info is the opposite of UTC offset
  return `${location} (UTC${utcDifference >= 0 ? "+" : ""}${utcDifference})`;
};

export const getRelativeTimestampFromNow = (timestamp: Date): string => {
  const diffInMs = new Date().getTime() - timestamp.getTime();
  const diffInMinutes = diffInMs / (1000 * 60);
  const diffInHours = diffInMinutes / 60;
  const diffInDays = diffInHours / 24;

  if (diffInHours < 1) {
    return `${Math.floor(diffInMinutes)} minutes ago`;
  } else if (diffInHours < 24) {
    return `${Math.floor(diffInHours)} hours ago`;
  } else if (diffInDays < 7) {
    return `${Math.floor(diffInDays)} days ago`;
  } else {
    return timestamp.toLocaleDateString("en-US", {
      year: "2-digit",
      month: "numeric",
      day: "numeric",
    });
  }
};
