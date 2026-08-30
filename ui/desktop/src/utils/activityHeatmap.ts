export const ACTIVITY_WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'] as const;

export interface HeatmapCell {
  date: string;
  weekday: number;
  weekIndex: number;
  inYear: boolean;
}

export function utcDateKey(date: Date): string {
  return date.toISOString().slice(0, 10);
}

export function parseUtcDateKey(date: string): Date {
  const [year, month, day] = date.split('-').map(Number);
  return new Date(Date.UTC(year, month - 1, day));
}

export function startOfUtcWeek(date: Date): Date {
  const start = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
  start.setUTCDate(start.getUTCDate() - start.getUTCDay());
  return start;
}

export function buildYearHeatmap(year: number): HeatmapCell[] {
  const yearStart = new Date(Date.UTC(year, 0, 1));
  const yearEnd = new Date(Date.UTC(year, 11, 31));
  const gridStart = startOfUtcWeek(yearStart);
  const gridEnd = startOfUtcWeek(yearEnd);
  gridEnd.setUTCDate(gridEnd.getUTCDate() + 6);

  const cells: HeatmapCell[] = [];
  const cursor = new Date(gridStart);
  let weekIndex = 0;

  while (cursor.getTime() <= gridEnd.getTime()) {
    const weekday = cursor.getUTCDay();
    cells.push({
      date: utcDateKey(cursor),
      weekday,
      weekIndex,
      inYear: cursor.getUTCFullYear() === year,
    });
    cursor.setUTCDate(cursor.getUTCDate() + 1);
    if (weekday === 6) {
      weekIndex += 1;
    }
  }

  return cells;
}

export function heatmapWeekCount(cells: HeatmapCell[]): number {
  return cells.reduce((max, cell) => Math.max(max, cell.weekIndex + 1), 0);
}

export function activityLevel(sessionCount: number, maxSessionCount: number): 0 | 1 | 2 | 3 | 4 {
  if (sessionCount <= 0 || maxSessionCount <= 0) {
    return 0;
  }
  const ratio = sessionCount / maxSessionCount;
  if (ratio <= 0.25) return 1;
  if (ratio <= 0.5) return 2;
  if (ratio <= 0.75) return 3;
  return 4;
}
