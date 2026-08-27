import { describe, expect, it } from 'vitest';
import type { AcpChatSessionSnapshot } from '../acp/chatSessionStore';
import { ChatState } from '../types/chatState';
import type { UserInput } from '../types/message';
import { isChatSubmissionAllowed } from './useChatSession';

const input: UserInput = { msg: 'hello', images: [] };

function snapshot(overrides: Partial<AcpChatSessionSnapshot> = {}): AcpChatSessionSnapshot {
  return {
    session: { id: 'session-1' },
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
    ...overrides,
  } as AcpChatSessionSnapshot;
}

describe('isChatSubmissionAllowed', () => {
  it.each([
    ChatState.LoadingConversation,
    ChatState.Streaming,
    ChatState.Thinking,
    ChatState.WaitingForUserInput,
    ChatState.Compacting,
    ChatState.RestartingAgent,
  ])('rejects submissions while the chat state is %s', (chatState) => {
    expect(isChatSubmissionAllowed(false, snapshot({ chatState }), input)).toBe(false);
  });

  it('rejects submissions while recovering or a prompt is active or cancelling', () => {
    expect(isChatSubmissionAllowed(true, snapshot(), input)).toBe(false);
    expect(
      isChatSubmissionAllowed(false, snapshot({ activePromptAttemptId: 'prompt-1' }), input)
    ).toBe(false);
    expect(
      isChatSubmissionAllowed(false, snapshot({ pendingCancelPromptAttemptId: 'prompt-1' }), input)
    ).toBe(false);
  });

  it('rejects missing sessions and empty new conversations', () => {
    expect(isChatSubmissionAllowed(false, snapshot({ session: undefined }), input)).toBe(false);
    expect(isChatSubmissionAllowed(false, snapshot(), { msg: '', images: [] })).toBe(false);
  });

  it('allows idle new messages and continuation of existing conversations', () => {
    expect(isChatSubmissionAllowed(false, snapshot(), input)).toBe(true);
    expect(
      isChatSubmissionAllowed(false, snapshot({ messages: [{}] as never[] }), {
        msg: '',
        images: [],
      })
    ).toBe(true);
  });
});
