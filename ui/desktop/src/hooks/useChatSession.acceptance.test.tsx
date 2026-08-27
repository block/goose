import type { ReactNode } from 'react';
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AcpChatSessionSnapshot } from '../acp/chatSessionStore';
import { IntlTestWrapper } from '../i18n/test-utils';
import { ChatState } from '../types/chatState';
import { useChatSession } from './useChatSession';

const mocks = vi.hoisted(() => ({
  snapshot: undefined as AcpChatSessionSnapshot | undefined,
  loadSession: vi.fn(),
  submitMessage: vi.fn(),
  setMessages: vi.fn(),
}));

vi.mock('../acp/chatSessionController', () => ({
  acpChatSessionController: {
    loadSession: mocks.loadSession,
    submitMessage: mocks.submitMessage,
    stop: vi.fn(),
    updateMessage: vi.fn(),
  },
}));

vi.mock('../acp/chatSessionStore', () => ({
  acpChatSessionActions: {
    setMessages: mocks.setMessages,
    setSessionMetadata: vi.fn(),
  },
  acpChatSessionStore: {
    getSnapshot: () => mocks.snapshot,
  },
  useAcpChatSessionSnapshot: () => mocks.snapshot,
}));

vi.mock('../acp/acpConnection', () => ({
  isAcpRecovering: () => false,
}));

vi.mock('../acp/elicitationRequests', () => ({
  resolveAcpElicitationRequest: vi.fn(),
}));

vi.mock('../acp/prompt', () => ({
  acpSteerSession: vi.fn(),
}));

vi.mock('../toasts', () => ({
  toastError: vi.fn(),
}));

function Wrapper({ children }: { children: ReactNode }) {
  return <IntlTestWrapper>{children}</IntlTestWrapper>;
}

describe('useChatSession submission acceptance', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.snapshot = {
      session: {
        id: 'session-1',
        name: 'Test session',
        created_at: '2026-01-01T00:00:00Z',
        updated_at: '2026-01-01T00:00:00Z',
        working_dir: '/tmp',
        extension_data: {},
        message_count: 0,
      },
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
      progressMessage: undefined,
      chatState: ChatState.Idle,
      sessionLoadError: undefined,
      activePromptAttemptId: null,
      activeRunId: null,
      pendingCancelPromptAttemptId: null,
    } as AcpChatSessionSnapshot;
    mocks.loadSession.mockResolvedValue(undefined);
  });

  it('acknowledges an accepted message before the agent run completes', async () => {
    mocks.submitMessage.mockReturnValue(new Promise<void>(() => undefined));
    const { result } = renderHook(
      () =>
        useChatSession({
          sessionId: 'session-1',
          onStreamFinish: vi.fn(),
          onSessionLoaded: vi.fn(),
        }),
      { wrapper: Wrapper }
    );

    let accepted = false;
    await act(async () => {
      accepted = await result.current.handleSubmit({ msg: 'from MCP App', images: [] });
    });

    expect(accepted).toBe(true);
    expect(mocks.submitMessage).toHaveBeenCalledTimes(1);
  });
});
