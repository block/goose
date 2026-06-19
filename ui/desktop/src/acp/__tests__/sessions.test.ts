import type { SessionInfo } from '@agentclientprotocol/sdk';
import { describe, expect, it } from 'vitest';
import { sessionInfoToSession } from '../sessions';

function sessionInfo(overrides: Partial<SessionInfo> = {}): SessionInfo {
  return {
    sessionId: 'session-1',
    cwd: '/tmp',
    title: 'Scheduled session',
    updatedAt: '2026-01-01T00:00:00Z',
    _meta: {
      createdAt: '2026-01-01T00:00:00Z',
      messageCount: 0,
      sessionType: 'scheduled',
    },
    ...overrides,
  } as unknown as SessionInfo;
}

describe('ACP sessions', () => {
  it('preserves session type from ACP session info metadata', () => {
    const session = sessionInfoToSession(sessionInfo());

    expect(session.session_type).toBe('scheduled');
  });
});
