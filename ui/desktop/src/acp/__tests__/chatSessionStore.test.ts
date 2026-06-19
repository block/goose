import type {
  CreateElicitationRequest,
  RequestPermissionRequest,
  SessionNotification,
} from '@agentclientprotocol/sdk';
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { Message, Session } from '../../api';
import { ChatState } from '../../types/chatState';
import {
  acpChatSessionStore,
  createAcpChatSessionStore,
  type AcpChatSessionStore,
  useAcpChatSessionSnapshot,
} from '../chatSessionStore';

function message(id: string, text: string): Message {
  return {
    id,
    role: 'user',
    created: 123,
    content: [{ type: 'text', text }],
    metadata: { userVisible: true, agentVisible: true },
  };
}

function session(id: string, conversation: Message[] = []): Session {
  return {
    id,
    name: `Session ${id}`,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    working_dir: '/tmp',
    message_count: conversation.length,
    extension_data: {},
    source: 'test',
    conversation,
    input_tokens: 1,
    output_tokens: 2,
    total_tokens: 3,
    accumulated_input_tokens: 4,
    accumulated_output_tokens: 5,
    accumulated_total_tokens: 9,
  } as Session;
}

function permissionRequest(sessionId: string, toolCallId = 'tool-1'): RequestPermissionRequest {
  return {
    sessionId,
    options: [{ optionId: 'allow-once', name: 'Allow once', kind: 'allow_once' }],
    toolCall: {
      toolCallId,
      title: 'Edit file',
      rawInput: { path: 'README.md' },
      content: [
        {
          type: 'content',
          content: { type: 'text', text: 'Allow editing README.md?' },
        },
      ],
      _meta: {
        goose: {
          toolCall: {
            toolName: 'edit_file',
          },
        },
      },
    },
  };
}

function elicitationRequest(sessionId: string): {
  id: string;
  sessionId: string;
  request: CreateElicitationRequest & {
    mode: 'form';
    sessionId: string;
  };
} {
  return {
    id: 'acp_elicitation_1',
    sessionId,
    request: {
      mode: 'form',
      sessionId,
      message: 'Choose a project',
      requestedSchema: {
        type: 'object',
        properties: {
          project: {
            type: 'string',
          },
        },
      },
    },
  };
}

function toolProgressNotification(sessionId: string): SessionNotification {
  return {
    sessionId,
    update: {
      sessionUpdate: 'tool_call_update',
      toolCallId: 'tool-1',
      status: 'in_progress',
      _meta: {
        toolNotification: {
          type: 'progress',
          params: {
            progressToken: 'scan-repo',
            progress: 3,
          },
        },
      },
    },
  };
}

describe('acpChatSessionStore', () => {
  let store: AcpChatSessionStore;

  beforeEach(() => {
    store = createAcpChatSessionStore();
  });

  it('finishes session load with session metadata', () => {
    const initialMessage = message('message-1', 'Hello');

    store.setMessages('session-1', [initialMessage]);

    const snapshot = store.finishSessionLoad('session-1', session('session-1'));

    expect(snapshot.session?.id).toBe('session-1');
    expect(snapshot.messages).toEqual([initialMessage]);
    expect(snapshot.tokenState).toMatchObject({
      inputTokens: 0,
      outputTokens: 0,
      totalTokens: 0,
      accumulatedInputTokens: 0,
      accumulatedOutputTokens: 0,
      accumulatedTotalTokens: 0,
    });
    expect(snapshot.chatState).toBe(ChatState.Idle);
    expect(snapshot.sessionLoadError).toBeUndefined();
  });

  it('keeps multiple session snapshots isolated', () => {
    store.setMessages('session-1', [message('message-1', 'One')]);
    store.setMessages('session-2', [message('message-2', 'Two')]);

    expect(store.getSnapshot('session-1')?.messages[0].id).toBe('message-1');
    expect(store.getSnapshot('session-2')?.messages[0].id).toBe('message-2');
  });

  it('deletes session snapshots', () => {
    store.setMessages('session-1', [message('message-1', 'One')]);

    store.deleteSnapshot('session-1');

    expect(store.getSnapshot('session-1')).toBeUndefined();
  });

  it('ignores stale prompt attempts and leaves the current attempt active', () => {
    store.startPromptAttempt('session-1', 'attempt-a');
    store.startPromptAttempt('session-1', 'attempt-b');

    expect(store.finishPromptAttemptIfCurrent('session-1', 'attempt-a', 'late error')).toBe(false);

    expect(store.getSnapshot('session-1')).toMatchObject({
      activePromptAttemptId: 'attempt-b',
      chatState: ChatState.Streaming,
      sessionLoadError: undefined,
    });

    expect(store.finishPromptAttemptIfCurrent('session-1', 'attempt-b')).toBe(true);
    expect(store.getSnapshot('session-1')).toMatchObject({
      activePromptAttemptId: null,
      chatState: ChatState.Idle,
    });
  });

  it('keeps loaded sessions streaming when a prompt attempt is active', () => {
    store.startPromptAttempt('session-1', 'attempt-1');

    const snapshot = store.finishSessionLoad('session-1', session('session-1'));

    expect(snapshot.activePromptAttemptId).toBe('attempt-1');
    expect(snapshot.chatState).toBe(ChatState.Streaming);
  });

  it('stores ACP tool notifications and clears them for a new prompt attempt', () => {
    const snapshot = store.applyAcpSessionNotification(toolProgressNotification('session-1'));

    expect(snapshot.notifications).toHaveLength(1);
    expect(snapshot.notifications[0]).toMatchObject({
      type: 'Notification',
      request_id: 'tool-1',
      message: {
        method: 'notifications/progress',
        params: {
          progressToken: 'scan-repo',
          progress: 3,
        },
      },
    });

    const nextSnapshot = store.startPromptAttempt('session-1', 'attempt-1');

    expect(nextSnapshot.notifications).toEqual([]);
  });

  it('applies permission requests as waiting action-required messages', () => {
    const snapshot = store.applyPermissionRequest(permissionRequest('session-1', 'tool-1'));

    expect(snapshot.chatState).toBe(ChatState.WaitingForUserInput);
    expect(snapshot.messages).toHaveLength(1);
    expect(snapshot.messages[0].role).toBe('assistant');
    expect(snapshot.messages[0].content[0]).toMatchObject({
      type: 'actionRequired',
      data: {
        actionType: 'toolConfirmation',
        id: 'tool-1',
      },
    });
  });

  it('applies elicitation requests as waiting action-required messages', () => {
    const snapshot = store.applyElicitationRequest(elicitationRequest('session-1'));

    expect(snapshot.chatState).toBe(ChatState.WaitingForUserInput);
    expect(snapshot.messages).toHaveLength(1);
    expect(snapshot.messages[0].role).toBe('assistant');
    expect(snapshot.messages[0].content[0]).toMatchObject({
      type: 'actionRequired',
      data: {
        actionType: 'elicitation',
        id: 'acp_elicitation_1',
        message: 'Choose a project',
      },
    });
  });

  it('stores submitted elicitation status', () => {
    store.applyElicitationRequest(elicitationRequest('session-1'));

    const snapshot = store.setElicitationStatus('session-1', 'acp_elicitation_1', 'submitted');

    expect(snapshot?.messages[0].content[0]).toMatchObject({
      type: 'actionRequired',
      data: {
        actionType: 'elicitation',
        id: 'acp_elicitation_1',
        isSubmitted: true,
        isCancelled: false,
      },
    });
  });

  it('stores cancelled elicitation status', () => {
    store.applyElicitationRequest(elicitationRequest('session-1'));

    const snapshot = store.setElicitationStatus('session-1', 'acp_elicitation_1', 'cancelled');

    expect(snapshot?.messages[0].content[0]).toMatchObject({
      type: 'actionRequired',
      data: {
        actionType: 'elicitation',
        id: 'acp_elicitation_1',
        isSubmitted: false,
        isCancelled: true,
      },
    });
  });
});

describe('useAcpChatSessionSnapshot', () => {
  const sessionId = 'hook-session-1';

  afterEach(() => {
    acpChatSessionStore.deleteSnapshot(sessionId);
  });

  it('subscribes to session store snapshots', () => {
    const { result } = renderHook(() => useAcpChatSessionSnapshot(sessionId));

    expect(result.current).toBeUndefined();

    const nextMessage = message('message-1', 'Hello from hook');
    act(() => {
      acpChatSessionStore.setMessages(sessionId, [nextMessage]);
    });

    expect(result.current?.messages).toEqual([nextMessage]);
  });
});
