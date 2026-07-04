import type { SessionInfo } from '@agentclientprotocol/sdk';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getAcpClient } from '../acpConnection';
import {
  acpArchiveSession,
  acpGetSessionListItem,
  acpListRecentSessions,
  acpListSessions,
  acpLoadSession,
  acpUnarchiveSession,
  sessionInfoToSession,
} from '../sessions';

vi.mock('../acpConnection', () => ({
  getAcpClient: vi.fn(),
}));

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
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('preserves session type from ACP session info metadata', () => {
    const session = sessionInfoToSession(sessionInfo());

    expect(session.session_type).toBe('scheduled');
  });

  it('returns session info refreshed after loading the ACP session', async () => {
    const loadedSessionInfo = sessionInfo({
      _meta: {
        createdAt: '2026-01-01T00:00:00Z',
        messageCount: 0,
        providerId: 'anthropic',
        modelId: 'claude-sonnet-4-5',
      },
    });
    const client = {
      gosling: {
        sessionInfo_unstable: vi
          .fn()
          .mockResolvedValueOnce({ session: sessionInfo() })
          .mockResolvedValueOnce({ session: loadedSessionInfo }),
      },
      loadSession: vi.fn().mockResolvedValue({}),
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    const result = await acpLoadSession('session-1');

    expect(client.loadSession).toHaveBeenCalledWith({
      sessionId: 'session-1',
      cwd: '/tmp',
      mcpServers: [],
    });
    expect(client.gosling.sessionInfo_unstable).toHaveBeenCalledTimes(2);
    expect(result.sessionInfo).toBe(loadedSessionInfo);
    expect(sessionInfoToSession(result.sessionInfo).provider_name).toBe('anthropic');
    expect(sessionInfoToSession(result.sessionInfo).model_config?.model_name).toBe(
      'claude-sonnet-4-5'
    );
  });

  it('returns a list item from ACP session info', async () => {
    const client = {
      gosling: {
        sessionInfo_unstable: vi.fn().mockResolvedValue({
          session: sessionInfo({
            title: 'Subagent session',
            _meta: {
              createdAt: '2026-01-01T00:00:00Z',
              lastMessageAt: '2026-01-01T00:01:00Z',
              messageCount: 3,
              sessionType: 'sub_agent',
              providerId: 'anthropic',
              modelId: 'claude-sonnet-4-5',
            },
          }),
        }),
      },
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    const item = await acpGetSessionListItem('session-1');

    expect(client.gosling.sessionInfo_unstable).toHaveBeenCalledWith({ sessionId: 'session-1' });
    expect(item).toMatchObject({
      id: 'session-1',
      name: 'Subagent session',
      workingDir: '/tmp',
      messageCount: 3,
      lastMessageAt: '2026-01-01T00:01:00Z',
      providerId: 'anthropic',
      modelId: 'claude-sonnet-4-5',
    });
  });

  it('sends archive-state metadata when listing sessions', async () => {
    const client = {
      listSessions: vi.fn().mockResolvedValue({ sessions: [], nextCursor: 'next-cursor' }),
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    const result = await acpListSessions('cursor-1', {
      keyword: ' archived  ',
      archiveState: 'archived',
      includeLastMessageSnippet: true,
    });

    expect(client.listSessions).toHaveBeenCalledWith({
      cursor: 'cursor-1',
      _meta: {
        types: ['user', 'scheduled'],
        query: 'archived',
        gosling: {
          archiveState: 'archived',
          includeLastMessageSnippet: true,
        },
      },
    });
    expect(result).toEqual({ sessions: [], nextCursor: 'next-cursor' });
  });

  it('defaults recent-session listing to active sessions', async () => {
    const client = {
      listSessions: vi.fn().mockResolvedValue({ sessions: [], nextCursor: null }),
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    await acpListRecentSessions(25);

    expect(client.listSessions).toHaveBeenCalledWith({
      _meta: {
        types: ['user', 'scheduled'],
        gosling: {
          archiveState: 'active',
          includeLastMessageSnippet: false,
        },
      },
    });
  });

  it('calls ACP archive and unarchive session wrappers', async () => {
    const client = {
      gosling: {
        sessionArchive_unstable: vi.fn().mockResolvedValue(undefined),
        sessionUnarchive_unstable: vi.fn().mockResolvedValue(undefined),
      },
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    await acpArchiveSession('session-1');
    await acpUnarchiveSession('session-1');

    expect(client.gosling.sessionArchive_unstable).toHaveBeenCalledWith({
      sessionId: 'session-1',
    });
    expect(client.gosling.sessionUnarchive_unstable).toHaveBeenCalledWith({
      sessionId: 'session-1',
    });
  });
});
