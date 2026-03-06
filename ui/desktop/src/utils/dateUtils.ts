import { Session } from '../api';

export interface DateGroup {
  label: string;
  sessions: Session[];
  date: Date;
}

interface GroupSessionsOptions {
  locale?: string;
  todayLabel?: string;
  yesterdayLabel?: string;
}

export function groupSessionsByDate(
  sessions: Session[],
  options: GroupSessionsOptions = {}
): DateGroup[] {
  const { locale = 'en-US', todayLabel = 'Today', yesterdayLabel = 'Yesterday' } = options;
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  const groups: { [key: string]: DateGroup } = {};

  sessions.forEach((session) => {
    const sessionDate = new Date(session.updated_at);
    const sessionDateStart = new Date(sessionDate);
    sessionDateStart.setHours(0, 0, 0, 0);

    let label: string;
    let groupKey: string;

    if (sessionDateStart.getTime() === today.getTime()) {
      label = todayLabel;
      groupKey = 'today';
    } else if (sessionDateStart.getTime() === yesterday.getTime()) {
      label = yesterdayLabel;
      groupKey = 'yesterday';
    } else {
      // Format as "Monday, January 1" or "January 1" if it's not this year
      const currentYear = today.getFullYear();
      const sessionYear = sessionDateStart.getFullYear();

      if (sessionYear === currentYear) {
        label = sessionDateStart.toLocaleDateString(locale, {
          weekday: 'long',
          month: 'long',
          day: 'numeric',
        });
      } else {
        label = sessionDateStart.toLocaleDateString(locale, {
          month: 'long',
          day: 'numeric',
          year: 'numeric',
        });
      }
      groupKey = sessionDateStart.toISOString().split('T')[0];
    }

    if (!groups[groupKey]) {
      groups[groupKey] = {
        label,
        sessions: [],
        date: sessionDateStart,
      };
    }

    groups[groupKey].sessions.push(session);
  });

  // Convert to array and sort by date (newest first)
  return Object.values(groups).sort((a, b) => b.date.getTime() - a.date.getTime());
}
