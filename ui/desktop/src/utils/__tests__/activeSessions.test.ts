import { describe, expect, it } from 'vitest';
import {
  addActiveSession,
  clearDeletedSessionFromCreatedDetail,
  clearInitialMessage,
  getCreatedSessionId,
  markSessionDeleted,
  removeActiveSession,
} from '../activeSessions';

describe('activeSessions', () => {
  it('blocks add-active-session for deleted ids and allows id reuse after session-created clear', () => {
    const deletedSessionIds = new Set<string>();
    const base = [{ sessionId: 'existing' }];

    markSessionDeleted(deletedSessionIds, 'reused');
    const blocked = addActiveSession(base, { sessionId: 'reused' }, deletedSessionIds, 10);
    expect(blocked).toEqual(base);

    const clearedFromSessionObject = clearDeletedSessionFromCreatedDetail(deletedSessionIds, {
      session: { id: 'reused' },
    });
    expect(clearedFromSessionObject).toBe('reused');
    const allowed = addActiveSession(base, { sessionId: 'reused' }, deletedSessionIds, 10);
    expect(allowed).toEqual([{ sessionId: 'existing' }, { sessionId: 'reused' }]);
  });

  it('clears tombstone from flat sessionId payload shape and unblocks add', () => {
    const deletedSessionIds = new Set<string>();
    markSessionDeleted(deletedSessionIds, 'flat-shape-id');

    const clearedFromFlatField = clearDeletedSessionFromCreatedDetail(deletedSessionIds, {
      sessionId: 'flat-shape-id',
    });
    expect(clearedFromFlatField).toBe('flat-shape-id');

    const allowed = addActiveSession([], { sessionId: 'flat-shape-id' }, deletedSessionIds, 10);
    expect(allowed).toEqual([{ sessionId: 'flat-shape-id', initialMessage: undefined }]);
  });

  it('moves existing sessions to the end and preserves initial message rules', () => {
    const message = { msg: 'hello', images: [] };
    const deletedSessionIds = new Set<string>();
    const base = [
      { sessionId: 'a' },
      { sessionId: 'b', initialMessage: message },
      { sessionId: 'c' },
    ];

    const moved = addActiveSession(base, { sessionId: 'b' }, deletedSessionIds, 10);
    expect(moved).toEqual([
      { sessionId: 'a' },
      { sessionId: 'c' },
      { sessionId: 'b', initialMessage: message },
    ]);

    const enriched = addActiveSession(
      [{ sessionId: 'x' }],
      { sessionId: 'x', initialMessage: message },
      deletedSessionIds,
      10
    );
    expect(enriched).toEqual([{ sessionId: 'x', initialMessage: message }]);
  });

  it('enforces max active sessions by evicting least-recently used entries', () => {
    const deletedSessionIds = new Set<string>();
    const base = [{ sessionId: 'a' }, { sessionId: 'b' }];

    const updated = addActiveSession(base, { sessionId: 'c' }, deletedSessionIds, 2);
    expect(updated).toEqual([{ sessionId: 'b' }, { sessionId: 'c' }]);
  });

  it('clears initial message for one session only', () => {
    const message = { msg: 'hello', images: [] };
    const base = [
      { sessionId: 'a', initialMessage: message },
      { sessionId: 'b', initialMessage: message },
    ];

    const updated = clearInitialMessage(base, 'a');
    expect(updated).toEqual([
      { sessionId: 'a', initialMessage: undefined },
      { sessionId: 'b', initialMessage: message },
    ]);
  });

  it('removes the deleted session from the active list', () => {
    const base = [{ sessionId: 'a' }, { sessionId: 'b' }];
    expect(removeActiveSession(base, 'a')).toEqual([{ sessionId: 'b' }]);
  });

  it('extracts created session id from both payload shapes', () => {
    expect(getCreatedSessionId({ session: { id: 'session-from-object' } })).toBe(
      'session-from-object'
    );
    expect(getCreatedSessionId({ sessionId: 'session-from-flat-field' })).toBe(
      'session-from-flat-field'
    );
    expect(getCreatedSessionId({})).toBeUndefined();
  });
});
