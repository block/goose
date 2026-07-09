import type { GooseSessionNotification_unstable } from '@aaif/goose-sdk';
import { describe, expect, it } from 'vitest';
import type { Message, MessageUsage } from '../../types/message';
import { applyGooseSessionNotification } from '../adapter/gooseSessionNotifications';
import type { AcpChatStateChange, AdapterState } from '../adapter/shared';

const SESSION_ID = 'session-1';

function gooseUpdate(
  update: GooseSessionNotification_unstable['update']
): GooseSessionNotification_unstable {
  return {
    sessionId: SESSION_ID,
    update,
  };
}

function message(id: string, role: Message['role']): Message {
  return {
    id,
    role,
    created: 1700000000,
    content: [{ type: 'text', text: `${role} ${id}` }],
    metadata: { userVisible: true, agentVisible: true },
  };
}

function makeState(): AdapterState {
  return {
    messages: [message('u1', 'user'), message('a1', 'assistant'), message('a2', 'assistant')],
    localSteerTextByMessageId: new Map(),
    toolCallStatesById: new Map(),
  };
}

const FULL_USAGE = {
  inputTokens: 1200,
  outputTokens: 340,
  totalTokens: 1540,
  cacheReadTokens: 800,
  cacheWriteTokens: 100,
  cost: 0.0123,
  costSource: 'estimated',
  elapsedMs: 4200,
  timeToFirstTokenMs: 840,
  isCompaction: false,
} satisfies MessageUsage;

function messageUsageNotification(
  messageId: string | undefined,
  usage: MessageUsage = FULL_USAGE
): GooseSessionNotification_unstable {
  return gooseUpdate({
    sessionUpdate: 'message_usage',
    ...(messageId !== undefined ? { messageId } : {}),
    usage,
  });
}

function expectMessagesChange(chatStateChanges: AcpChatStateChange[]): Message[] {
  const chatStateChange = chatStateChanges.find((change) => change.type === 'messages');

  if (!chatStateChange || chatStateChange.type !== 'messages') {
    throw new Error('expected messages state change');
  }

  return chatStateChange.messages;
}

function findTokenStateChange(chatStateChanges: AcpChatStateChange[]) {
  return chatStateChanges.find((change) => change.type === 'tokenState');
}

describe('applyGooseSessionNotification', () => {
  describe('message_usage', () => {
    it('attaches usage to the message matching messageId', () => {
      const state = makeState();

      const changes = applyGooseSessionNotification(state, messageUsageNotification('a1'));

      expect(changes).toHaveLength(2);
      const messages = expectMessagesChange(changes);
      expect(messages).toHaveLength(3);

      const [, first, second] = messages;
      expect(first.id).toBe('a1');
      expect(first.metadata.usage).toEqual(FULL_USAGE);
      expect(second.metadata.usage).toBeUndefined();

      expect(state.messages[1].metadata.usage).toEqual(FULL_USAGE);

      expect(findTokenStateChange(changes)).toEqual({
        type: 'tokenState',
        tokenState: {
          inputTokens: 1200,
          outputTokens: 340,
          totalTokens: 1540,
          cacheReadTokens: 800,
          cacheWriteTokens: 100,
        },
      });
    });

    it('omits nullish usage fields from the token state change', () => {
      const state = makeState();

      const changes = applyGooseSessionNotification(
        state,
        messageUsageNotification('a1', { outputTokens: 340, cacheReadTokens: null })
      );

      expect(findTokenStateChange(changes)).toEqual({
        type: 'tokenState',
        tokenState: { outputTokens: 340 },
      });
    });

    it('emits no token state change when all token fields are nullish', () => {
      const state = makeState();

      const changes = applyGooseSessionNotification(
        state,
        messageUsageNotification('a1', { elapsedMs: 4200 })
      );

      expect(changes).toHaveLength(1);
      expect(findTokenStateChange(changes)).toBeUndefined();
    });

    it('emits no token state change for compaction usage', () => {
      const state = makeState();

      const changes = applyGooseSessionNotification(
        state,
        messageUsageNotification('a1', { ...FULL_USAGE, isCompaction: true })
      );

      expect(changes).toHaveLength(1);
      expect(findTokenStateChange(changes)).toBeUndefined();
      expect(state.messages[1].metadata.usage).toEqual({ ...FULL_USAGE, isCompaction: true });
    });

    it.each([
      ['absent', undefined],
      ['unknown', 'missing'],
    ])('falls back to the last assistant message for an %s messageId', (_name, messageId) => {
      const state = makeState();

      const changes = applyGooseSessionNotification(state, messageUsageNotification(messageId));

      const messages = expectMessagesChange(changes);
      const [user, first, last] = messages;
      expect(user.metadata.usage).toBeUndefined();
      expect(first.metadata.usage).toBeUndefined();
      expect(last.id).toBe('a2');
      expect(last.metadata.usage).toEqual(FULL_USAGE);
    });

    it('skips client-made approval rows when falling back', () => {
      const state = makeState();
      state.messages.push({
        id: 'acp_permission_t1',
        role: 'assistant',
        created: 1700000001,
        content: [
          {
            type: 'actionRequired',
            data: { actionType: 'toolConfirmation', id: 't1', toolName: 'shell', arguments: {} },
          },
        ],
        metadata: { userVisible: true, agentVisible: true },
      });

      const changes = applyGooseSessionNotification(state, messageUsageNotification('missing'));

      const messages = expectMessagesChange(changes);
      expect(messages[3].metadata.usage).toBeUndefined();
      expect(messages[2].id).toBe('a2');
      expect(messages[2].metadata.usage).toEqual(FULL_USAGE);
    });

    it('emits only a token state change when no assistant message exists', () => {
      const state: AdapterState = {
        messages: [message('u1', 'user')],
        localSteerTextByMessageId: new Map(),
        toolCallStatesById: new Map(),
      };

      const changes = applyGooseSessionNotification(state, messageUsageNotification('missing'));

      expect(changes).toHaveLength(1);
      expect(findTokenStateChange(changes)).toBeDefined();
      expect(state.messages[0].metadata.usage).toBeUndefined();
    });
  });

  describe('usage_update', () => {
    it('still maps usage updates into token state', () => {
      const changes = applyGooseSessionNotification(
        makeState(),
        gooseUpdate({
          sessionUpdate: 'usage_update',
          used: 42,
          contextLimit: 200,
          accumulatedInputTokens: 10,
          accumulatedOutputTokens: 15,
          accumulatedCost: 0.12,
        })
      );

      expect(changes).toEqual([
        {
          type: 'tokenState',
          tokenState: {
            totalTokens: 42,
            accumulatedInputTokens: 10,
            accumulatedOutputTokens: 15,
            accumulatedTotalTokens: 25,
            accumulatedCost: 0.12,
          },
        },
      ]);
    });
  });
});
