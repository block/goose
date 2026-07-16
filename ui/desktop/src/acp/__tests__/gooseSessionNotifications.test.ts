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

function expectOnlyMessagesChange(chatStateChanges: AcpChatStateChange[]): Message[] {
  expect(chatStateChanges).toHaveLength(1);

  const [chatStateChange] = chatStateChanges;
  expect(chatStateChange.type).toBe('messages');

  if (chatStateChange.type !== 'messages') {
    throw new Error('expected messages state change');
  }

  return chatStateChange.messages;
}

describe('applyGooseSessionNotification', () => {
  describe('message_usage', () => {
    it('attaches usage to the message matching messageId', () => {
      const state = makeState();

      const changes = applyGooseSessionNotification(state, messageUsageNotification('a1'));

      const messages = expectOnlyMessagesChange(changes);
      expect(messages).toHaveLength(3);

      const [, first, second] = messages;
      expect(first.id).toBe('a1');
      expect(first.metadata.usage).toEqual(FULL_USAGE);
      expect(second.metadata.usage).toBeUndefined();

      expect(state.messages[1].metadata.usage).toEqual(FULL_USAGE);
    });

    it.each([
      ['absent', undefined],
      ['unknown', 'missing'],
    ])('falls back to the last assistant message for an %s messageId', (_name, messageId) => {
      const state = makeState();

      const changes = applyGooseSessionNotification(state, messageUsageNotification(messageId));

      const messages = expectOnlyMessagesChange(changes);
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

      const messages = expectOnlyMessagesChange(changes);
      expect(messages[3].metadata.usage).toBeUndefined();
      expect(messages[2].id).toBe('a2');
      expect(messages[2].metadata.usage).toEqual(FULL_USAGE);
    });

    it('returns no changes and mutates nothing when no assistant message exists', () => {
      const state: AdapterState = {
        messages: [message('u1', 'user')],
        localSteerTextByMessageId: new Map(),
      };

      const changes = applyGooseSessionNotification(state, messageUsageNotification('missing'));

      expect(changes).toEqual([]);
      expect(state.messages[0].metadata.usage).toBeUndefined();
    });
  });

  describe('history_replaced', () => {
    it('removes assistant messages not in the retained set', () => {
      const state = makeState(); // u1(user), a1(assistant), a2(assistant)

      const changes = applyGooseSessionNotification(
        state,
        gooseUpdate({ sessionUpdate: 'history_replaced', retainedMessageIds: ['a2'] })
      );

      const messages = expectOnlyMessagesChange(changes);
      // u1 kept (user messages are always preserved regardless of retained set)
      // a1 removed (not retained)
      // a2 kept (retained)
      expect(messages.map((m) => m.id)).toEqual(['u1', 'a2']);
    });

    it('always keeps user messages even when their client ID is not in the retained set', () => {
      const state = makeState(); // u1 has a client-generated ID that won't match any server ID

      const changes = applyGooseSessionNotification(
        state,
        gooseUpdate({ sessionUpdate: 'history_replaced', retainedMessageIds: ['a1'] })
      );

      const messages = expectOnlyMessagesChange(changes);
      expect(messages.map((m) => m.id)).toEqual(['u1', 'a1']);
    });

    it('always keeps client-generated messages (agentVisible: false)', () => {
      const state = makeState();
      state.messages.push({
        id: `acp_status_${SESSION_ID}_123`,
        role: 'assistant',
        created: 1700000002,
        content: [
          { type: 'systemNotification', notificationType: 'inlineMessage', msg: 'Compaction complete' },
        ],
        metadata: { userVisible: true, agentVisible: false },
      });

      const changes = applyGooseSessionNotification(
        state,
        gooseUpdate({ sessionUpdate: 'history_replaced', retainedMessageIds: ['a2'] })
      );

      const messages = expectOnlyMessagesChange(changes);
      // u1(user) kept, a1 removed, a2 kept, status message kept
      expect(messages.map((m) => m.id)).toEqual(['u1', 'a2', `acp_status_${SESSION_ID}_123`]);
    });

    it('removes all assistant messages when retained set is empty', () => {
      const state = makeState();

      const changes = applyGooseSessionNotification(
        state,
        gooseUpdate({ sessionUpdate: 'history_replaced', retainedMessageIds: [] })
      );

      const messages = expectOnlyMessagesChange(changes);
      // only the user message survives
      expect(messages.map((m) => m.id)).toEqual(['u1']);
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
