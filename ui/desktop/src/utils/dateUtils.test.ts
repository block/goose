import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Session } from '../api';
import { groupSessionsByDate } from './dateUtils';

function createSession(id: string, updatedAt: string): Session {
  return {
    id,
    updated_at: updatedAt,
    name: id,
  } as Session;
}

describe('groupSessionsByDate', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-03-06T12:00:00Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('uses localized labels for today and yesterday', () => {
    const groups = groupSessionsByDate(
      [
        createSession('today', '2026-03-06T08:00:00Z'),
        createSession('yesterday', '2026-03-05T08:00:00Z'),
      ],
      {
        locale: 'zh-CN',
        todayLabel: '今天',
        yesterdayLabel: '昨天',
      }
    );

    expect(groups[0]?.label).toBe('今天');
    expect(groups[1]?.label).toBe('昨天');
  });
});
