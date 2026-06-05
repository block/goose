import { describe, it, expect } from 'vitest';
import { shouldShowNewChatTitle } from '../sessions';
import { getSessionDisplayName, sortAndTrim, prependUnique } from '../hooks/useNavigationSessions';
import type { Session } from '../api';

// Helper to build a minimal Session object for testing.
function makeSession(overrides: Partial<Session> = {}): Session {
  return {
    id: 'sess-1',
    name: 'untitled',
    message_count: 0,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    working_dir: '/tmp',
    extension_data: { active: [], installed: [] },
    ...overrides,
  };
}

describe('shouldShowNewChatTitle', () => {
  it('returns true for an empty session without a user-set name', () => {
    const session = makeSession({ message_count: 0, user_set_name: false });
    expect(shouldShowNewChatTitle(session)).toBe(true);
  });

  it('returns false when the session has messages', () => {
    const session = makeSession({ message_count: 3, user_set_name: false });
    expect(shouldShowNewChatTitle(session)).toBe(false);
  });

  it('returns false when the user has set a custom name', () => {
    const session = makeSession({ message_count: 0, user_set_name: true });
    expect(shouldShowNewChatTitle(session)).toBe(false);
  });

  it('returns false when the session has a recipe', () => {
    const session = makeSession({
      message_count: 0,
      user_set_name: false,
      recipe: { title: 'Recipe', steps: [] } as unknown as Session['recipe'],
    });
    expect(shouldShowNewChatTitle(session)).toBe(false);
  });
});

describe('getSessionDisplayName (fix for #8865)', () => {
  it('returns the user-set name for a recipe session that has been renamed', () => {
    const session = makeSession({
      name: 'My Renamed Chat',
      user_set_name: true,
      message_count: 2,
      recipe: { title: 'Some Recipe' } as unknown as Session['recipe'],
    });
    expect(getSessionDisplayName(session)).toBe('My Renamed Chat');
  });

  it('falls back to the recipe title when the user has not renamed', () => {
    const session = makeSession({
      name: 'auto-generated',
      user_set_name: false,
      message_count: 2,
      recipe: { title: 'Some Recipe' } as unknown as Session['recipe'],
    });
    expect(getSessionDisplayName(session)).toBe('Some Recipe');
  });
});

describe('sortAndTrim', () => {
  it('sorts by updated_at descending', () => {
    const result = sortAndTrim([
      makeSession({
        id: 'old-but-active',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-03-01T00:00:00Z',
      }),
      makeSession({
        id: 'newer-but-idle',
        created_at: '2024-03-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      }),
      makeSession({
        id: 'mid',
        created_at: '2024-02-01T00:00:00Z',
        updated_at: '2024-02-01T00:00:00Z',
      }),
    ]);
    expect(result.map((s) => s.id)).toEqual(['old-but-active', 'mid', 'newer-but-idle']);
  });

  it('caps the list at 25 sessions', () => {
    const sessions = Array.from({ length: 40 }, (_, i) =>
      makeSession({ id: `s-${i}`, created_at: new Date(2024, 0, i + 1).toISOString() })
    );
    expect(sortAndTrim(sessions)).toHaveLength(25);
  });

  it('does not mutate the input array', () => {
    const input = [
      makeSession({ id: 'a', updated_at: '2024-01-01T00:00:00Z' }),
      makeSession({ id: 'b', updated_at: '2024-02-01T00:00:00Z' }),
    ];
    sortAndTrim(input);
    expect(input.map((s) => s.id)).toEqual(['a', 'b']);
  });
});

describe('prependUnique', () => {
  it('prepends a new session to the front', () => {
    const prev = [makeSession({ id: 'a' })];
    const result = prependUnique(prev, makeSession({ id: 'b' }));
    expect(result.map((s) => s.id)).toEqual(['b', 'a']);
  });

  it('returns the same reference when the session is already present', () => {
    const prev = [makeSession({ id: 'a' }), makeSession({ id: 'b' })];
    const result = prependUnique(prev, makeSession({ id: 'a' }));
    expect(result).toBe(prev);
  });

  it('caps the list at 25 sessions', () => {
    const prev = Array.from({ length: 25 }, (_, i) => makeSession({ id: `s-${i}` }));
    const result = prependUnique(prev, makeSession({ id: 'new' }));
    expect(result).toHaveLength(25);
    expect(result[0].id).toBe('new');
  });
});
