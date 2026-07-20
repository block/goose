import { describe, it, expect } from 'vitest';
import { getSessionDisplayName, shouldShowNewChatTitle } from '../sessions';
import { prependUnique } from '../hooks/useNavigationSessions';
import type { SessionListItem } from '../acp/sessions';
import type { Session } from '../types/session';
import { DEFAULT_CHAT_TITLE } from '../contexts/ChatContext';

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

describe('shouldShowNewChatTitle', () => {
  it('returns true for an empty session without a user-set name', () => {
    const session = makeSession({
      name: DEFAULT_CHAT_TITLE,
      message_count: 0,
      user_set_name: false,
    });
    expect(shouldShowNewChatTitle(session)).toBe(true);
  });

  it('returns false when an empty session already has a generated title', () => {
    const session = makeSession({
      name: 'Generated title',
      message_count: 0,
      user_set_name: false,
    });
    expect(shouldShowNewChatTitle(session)).toBe(false);
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
  it('returns a generated title even when message count metadata is stale', () => {
    const session = makeSession({
      name: 'Generated title',
      user_set_name: false,
      message_count: 0,
    });
    expect(getSessionDisplayName(session)).toBe('Generated title');
  });

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
