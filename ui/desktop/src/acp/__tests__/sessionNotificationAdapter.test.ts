import type { GooseSessionNotification_unstable as GooseSessionNotification } from '@aaif/goose-sdk';
import type { RequestPermissionRequest, SessionNotification } from '@agentclientprotocol/sdk';
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

function goosePendingElicitationNotification(): GooseSessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'interaction_update',
      interaction: {
        type: 'elicitation',
        id: 'elicitation-1',
        state: 'pending',
        message: 'Please provide deployment details',
        requestedSchema: {
          type: 'object',
          properties: {
            environment: { type: 'string' },
          },
        },
      },
      _meta: {
        goose: {
          created: 1_700_000_000,
          messageId: 'elicitation-message',
        },
      },
    },
  } as unknown as GooseSessionNotification;
}

function gooseSubmittedElicitationNotification(): GooseSessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'interaction_update',
      interaction: {
        type: 'elicitation',
        id: 'elicitation-1',
        state: 'submitted',
      },
    },
  } as unknown as GooseSessionNotification;
}

function gooseStatusMessageNotification(
  status: Extract<GooseSessionNotification['update'], { sessionUpdate: 'status_message' }>['status']
): GooseSessionNotification {
  return {
    sessionId: 'session-1',
    update: {
      sessionUpdate: 'status_message',
      status,
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

function permissionRequest(): RequestPermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId: 'tool-1',
      title: 'Shell',
      rawInput: { command: 'ls -1 ~/Desktop | wc -l' },
      content: [
        {
          type: 'content',
          content: {
            type: 'text',
            text: 'Run this command?',
          },
        },
      ],
      _meta: {
        goose: {
          toolCall: {
            toolName: 'developer__shell',
            extensionName: 'developer',
          },
        },
      },
    },
    options: [
      { optionId: 'allow_once', name: 'allow_once', kind: 'allow_once' },
      { optionId: 'allow_always', name: 'allow_always', kind: 'allow_always' },
      { optionId: 'reject_once', name: 'reject_once', kind: 'reject_once' },
      { optionId: 'reject_always', name: 'reject_always', kind: 'reject_always' },
    ],
  } as RequestPermissionRequest;
}

function permissionRequestForTool(toolCallId: string): RequestPermissionRequest {
  const request = permissionRequest();
  return {
    ...request,
    toolCall: {
      ...request.toolCall,
      toolCallId,
    },
  };
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

  it('appends consecutive text chunks without message IDs to the current role message', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(textNotificationWithoutMessageId('agent_message_chunk', 'Hello'));
    adapter.apply(textNotificationWithoutMessageId('agent_message_chunk', ' Summer'));
    adapter.apply(textNotificationWithoutMessageId('agent_message_chunk', '!'));

    expect(adapter.snapshot().messages).toMatchObject([
      {
        role: 'assistant',
        content: [{ type: 'text', text: 'Hello Summer!' }],
      },
    ]);
    expect(adapter.snapshot().messages[0].id).toBeUndefined();
  });

  it('does not duplicate replayed optimistic user text without a message ID', () => {
    const adapter = createAcpSessionNotificationAdapter([
      {
        role: 'user',
        created: 1,
        content: [{ type: 'text', text: 'hellohello' }],
        metadata: { userVisible: true, agentVisible: true },
      },
    ]);

    adapter.apply(textNotificationWithoutMessageId('user_message_chunk', 'hellohello'));

    expect(adapter.snapshot().messages).toMatchObject([
      {
        role: 'user',
        content: [{ type: 'text', text: 'hellohello' }],
      },
    ]);
  });

  it('uses cumulative text chunks without message IDs as the latest text', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(textNotificationWithoutMessageId('agent_message_chunk', 'Hello'));
    adapter.apply(textNotificationWithoutMessageId('agent_message_chunk', 'Hello Summer'));
    adapter.apply(textNotificationWithoutMessageId('agent_message_chunk', 'Hello Summer!'));

    expect(adapter.snapshot().messages).toMatchObject([
      {
        role: 'assistant',
        content: [{ type: 'text', text: 'Hello Summer!' }],
      },
    ]);
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

  it('does not duplicate the same ACP image content block for an existing message', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(imageNotification());
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

  it('converts config option updates', () => {
    const adapter = createAcpSessionNotificationAdapter();
    const configOptions = [
      {
        id: 'mode',
        name: 'Mode',
        type: 'select',
        currentValue: 'approve',
        options: [{ value: 'approve', name: 'Manual' }],
        category: 'mode',
      },
    ];

    const updates = adapter.apply({
      sessionId: 'session-1',
      update: {
        sessionUpdate: 'config_option_update',
        configOptions,
      },
    } as SessionNotification);

    expect(updates).toEqual([{ type: 'configOptions', configOptions }]);
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

  it('converts Goose notice status messages into local inline notifications', () => {
    const adapter = createAcpSessionNotificationAdapter();

    const updates = adapter.applyGoose(
      gooseStatusMessageNotification({
        type: 'notice',
        message: 'Context limit reached. Compacting to continue conversation...',
      })
    );

    expect(updates).toMatchObject([
      {
        type: 'messages',
        messages: [
          {
            role: 'assistant',
            content: [
              {
                type: 'systemNotification',
                notificationType: 'inlineMessage',
                msg: 'Context limit reached. Compacting to continue conversation...',
              },
            ],
            metadata: { userVisible: true, agentVisible: false },
          },
        ],
      },
    ]);
  });

  it('keeps Goose status messages in the adapter snapshot after later ACP messages', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.applyGoose(
      gooseStatusMessageNotification({
        type: 'notice',
        message: 'Compaction completed',
      })
    );

    const updates = adapter.apply(textNotification('agent_message_chunk', 'Next response'));

    expect(updates).toMatchObject([
      {
        type: 'messages',
        messages: [
          {
            role: 'assistant',
            content: [
              {
                type: 'systemNotification',
                notificationType: 'inlineMessage',
                msg: 'Compaction completed',
              },
            ],
          },
          {
            role: 'assistant',
            content: [{ type: 'text', text: 'Next response' }],
          },
        ],
      },
    ]);
    expect(adapter.snapshot().messages).toMatchObject([
      {
        role: 'assistant',
        content: [
          {
            type: 'systemNotification',
            notificationType: 'inlineMessage',
            msg: 'Compaction completed',
          },
        ],
      },
      {
        role: 'assistant',
        content: [{ type: 'text', text: 'Next response' }],
      },
    ]);
  });

  it('does not merge id-less thought chunks into Goose status messages', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.applyGoose(
      gooseStatusMessageNotification({
        type: 'progress',
        message: 'goose is compacting the conversation...',
      })
    );

    const updates = adapter.apply(thoughtNotification('checking the next step'));

    expect(updates).toMatchObject([
      {
        type: 'messages',
        messages: [
          {
            role: 'assistant',
            content: [
              {
                type: 'systemNotification',
                notificationType: 'thinkingMessage',
                msg: 'goose is compacting the conversation...',
              },
            ],
          },
          {
            role: 'assistant',
            content: [{ type: 'thinking', thinking: 'checking the next step' }],
          },
        ],
      },
    ]);
  });

  it('converts Goose progress status messages into local thinking notifications', () => {
    const adapter = createAcpSessionNotificationAdapter();

    const updates = adapter.applyGoose(
      gooseStatusMessageNotification({
        type: 'progress',
        message: 'goose is compacting the conversation...',
      })
    );

    expect(updates).toMatchObject([
      {
        type: 'messages',
        messages: [
          {
            role: 'assistant',
            content: [
              {
                type: 'systemNotification',
                notificationType: 'thinkingMessage',
                msg: 'goose is compacting the conversation...',
              },
            ],
            metadata: { userVisible: true, agentVisible: false },
          },
        ],
      },
    ]);
  });

  it('converts Goose pending elicitation updates into desktop action-required messages', () => {
    const adapter = createAcpSessionNotificationAdapter();

    const updates = adapter.applyGoose(goosePendingElicitationNotification());

    expect(updates).toMatchObject([
      {
        type: 'messages',
        messages: [
          {
            id: 'elicitation-message',
            created: 1_700_000_000,
            role: 'assistant',
            content: [
              {
                type: 'actionRequired',
                data: {
                  actionType: 'elicitation',
                  id: 'elicitation-1',
                  message: 'Please provide deployment details',
                  requested_schema: {
                    type: 'object',
                    properties: {
                      environment: { type: 'string' },
                    },
                  },
                },
              },
            ],
          },
        ],
      },
    ]);
  });

  it('removes pending elicitation messages after Goose submitted updates', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.applyGoose(goosePendingElicitationNotification());
    const updates = adapter.applyGoose(gooseSubmittedElicitationNotification());

    expect(updates).toEqual([{ type: 'messages', messages: [] }]);
    expect(adapter.snapshot().messages).toHaveLength(0);
  });

  it('converts an ACP permission request into desktop tool confirmation content', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.applyPermissionRequest(permissionRequest());

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'acp_permission_tool-1',
        role: 'assistant',
        content: [
          {
            type: 'actionRequired',
            data: {
              actionType: 'toolConfirmation',
              id: 'tool-1',
              toolName: 'developer__shell',
              arguments: { command: 'ls -1 ~/Desktop | wc -l' },
              prompt: 'Run this command?',
            },
          },
        ],
      },
    ]);
  });

  it('gives ACP permission messages unique IDs for React keys', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.applyPermissionRequest(permissionRequestForTool('tool-1'));
    adapter.applyPermissionRequest(permissionRequestForTool('tool-2'));

    expect(adapter.snapshot().messages.map((message) => message.id)).toEqual([
      'acp_permission_tool-1',
      'acp_permission_tool-2',
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

  it('does not create an empty assistant message for an orphan completed tool update', () => {
    const adapter = createAcpSessionNotificationAdapter();

    adapter.apply(toolCallUpdateNotification());

    expect(adapter.snapshot().messages).toMatchObject([
      {
        id: 'tool-response-message',
        role: 'user',
        content: [
          {
            type: 'toolResponse',
            id: 'tool-1',
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
