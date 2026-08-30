import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  activityLevel,
  buildYearHeatmap,
  heatmapWeekCount,
  localDateKey,
  parseLocalDateKey,
  startOfLocalWeek,
} from './activityHeatmap';

describe('activityHeatmap', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('builds a Sunday-start grid covering the calendar year', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2024, 5, 15));
    const cells = buildYearHeatmap(2024);
    expect(cells[0].weekday).toBe(0);
    expect(cells.some((cell) => cell.date === '2024-01-01' && cell.inYear)).toBe(true);
    expect(cells.some((cell) => cell.date === '2024-12-31' && cell.inYear)).toBe(true);
    expect(heatmapWeekCount(cells)).toBeGreaterThanOrEqual(52);
    expect(cells.filter((cell) => cell.inYear)).toHaveLength(366);
  });

  it('keeps weekdays Sunday-first', () => {
    const monday = parseLocalDateKey('2024-01-01');
    expect(monday.getDay()).toBe(1);
    expect(localDateKey(startOfLocalWeek(monday))).toBe('2023-12-31');
  });

  it('maps session counts onto five intensity levels', () => {
    expect(activityLevel(0, 8)).toBe(0);
    expect(activityLevel(1, 8)).toBe(1);
    expect(activityLevel(2, 8)).toBe(1);
    expect(activityLevel(4, 8)).toBe(2);
    expect(activityLevel(6, 8)).toBe(3);
    expect(activityLevel(8, 8)).toBe(4);
  });
});
