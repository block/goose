import { describe, it, expect } from 'vitest';
import { shouldShowNewChatTitle } from '../sessions';
import {
  getSessionDisplayName,
  sortAndTrim,
  mergeWithEmptyLocals,
  prependUnique,
} from '../hooks/useNavigationSessions';
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

describe('session reuse scoping (fix for #7601)', () => {
  // Simulates the core logic extracted from handleNewChat in useNavigationSessions.ts.
  // Before the fix: `sessions.find(s => shouldShowNewChatTitle(s))` picked the
  // first global empty session regardless of which window called it.
  // After the fix: only the current window's activeSessionId is considered.
  function findReusableSession(
    sessions: Session[],
    activeSessionId: string | undefined
  ): Session | undefined {
    const currentActive = activeSessionId
      ? sessions.find((s) => s.id === activeSessionId)
      : undefined;
    if (currentActive && shouldShowNewChatTitle(currentActive)) {
      return currentActive;
    }
    return undefined;
  }

  const emptySessionA = makeSession({ id: 'empty-a', message_count: 0, user_set_name: false });
  const emptySessionB = makeSession({ id: 'empty-b', message_count: 0, user_set_name: false });
  const usedSession = makeSession({ id: 'used-c', message_count: 5, user_set_name: true });

  const allSessions = [emptySessionA, emptySessionB, usedSession];

  it("window A only reuses its own active empty session, not window B's", () => {
    // Window A has emptySessionA active, Window B has emptySessionB active.
    // Under the old logic, both would grab emptySessionA (the first in the list).
    const windowAResult = findReusableSession(allSessions, 'empty-a');
    const windowBResult = findReusableSession(allSessions, 'empty-b');

    expect(windowAResult?.id).toBe('empty-a');
    expect(windowBResult?.id).toBe('empty-b');
    // They never collide on the same session.
    expect(windowAResult?.id).not.toBe(windowBResult?.id);
  });

  it('does not reuse a session that has messages even if it is active', () => {
    const result = findReusableSession(allSessions, 'used-c');
    expect(result).toBeUndefined();
  });

  it('returns undefined when there is no active session id', () => {
    const result = findReusableSession(allSessions, undefined);
    expect(result).toBeUndefined();
  });

  it('returns undefined when the active session id is not in the list', () => {
    const result = findReusableSession(allSessions, 'nonexistent');
    expect(result).toBeUndefined();
  });

  it('demonstrates the old bug: global find would give same session to both windows', () => {
    // Old logic (before fix) - both windows get the same session.
    const oldLogicFind = (sessions: Session[]) => sessions.find((s) => shouldShowNewChatTitle(s));

    const windowAOld = oldLogicFind(allSessions);
    const windowBOld = oldLogicFind(allSessions);

    // Both windows would grab the exact same session - the bug.
    expect(windowAOld?.id).toBe(windowBOld?.id);
    expect(windowAOld?.id).toBe('empty-a');
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
  it('sorts by created_at descending', () => {
    const result = sortAndTrim([
      makeSession({ id: 'old', created_at: '2024-01-01T00:00:00Z' }),
      makeSession({ id: 'new', created_at: '2024-03-01T00:00:00Z' }),
      makeSession({ id: 'mid', created_at: '2024-02-01T00:00:00Z' }),
    ]);
    expect(result.map((s) => s.id)).toEqual(['new', 'mid', 'old']);
  });

  it('caps the list at 25 sessions', () => {
    const sessions = Array.from({ length: 40 }, (_, i) =>
      makeSession({ id: `s-${i}`, created_at: new Date(2024, 0, i + 1).toISOString() })
    );
    expect(sortAndTrim(sessions)).toHaveLength(25);
  });

  it('does not mutate the input array', () => {
    const input = [
      makeSession({ id: 'a', created_at: '2024-01-01T00:00:00Z' }),
      makeSession({ id: 'b', created_at: '2024-02-01T00:00:00Z' }),
    ];
    sortAndTrim(input);
    expect(input.map((s) => s.id)).toEqual(['a', 'b']);
  });
});

describe('mergeWithEmptyLocals', () => {
  it('keeps locally-tracked empty sessions the api has not returned yet', () => {
    const emptyLocal = makeSession({ id: 'local-empty', message_count: 0 });
    const apiSessions = [makeSession({ id: 'api-1', message_count: 3 })];
    const result = mergeWithEmptyLocals([emptyLocal], apiSessions);
    expect(result.map((s) => s.id)).toEqual(['local-empty', 'api-1']);
  });

  it('drops an empty local once the api returns it', () => {
    const local = makeSession({ id: 'shared', message_count: 0 });
    const apiSessions = [makeSession({ id: 'shared', message_count: 1 })];
    const result = mergeWithEmptyLocals([local], apiSessions);
    expect(result).toHaveLength(1);
    expect(result[0].message_count).toBe(1);
  });

  it('does not keep non-empty locals missing from the api', () => {
    const usedLocal = makeSession({ id: 'used-local', message_count: 5 });
    const apiSessions = [makeSession({ id: 'api-1', message_count: 3 })];
    const result = mergeWithEmptyLocals([usedLocal], apiSessions);
    expect(result.map((s) => s.id)).toEqual(['api-1']);
  });

  it('caps the merged list at 25 sessions', () => {
    const emptyLocals = Array.from({ length: 5 }, (_, i) =>
      makeSession({ id: `local-${i}`, message_count: 0 })
    );
    const apiSessions = Array.from({ length: 25 }, (_, i) =>
      makeSession({ id: `api-${i}`, message_count: 1 })
    );
    expect(mergeWithEmptyLocals(emptyLocals, apiSessions)).toHaveLength(25);
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
