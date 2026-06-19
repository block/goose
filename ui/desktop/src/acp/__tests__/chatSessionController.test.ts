import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Session } from '../../api';
import { acpChatSessionController } from '../chatSessionController';
import { acpChatSessionActions, acpChatSessionStore } from '../chatSessionStore';
import { acpLoadSession, isAcpSessionLoadInFlight, sessionInfoToSession } from '../sessions';

vi.mock('../../utils/extensionErrorUtils', () => ({
  showExtensionLoadResults: vi.fn(),
}));

vi.mock('../chatSessionStore', () => ({
  acpChatSessionStore: {
    getSnapshot: vi.fn(),
  },
  acpChatSessionActions: {
    startSessionLoad: vi.fn(),
    finishSessionLoad: vi.fn(),
    failSessionLoad: vi.fn(),
    startPromptAttempt: vi.fn(),
    finishPromptAttemptIfCurrent: vi.fn(),
    isCurrentPromptAttempt: vi.fn(),
    setMessages: vi.fn(),
    clearActivePromptAttempt: vi.fn(),
    setChatState: vi.fn(),
    setSessionMetadata: vi.fn(),
    setSessionLoadError: vi.fn(),
  },
}));

vi.mock('../sessions', () => ({
  acpLoadSession: vi.fn(),
  isAcpSessionLoadInFlight: vi.fn(),
  sessionInfoToSession: vi.fn(),
  acpForkSession: vi.fn(),
  acpTruncateSessionConversation: vi.fn(),
}));

const SESSION_ID = 'session-1';

function loadedSession(): Session {
  return {
    id: SESSION_ID,
    name: 'Loaded session',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    working_dir: '/tmp',
    message_count: 0,
    extension_data: {},
    source: 'test',
  } as Session;
}

function mockLoadResult() {
  return {
    sessionInfo: {
      sessionId: SESSION_ID,
      cwd: '/tmp',
      title: 'Loaded session',
      updatedAt: '2026-01-01T00:00:00Z',
    },
    response: {},
    meta: {},
  } as Awaited<ReturnType<typeof acpLoadSession>>;
}

describe('acpChatSessionController.loadSession', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(acpChatSessionStore.getSnapshot).mockReturnValue(undefined);
    vi.mocked(acpLoadSession).mockResolvedValue(mockLoadResult());
    vi.mocked(sessionInfoToSession).mockReturnValue(loadedSession());
  });

  it('starts a fresh session load before ACP replays notifications', async () => {
    vi.mocked(isAcpSessionLoadInFlight).mockReturnValue(false);

    await acpChatSessionController.loadSession(SESSION_ID);

    expect(acpChatSessionActions.startSessionLoad).toHaveBeenCalledWith(SESSION_ID);
    expect(acpLoadSession).toHaveBeenCalledWith(SESSION_ID);
    expect(acpChatSessionActions.finishSessionLoad).toHaveBeenCalledWith(
      SESSION_ID,
      loadedSession()
    );
  });

  it('does not reset replay state when joining an in-flight session load', async () => {
    vi.mocked(isAcpSessionLoadInFlight).mockReturnValue(true);

    await acpChatSessionController.loadSession(SESSION_ID);

    expect(acpChatSessionActions.startSessionLoad).not.toHaveBeenCalled();
    expect(acpLoadSession).toHaveBeenCalledWith(SESSION_ID);
    expect(acpChatSessionActions.finishSessionLoad).toHaveBeenCalledWith(
      SESSION_ID,
      loadedSession()
    );
  });
});
