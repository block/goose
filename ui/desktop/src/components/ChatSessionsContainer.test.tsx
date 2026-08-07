import { render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ChatSessionsContainer from './ChatSessionsContainer';
import { subscribeToAcpRecovery } from '../acp/acpConnection';
import { acpChatSessionController } from '../acp/chatSessionController';
import { acpChatSessionActions } from '../acp/chatSessionStore';
import { cancelAcpElicitationRequestsForSession } from '../acp/elicitationRequests';
import { cancelAcpPermissionRequestsForSession } from '../acp/permissionRequests';

vi.mock('react-router', () => ({
  useSearchParams: () => [new URLSearchParams('resumeSessionId=session-1')],
}));

vi.mock('./BaseChat', () => ({
  default: ({ sessionId }: { sessionId: string }) => <div>{sessionId}</div>,
}));

vi.mock('../acp/acpConnection', () => ({
  subscribeToAcpRecovery: vi.fn(),
}));

vi.mock('../acp/chatSessionController', () => ({
  acpChatSessionController: {
    restoreSession: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock('../acp/chatSessionStore', () => ({
  acpChatSessionActions: {
    clearActivePromptAttempt: vi.fn(),
  },
}));

vi.mock('../acp/elicitationRequests', () => ({
  cancelAcpElicitationRequestsForSession: vi.fn(),
}));

vi.mock('../acp/permissionRequests', () => ({
  cancelAcpPermissionRequestsForSession: vi.fn(),
}));

describe('ChatSessionsContainer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('clears in-flight prompt state while ACP is reconnecting', () => {
    let onRecoveryChanged: ((recovering: boolean) => void) | undefined;
    vi.mocked(subscribeToAcpRecovery).mockImplementation((listener) => {
      onRecoveryChanged = listener;
      return () => undefined;
    });

    render(
      <ChatSessionsContainer
        setChat={vi.fn()}
        activeSessions={[{ sessionId: 'session-1' }, { sessionId: 'session-2' }]}
      />
    );

    onRecoveryChanged?.(true);

    expect(cancelAcpPermissionRequestsForSession).toHaveBeenCalledTimes(2);
    expect(cancelAcpPermissionRequestsForSession).toHaveBeenCalledWith('session-1');
    expect(cancelAcpPermissionRequestsForSession).toHaveBeenCalledWith('session-2');
    expect(cancelAcpElicitationRequestsForSession).toHaveBeenCalledTimes(2);
    expect(acpChatSessionActions.clearActivePromptAttempt).toHaveBeenCalledTimes(2);
    expect(acpChatSessionActions.clearActivePromptAttempt).toHaveBeenCalledWith('session-1');
    expect(acpChatSessionActions.clearActivePromptAttempt).toHaveBeenCalledWith('session-2');
    expect(acpChatSessionController.restoreSession).not.toHaveBeenCalled();
  });

  it('restores active chat sessions after ACP reconnects', () => {
    let onRecoveryChanged: ((recovering: boolean) => void) | undefined;
    vi.mocked(subscribeToAcpRecovery).mockImplementation((listener) => {
      onRecoveryChanged = listener;
      return () => undefined;
    });

    render(
      <ChatSessionsContainer
        setChat={vi.fn()}
        activeSessions={[{ sessionId: 'session-1' }, { sessionId: 'session-2' }]}
      />
    );

    onRecoveryChanged?.(false);

    expect(acpChatSessionController.restoreSession).toHaveBeenCalledTimes(2);
    expect(acpChatSessionController.restoreSession).toHaveBeenCalledWith('session-1');
    expect(acpChatSessionController.restoreSession).toHaveBeenCalledWith('session-2');
  });
});
