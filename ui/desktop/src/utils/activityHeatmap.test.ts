import { describe, expect, it } from 'vitest';
import {
  activityLevel,
  buildYearHeatmap,
  heatmapWeekCount,
  parseUtcDateKey,
  startOfUtcWeek,
  utcDateKey,
} from './activityHeatmap';

describe('activityHeatmap', () => {
  it('builds a Sunday-start grid covering the calendar year', () => {
    const cells = buildYearHeatmap(2024);
    expect(cells[0].date).toBe('2023-12-31');
    expect(cells[0].inYear).toBe(false);
    expect(cells.some((cell) => cell.date === '2024-01-01' && cell.inYear)).toBe(true);
    expect(cells.some((cell) => cell.date === '2024-12-31' && cell.inYear)).toBe(true);
    expect(heatmapWeekCount(cells)).toBeGreaterThanOrEqual(52);
    expect(cells.filter((cell) => cell.inYear)).toHaveLength(366);
  });

  it('keeps weekdays Sunday-first', () => {
    const monday = parseUtcDateKey('2024-01-01');
    expect(utcDateKey(startOfUtcWeek(monday))).toBe('2023-12-31');
    expect(monday.getUTCDay()).toBe(1);
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
