import { describe, expect, it } from 'vitest';
import { PHANTOM_SESSION_MIN_AGE_MS, selectPhantomSessionsForPurge } from './phantomSessions';

describe('selectPhantomSessionsForPurge', () => {
  const now = Date.parse('2026-08-08T23:00:00.000Z');

  it('purges old empty untitled sessions', () => {
    const sessions = [
      {
        id: 'old',
        messageCount: 0,
        createdAt: new Date(now - PHANTOM_SESSION_MIN_AGE_MS - 1).toISOString(),
      },
    ];

    expect(selectPhantomSessionsForPurge(sessions, new Set(), now).map((s) => s.id)).toEqual([
      'old',
    ]);
  });

  it('keeps brand-new Hub sessions younger than the age floor', () => {
    const sessions = [
      {
        id: 'fresh',
        messageCount: 0,
        createdAt: new Date(now - 5_000).toISOString(),
      },
    ];

    expect(selectPhantomSessionsForPurge(sessions, new Set(), now)).toEqual([]);
  });

  it('keeps protected active session ids even when old', () => {
    const sessions = [
      {
        id: 'active',
        messageCount: 0,
        createdAt: new Date(now - PHANTOM_SESSION_MIN_AGE_MS * 2).toISOString(),
      },
    ];

    expect(selectPhantomSessionsForPurge(sessions, new Set(['active']), now)).toEqual([]);
  });

  it('skips sessions with messages, names, recipes, or missing timestamps', () => {
    const sessions = [
      {
        id: 'has-messages',
        messageCount: 1,
        createdAt: new Date(now - PHANTOM_SESSION_MIN_AGE_MS * 2).toISOString(),
      },
      {
        id: 'named',
        messageCount: 0,
        userSetName: true,
        createdAt: new Date(now - PHANTOM_SESSION_MIN_AGE_MS * 2).toISOString(),
      },
      {
        id: 'recipe',
        messageCount: 0,
        hasRecipe: true,
        createdAt: new Date(now - PHANTOM_SESSION_MIN_AGE_MS * 2).toISOString(),
      },
      { id: 'no-date', messageCount: 0 },
    ];

    expect(selectPhantomSessionsForPurge(sessions, new Set(), now)).toEqual([]);
  });
});
