export const ACTIVITY_WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'] as const;

export interface HeatmapCell {
  date: string;
  weekday: number;
  weekIndex: number;
  inYear: boolean;
}

function pad(value: number): string {
  return String(value).padStart(2, '0');
}

export function localDateKey(date: Date): string {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

export function parseLocalDateKey(date: string): Date {
  const [year, month, day] = date.split('-').map(Number);
  return new Date(year, month - 1, day);
}

export function startOfLocalWeek(date: Date): Date {
  const start = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  start.setDate(start.getDate() - start.getDay());
  return start;
}

export function buildYearHeatmap(year: number): HeatmapCell[] {
  const yearStart = new Date(year, 0, 1);
  const yearEnd = new Date(year, 11, 31);
  const gridStart = startOfLocalWeek(yearStart);
  const gridEnd = startOfLocalWeek(yearEnd);
  gridEnd.setDate(gridEnd.getDate() + 6);

  const cells: HeatmapCell[] = [];
  const cursor = new Date(gridStart);
  let weekIndex = 0;

  while (cursor.getTime() <= gridEnd.getTime()) {
    const weekday = cursor.getDay();
    cells.push({
      date: localDateKey(cursor),
      weekday,
      weekIndex,
      inYear: cursor.getFullYear() === year,
    });
    cursor.setDate(cursor.getDate() + 1);
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
