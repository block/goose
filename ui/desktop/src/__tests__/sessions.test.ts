import { describe, it, expect } from 'vitest';
import { getSessionDisplayName } from '../sessions';
import { prependUnique } from '../hooks/useNavigationSessions';
import type { Session } from '../api';
import type { SessionListItem } from '../acp/sessions';

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

function makeListItem(overrides: Partial<SessionListItem> = {}): SessionListItem {
  return {
    id: 'sess-1',
    name: 'untitled',
    workingDir: '/tmp',
    updatedAt: new Date().toISOString(),
    messageCount: 0,
    createdAt: new Date().toISOString(),
    ...overrides,
  };
}

describe('getSessionDisplayName', () => {
  it('returns the session name', () => {
    const session = makeSession({
      name: 'My Chat',
    });
    expect(getSessionDisplayName(session)).toBe('My Chat');
  });

  it('falls back to the default title when the session name is empty', () => {
    const session = makeSession({
      name: '',
    });
    expect(getSessionDisplayName(session)).toBe('New Chat');
  });
});

describe('prependUnique', () => {
  it('prepends a new session to the front', () => {
    const prev = [makeListItem({ id: 'a' })];
    const result = prependUnique(prev, makeListItem({ id: 'b' }));
    expect(result.map((s) => s.id)).toEqual(['b', 'a']);
  });

  it('returns the same reference when the session is already present', () => {
    const prev = [makeListItem({ id: 'a' }), makeListItem({ id: 'b' })];
    const result = prependUnique(prev, makeListItem({ id: 'a' }));
    expect(result).toBe(prev);
  });

  it('caps the list at 25 sessions', () => {
    const prev = Array.from({ length: 25 }, (_, i) => makeListItem({ id: `s-${i}` }));
    const result = prependUnique(prev, makeListItem({ id: 'new' }));
    expect(result).toHaveLength(25);
    expect(result[0].id).toBe('new');
  });
});
