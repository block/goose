import type { GooseSessionNotification } from '@aaif/goose-sdk';
import type { SessionNotification } from '@agentclientprotocol/sdk';
import { describe, expect, it } from 'vitest';
import type { Message } from '../../api';
import { createAcpSessionNotificationAdapter } from '../sessionNotificationAdapter';

function textNotification(
  sessionUpdate: 'user_message_chunk' | 'agent_message_chunk',
  text: string,
  messageId = 'message-1'
): SessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate,
      messageId,
      content: {
        type: 'text',
        text,
      },
    },
  } as SessionNotification;
}

function textNotificationWithoutMessageId(
  sessionUpdate: 'user_message_chunk' | 'agent_message_chunk',
  text: string
): SessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate,
      content: {
        type: 'text',
        text,
      },
    },
  } as SessionNotification;
}

function thoughtNotification(text: string, messageId = 'message-1'): SessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'agent_thought_chunk',
      messageId,
      content: {
        type: 'text',
        text,
      },
    },
  } as SessionNotification;
}

function imageNotification(messageId = 'message-1'): SessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'user_message_chunk',
      messageId,
      content: {
        type: 'image',
        data: 'abc',
        mimeType: 'image/png',
      },
    },
  } as SessionNotification;
}

function replayTextNotification(
  sessionUpdate: 'user_message_chunk' | 'agent_message_chunk',
  text: string,
  meta: { created?: number; messageId?: string }
): SessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate,
      content: {
        type: 'text',
        text,
      },
      _meta: {
        goose: meta,
      },
    },
  } as SessionNotification;
}

function gooseUsageNotification(): GooseSessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'usage_update',
      used: 42,
      contextLimit: 100,
      accumulatedInputTokens: 10,
      accumulatedOutputTokens: 20,
      accumulatedCost: 0.12,
    },
  };
}

function toolCallNotification(): SessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'tool_call',
      toolCallId: 'tool-1',
      title: 'Read file',
      rawInput: { path: 'README.md' },
      status: 'in_progress',
      _meta: {
        goose: {
          created: 1_700_000_000,
          messageId: 'assistant-message',
          toolCall: {
            toolName: 'developer__shell',
            extensionName: 'developer',
          },
        },
      },
    },
  } as SessionNotification;
}

function toolCallUpdateNotification(): SessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'tool_call_update',
      toolCallId: 'tool-1',
      status: 'completed',
      content: [
        {
          type: 'content',
          content: {
            type: 'text',
            text: 'file contents',
          },
        },
      ],
      _meta: {
        goose: {
          messageId: 'tool-response-message',
        },
      },
    },
  } as SessionNotification;
}

function mcpAppToolCallNotification(): SessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'tool_call',
      toolCallId: 'tool-1',
      title: 'Render weather',
      rawInput: { city: 'Oakland' },
      _meta: {
        goose: {
          messageId: 'assistant-message',
          toolCall: {
            toolName: 'weather__render',
            extensionName: 'weather',
          },
          mcpApp: {
            resourceUri: 'ui://weather/app',
            extensionName: 'weather',
            toolName: 'weather__render',
          },
        },
      },
    },
  } as SessionNotification;
}

function mcpAppToolCallUpdateNotification(): SessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'tool_call_update',
      toolCallId: 'tool-1',
      status: 'completed',
      content: [],
      _meta: {
        goose: {
          messageId: 'tool-response-message',
          mcpApp: {
            resourceUri: 'ui://weather/app',
            extensionName: 'weather',
            toolName: 'weather__render',
          },
        },
      },
    },
  } as SessionNotification;
}

describe('sessionNotificationAdapter', () => {
  it('converts a user text chunk into a desktop message', () => {
    const adapter = createAcpSessionNotificationAdapter();

    const updates = adapter.apply(textNotification('user_message_chunk', 'hello'));

    expect(updates).toEqual([
      {
        type: 'messages',
        messages: [
          {
            id: 'message-1',
            role: 'user',
            created: expect.any(Number),
            content: [{ type: 'text', text: 'hello' }],
            metadata: { userVisible: true, agentVisible: true },
          },
        ],
      },
    ]);
  });

  it('converts an agent text chunk into an assistant desktop message', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(textNotification('agent_message_chunk', 'hi'));

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'message-1',
        role: 'assistant',
        content: [{ type: 'text', text: 'hi' }],
      },
    ]);
  });

  it('uses replay metadata for historical message created time and ID', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(
      replayTextNotification('agent_message_chunk', 'history', {
        created: 1_700_000_000,
        messageId: 'historical-message',
      })
    );

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'historical-message',
        created: 1_700_000_000,
        role: 'assistant',
        content: [{ type: 'text', text: 'history' }],
      },
    ]);
  });

  it('does not invent fallback message IDs', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(textNotificationWithoutMessageId('agent_message_chunk', 'no id'));

    expect(adapter.snapshot().messages).toMatchObject([
      {
        role: 'assistant',
        content: [{ type: 'text', text: 'no id' }],
      },
    ]);
    expect(adapter.snapshot().messages[0].id).toBeUndefined();
  });

  it('appends text chunks with the same role and message ID', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(textNotification('agent_message_chunk', 'hello ', 'message-1'));
    adapter.apply(textNotification('agent_message_chunk', 'there', 'message-1'));

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'message-1',
        role: 'assistant',
        content: [{ type: 'text', text: 'hello there' }],
      },
    ]);
  });

  it('does not mutate seeded messages when appending chunks', () => {
    const initialMessages: Message[] = [
      {
        id: 'message-1',
        role: 'assistant',
        created: 1,
        content: [{ type: 'text', text: 'hello ' }],
        metadata: { userVisible: true, agentVisible: true },
      },
    ];
    const adapter = createAcpSessionNotificationAdapter(initialMessages);

    adapter.apply(textNotification('agent_message_chunk', 'there', 'message-1'));

    expect(initialMessages[0].content).toEqual([{ type: 'text', text: 'hello ' }]);
    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'message-1',
        content: [{ type: 'text', text: 'hello there' }],
      },
    ]);
  });

  it('keeps different message IDs as separate messages', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(textNotification('agent_message_chunk', 'first', 'message-1'));
    adapter.apply(textNotification('agent_message_chunk', 'second', 'message-2'));

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'message-1',
        content: [{ type: 'text', text: 'first' }],
      },
      {
        id: 'message-2',
        content: [{ type: 'text', text: 'second' }],
      },
    ]);
  });

  it('converts an ACP image content block into desktop image content', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(imageNotification());

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'message-1',
        role: 'user',
        content: [{ type: 'image', data: 'abc', mimeType: 'image/png' }],
      },
    ]);
  });

  it('converts an agent thought chunk into assistant thinking content', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(thoughtNotification('thinking'));

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'message-1',
        role: 'assistant',
        content: [{ type: 'thinking', thinking: 'thinking', signature: '' }],
      },
    ]);
  });

  it('appends thought chunks with the same message ID', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(thoughtNotification('step one ', 'message-1'));
    adapter.apply(thoughtNotification('step two', 'message-1'));

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'message-1',
        role: 'assistant',
        content: [{ type: 'thinking', thinking: 'step one step two', signature: '' }],
      },
    ]);
  });

  it('converts session info title updates', () => {
    const adapter = createAcpSessionNotificationAdapter();

    const updates = adapter.apply({
      sessionId: 'session-1',
      update: {
        sessionUpdate: 'session_info_update',
        title: 'New title',
      },
    } as SessionNotification);

    expect(updates).toEqual([{ type: 'sessionInfo', name: 'New title' }]);
  });

  it('converts Goose usage updates into token state updates', () => {
    const adapter = createAcpSessionNotificationAdapter();

    const updates = adapter.applyGoose(gooseUsageNotification());

    expect(updates).toEqual([
      {
        type: 'tokenState',
        tokenState: {
          totalTokens: 42,
          accumulatedInputTokens: 10,
          accumulatedOutputTokens: 20,
          accumulatedTotalTokens: 30,
          accumulatedCost: 0.12,
        },
      },
    ]);
  });

  it('converts ACP tool calls and completed tool updates into desktop tool content', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(toolCallNotification());
    adapter.apply(toolCallUpdateNotification());

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'assistant-message',
        created: 1_700_000_000,
        role: 'assistant',
        content: [
          {
            type: 'toolRequest',
            id: 'tool-1',
            toolCall: {
              status: 'success',
              value: {
                name: 'developer__shell',
                arguments: { path: 'README.md' },
              },
            },
            metadata: {
              title: 'Read file',
              extensionName: 'developer',
            },
          },
        ],
      },
      {
        id: 'tool-response-message',
        role: 'user',
        content: [
          {
            type: 'toolResponse',
            id: 'tool-1',
            toolResult: {
              status: 'success',
              value: {
                content: [{ type: 'text', text: 'file contents' }],
              },
            },
          },
        ],
      },
    ]);
  });

  it('converts Goose MCP app metadata into the desktop UI resource metadata shape', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(mcpAppToolCallNotification());
    adapter.apply(mcpAppToolCallUpdateNotification());

    expect(adapter.snapshot().messages).toMatchObject([
      {
        content: [
          {
            type: 'toolRequest',
            id: 'tool-1',
            toolCall: {
              status: 'success',
              value: {
                name: 'weather__render',
                arguments: { city: 'Oakland' },
              },
            },
            _meta: {
              ui: {
                resourceUri: 'ui://weather/app',
              },
              extensionName: 'weather',
              toolName: 'weather__render',
            },
          },
        ],
      },
      {
        content: [
          {
            type: 'toolResponse',
            id: 'tool-1',
            toolResult: {
              status: 'success',
              value: {
                content: [],
                _meta: {
                  ui: {
                    resourceUri: 'ui://weather/app',
                  },
                  extensionName: 'weather',
                  toolName: 'weather__render',
                },
              },
            },
          },
        ],
      },
    ]);
  });

  it('ignores ACP content blocks that do not have a desktop message shape yet', () => {
    const adapter = createAcpSessionNotificationAdapter();

    const updates = adapter.apply({
      sessionId: 'session-1',
      update: {
        sessionUpdate: 'agent_message_chunk',
        messageId: 'message-1',
        content: {
          type: 'audio',
          data: 'abc',
          mimeType: 'audio/wav',
        },
      },
    } as SessionNotification);

    expect(updates).toEqual([]);
    expect(adapter.snapshot().messages).toEqual([]);
  });
});
