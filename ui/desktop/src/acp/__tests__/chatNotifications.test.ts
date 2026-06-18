import type { SessionNotification } from '@agentclientprotocol/sdk';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Session } from '../../api';
import { AppEvents } from '../../constants/events';
import { ChatState } from '../../types/chatState';
import { handleAcpSessionNotification } from '../chatNotifications';
import type { AcpChatSessionSnapshot } from '../chatSessionStore';
import { acpChatSessionStore } from '../chatSessionStore';

vi.mock('../../acpChatFeatureFlag', () => ({
  USE_ACP_CHAT: true,
}));

vi.mock('../chatSessionStore', () => ({
  acpChatSessionStore: {
    getSnapshot: vi.fn(),
    applyAcpSessionNotification: vi.fn(),
  },
}));

const SESSION_ID = 'session-1';

function sessionInfoUpdate(title: string): SessionNotification {
  return {
    sessionId: SESSION_ID,
    update: {
      sessionUpdate: 'session_info_update',
      title,
    },
  };
}

function sessionWithName(name: string): Session {
  return {
    id: SESSION_ID,
    name,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    working_dir: '/tmp',
    message_count: 0,
    extension_data: {},
    source: 'test',
  } as Session;
}

function snapshotWithName(name: string): AcpChatSessionSnapshot {
  return {
    session: sessionWithName(name),
    messages: [],
    tokenState: {
      inputTokens: 0,
      outputTokens: 0,
      totalTokens: 0,
      accumulatedInputTokens: 0,
      accumulatedOutputTokens: 0,
      accumulatedTotalTokens: 0,
    },
    notifications: [],
    chatState: ChatState.Idle,
    sessionLoadError: undefined,
    activePromptAttemptId: null,
  };
}

function snapshotWithoutSession(): AcpChatSessionSnapshot {
  return {
    session: undefined,
    messages: [],
    tokenState: {
      inputTokens: 0,
      outputTokens: 0,
      totalTokens: 0,
      accumulatedInputTokens: 0,
      accumulatedOutputTokens: 0,
      accumulatedTotalTokens: 0,
    },
    notifications: [],
    chatState: ChatState.Idle,
    sessionLoadError: undefined,
    activePromptAttemptId: null,
  };
}

describe('handleAcpSessionNotification', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('dispatches SESSION_RENAMED when a session info notification changes the name', async () => {
    const dispatchEvent = vi.spyOn(window, 'dispatchEvent');
    vi.mocked(acpChatSessionStore.getSnapshot).mockReturnValueOnce(snapshotWithName('Old name'));
    vi.mocked(acpChatSessionStore.applyAcpSessionNotification).mockReturnValueOnce(
      snapshotWithName('New name')
    );

    await handleAcpSessionNotification(sessionInfoUpdate('New name'));

    expect(dispatchEvent).toHaveBeenCalledWith(
      expect.objectContaining({
        type: AppEvents.SESSION_RENAMED,
        detail: { sessionId: SESSION_ID, newName: 'New name' },
      })
    );
  });

  it('does not dispatch SESSION_RENAMED when the name is unchanged', async () => {
    const dispatchEvent = vi.spyOn(window, 'dispatchEvent');
    vi.mocked(acpChatSessionStore.getSnapshot).mockReturnValueOnce(snapshotWithName('Same name'));
    vi.mocked(acpChatSessionStore.applyAcpSessionNotification).mockReturnValueOnce(
      snapshotWithName('Same name')
    );

    await handleAcpSessionNotification(sessionInfoUpdate('Same name'));

    expect(dispatchEvent).not.toHaveBeenCalled();
  });

  it('dispatches SESSION_RENAMED from the notification title when the session is not loaded', async () => {
    const dispatchEvent = vi.spyOn(window, 'dispatchEvent');
    vi.mocked(acpChatSessionStore.getSnapshot).mockReturnValueOnce(snapshotWithoutSession());
    vi.mocked(acpChatSessionStore.applyAcpSessionNotification).mockReturnValueOnce(
      snapshotWithoutSession()
    );

    await handleAcpSessionNotification(sessionInfoUpdate('Generated name'));

    expect(dispatchEvent).toHaveBeenCalledWith(
      expect.objectContaining({
        type: AppEvents.SESSION_RENAMED,
        detail: { sessionId: SESSION_ID, newName: 'Generated name' },
      })
    );
  });
});
