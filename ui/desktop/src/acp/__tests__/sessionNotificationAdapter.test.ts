import type { GoslingSessionNotification_unstable } from '@repo-makeover/gosling-sdk';
import type { RequestPermissionRequest, SessionNotification } from '@agentclientprotocol/sdk';
import { describe, expect, it } from 'vitest';
import type { Message, NotificationEvent } from '../../types/message';
import {
  createAcpSessionNotificationAdapter,
  type AcpChatStateChange,
} from '../sessionNotificationAdapter';

const SESSION_ID = 'session-1';

function acpUpdate(update: SessionNotification['update']): SessionNotification {
  return {
    sessionId: SESSION_ID,
    update,
  };
}

function goslingUpdate(
  update: GoslingSessionNotification_unstable['update']
): GoslingSessionNotification_unstable {
  return {
    sessionId: SESSION_ID,
    update,
  };
}

function agentText(text: string): SessionNotification {
  return acpUpdate({
    sessionUpdate: 'agent_message_chunk',
    content: { type: 'text', text },
  });
}

function userText(text: string): SessionNotification {
  return acpUpdate({
    sessionUpdate: 'user_message_chunk',
    content: { type: 'text', text },
  });
}

function agentThought(text: string): SessionNotification {
  return acpUpdate({
    sessionUpdate: 'agent_thought_chunk',
    content: { type: 'text', text },
  });
}

function agentImage(data: string, mimeType: string): SessionNotification {
  return acpUpdate({
    sessionUpdate: 'agent_message_chunk',
    content: { type: 'image', data, mimeType },
  });
}

function expectOnlyMessagesChange(
  adapter: ReturnType<typeof createAcpSessionNotificationAdapter>,
  chatStateChanges: AcpChatStateChange[]
): Message[] {
  expect(chatStateChanges).toHaveLength(1);

  const [chatStateChange] = chatStateChanges;
  expect(['messages', 'messageUpserted']).toContain(chatStateChange.type);

  if (chatStateChange.type === 'messages') {
    return chatStateChange.messages;
  }
  if (chatStateChange.type === 'messageUpserted') {
    return adapter.getMessages();
  }

  throw new Error('expected messages state change');
}

function expectMessagesAndLocalSteerConfirmation(
  adapter: ReturnType<typeof createAcpSessionNotificationAdapter>,
  chatStateChanges: AcpChatStateChange[],
  messageId: string
): Message[] {
  expect(chatStateChanges).toHaveLength(2);

  const [messagesChange, confirmationChange] = chatStateChanges;
  expect(['messages', 'messageUpserted']).toContain(messagesChange.type);
  expect(confirmationChange).toEqual({ type: 'localSteerConfirmed', messageId });

  if (messagesChange.type === 'messages') {
    return messagesChange.messages;
  }
  if (messagesChange.type === 'messageUpserted') {
    return adapter.getMessages();
  }

  throw new Error('expected messages state change');
}

function expectOnlyNotificationChange(chatStateChanges: AcpChatStateChange[]): NotificationEvent {
  expect(chatStateChanges).toHaveLength(1);

  const [chatStateChange] = chatStateChanges;
  expect(chatStateChange.type).toBe('notification');

  if (chatStateChange.type !== 'notification') {
    throw new Error('expected notification state change');
  }

  return chatStateChange.notification;
}

function firstContent(message: Message): Message['content'][number] {
  const content = message.content[0];
  expect(content).toBeDefined();
  return content;
}

describe('createAcpSessionNotificationAdapter', () => {
  describe('apply', () => {
    describe('message chunks', () => {
      it('maps and merges text chunks by role', () => {
        const adapter = createAcpSessionNotificationAdapter();

        adapter.apply(agentText('Hello '));

        const secondChunkStateChanges = adapter.apply(agentText('world'));
        let messages = expectOnlyMessagesChange(adapter, secondChunkStateChanges);

        expect(messages).toHaveLength(1);
        expect(messages[0].role).toBe('assistant');
        expect(firstContent(messages[0])).toMatchObject({ type: 'text', text: 'Hello world' });

        const userTextStateChanges = adapter.apply(userText('Question'));
        messages = expectOnlyMessagesChange(adapter, userTextStateChanges);

        expect(messages).toHaveLength(2);
        expect(messages[1].role).toBe('user');
        expect(firstContent(messages[1])).toMatchObject({ type: 'text', text: 'Question' });
      });

      it('appends repeated adjacent text deltas', () => {
        const adapter = createAcpSessionNotificationAdapter();

        adapter.apply(agentText('Hel'));
        const messages = expectOnlyMessagesChange(adapter, adapter.apply(agentText('l')));

        expect(messages).toHaveLength(1);
        expect(firstContent(messages[0])).toMatchObject({ type: 'text', text: 'Hell' });
      });

      it('reconciles locally rendered steer text with server chunks', () => {
        const adapter = createAcpSessionNotificationAdapter([
          {
            id: 'steer-1',
            role: 'user',
            created: 123,
            content: [
              { type: 'text', text: 'hello' },
              { type: 'image', data: 'base64-image', mimeType: 'image/png' },
            ],
            metadata: { userVisible: true, agentVisible: true, steer: true },
          },
        ]);

        let messages = expectMessagesAndLocalSteerConfirmation(
          adapter,
          adapter.apply(
            acpUpdate({
              sessionUpdate: 'user_message_chunk',
              content: { type: 'text', text: 'hel' },
              _meta: {
                gosling: {
                  messageId: 'steer-1',
                  steer: true,
                },
              },
            } as SessionNotification['update'])
          ),
          'steer-1'
        );

        expect(firstContent(messages[0])).toMatchObject({ type: 'text', text: 'hel' });
        expect(messages[0].content[1]).toMatchObject({
          type: 'image',
          data: 'base64-image',
          mimeType: 'image/png',
        });
        expect(messages[0].metadata.steer).toBe(true);

        messages = expectMessagesAndLocalSteerConfirmation(
          adapter,
          adapter.apply(
            acpUpdate({
              sessionUpdate: 'user_message_chunk',
              content: { type: 'text', text: 'lo' },
              _meta: {
                gosling: {
                  messageId: 'steer-1',
                  steer: true,
                },
              },
            } as SessionNotification['update'])
          ),
          'steer-1'
        );

        expect(firstContent(messages[0])).toMatchObject({ type: 'text', text: 'hello' });

        messages = expectMessagesAndLocalSteerConfirmation(
          adapter,
          adapter.apply(
            acpUpdate({
              sessionUpdate: 'user_message_chunk',
              content: { type: 'image', data: 'base64-image', mimeType: 'image/png' },
              _meta: {
                gosling: {
                  messageId: 'steer-1',
                  steer: true,
                },
              },
            } as SessionNotification['update'])
          ),
          'steer-1'
        );

        expect(messages[0].content).toEqual([
          { type: 'text', text: 'hello' },
          { type: 'image', data: 'base64-image', mimeType: 'image/png' },
        ]);
      });

      it('appends repeated local steer text deltas without collapsing them', () => {
        const adapter = createAcpSessionNotificationAdapter([
          {
            id: 'steer-1',
            role: 'user',
            created: 123,
            content: [{ type: 'text', text: 'haha' }],
            metadata: { userVisible: true, agentVisible: true, steer: true },
          },
        ]);

        let messages = expectMessagesAndLocalSteerConfirmation(
          adapter,
          adapter.apply(
            acpUpdate({
              sessionUpdate: 'user_message_chunk',
              content: { type: 'text', text: 'ha' },
              _meta: {
                gosling: {
                  messageId: 'steer-1',
                  steer: true,
                },
              },
            } as SessionNotification['update'])
          ),
          'steer-1'
        );

        expect(firstContent(messages[0])).toMatchObject({ type: 'text', text: 'ha' });

        messages = expectMessagesAndLocalSteerConfirmation(
          adapter,
          adapter.apply(
            acpUpdate({
              sessionUpdate: 'user_message_chunk',
              content: { type: 'text', text: 'ha' },
              _meta: {
                gosling: {
                  messageId: 'steer-1',
                  steer: true,
                },
              },
            } as SessionNotification['update'])
          ),
          'steer-1'
        );

        expect(firstContent(messages[0])).toMatchObject({ type: 'text', text: 'haha' });
      });

      it('maps image and thinking chunks to existing message content shapes', () => {
        const imageAdapter = createAcpSessionNotificationAdapter();

        const imageStateChanges = imageAdapter.apply(agentImage('base64-image', 'image/png'));
        const imageMessages = expectOnlyMessagesChange(imageAdapter, imageStateChanges);

        expect(firstContent(imageMessages[0])).toMatchObject({
          type: 'image',
          data: 'base64-image',
          mimeType: 'image/png',
        });

        const thoughtAdapter = createAcpSessionNotificationAdapter();
        thoughtAdapter.apply(agentThought('Thinking '));

        const thoughtStateChanges = thoughtAdapter.apply(agentThought('more'));
        const thoughtMessages = expectOnlyMessagesChange(thoughtAdapter, thoughtStateChanges);

        expect(thoughtMessages).toHaveLength(1);
        expect(firstContent(thoughtMessages[0])).toMatchObject({
          type: 'thinking',
          thinking: 'Thinking more',
          signature: '',
        });
      });
    });

    describe('tools', () => {
      it('maps tool calls and successful responses, including MCP app metadata', () => {
        const adapter = createAcpSessionNotificationAdapter();

        const toolCallStateChanges = adapter.apply(
          acpUpdate({
            sessionUpdate: 'tool_call',
            toolCallId: 'tool-1',
            title: 'Read file',
            kind: 'read',
            status: 'in_progress',
            rawInput: { path: 'README.md' },
            locations: [{ path: 'README.md', line: 1 }],
            _meta: {
              gosling: {
                toolCall: {
                  extensionName: 'developer',
                  toolName: 'read_file',
                },
              },
            },
          })
        );
        let messages = expectOnlyMessagesChange(adapter, toolCallStateChanges);

        expect(messages).toHaveLength(1);
        expect(messages[0].role).toBe('assistant');
        expect(firstContent(messages[0])).toMatchObject({
          type: 'toolRequest',
          id: 'tool-1',
          toolCall: {
            status: 'success',
            value: {
              name: 'read_file',
              arguments: { path: 'README.md' },
            },
          },
          metadata: {
            title: 'Read file',
            status: 'in_progress',
            extensionName: 'developer',
            kind: 'read',
            locations: [{ path: 'README.md', line: 1 }],
          },
        });

        const toolResponseStateChanges = adapter.apply(
          acpUpdate({
            sessionUpdate: 'tool_call_update',
            toolCallId: 'tool-1',
            status: 'completed',
            rawOutput: 'raw result',
            content: [
              {
                type: 'content',
                content: { type: 'text', text: 'rendered result' },
              },
            ],
            _meta: {
              gosling: {
                mcpApp: {
                  resourceUri: 'ui://app/resource',
                  extensionName: 'developer',
                  toolName: 'read_file',
                },
              },
            },
          })
        );
        messages = expectOnlyMessagesChange(adapter, toolResponseStateChanges);

        expect(messages).toHaveLength(2);
        expect(messages[1].role).toBe('user');
        expect(firstContent(messages[1])).toMatchObject({
          type: 'toolResponse',
          id: 'tool-1',
          toolResult: {
            status: 'success',
            value: {
              content: [{ type: 'text', text: 'rendered result' }],
              isError: false,
              _meta: {
                ui: { resourceUri: 'ui://app/resource' },
                extensionName: 'developer',
                toolName: 'read_file',
              },
            },
          },
          metadata: {
            status: 'completed',
            rawOutput: 'raw result',
          },
        });
      });

      it('maps failed tool responses to error results', () => {
        const adapter = createAcpSessionNotificationAdapter();

        const failedToolStateChanges = adapter.apply(
          acpUpdate({
            sessionUpdate: 'tool_call_update',
            toolCallId: 'tool-1',
            status: 'failed',
            title: 'Read file',
            rawOutput: 'permission denied',
          })
        );
        const messages = expectOnlyMessagesChange(adapter, failedToolStateChanges);

        expect(messages).toHaveLength(1);
        expect(messages[0].role).toBe('user');
        expect(firstContent(messages[0])).toMatchObject({
          type: 'toolResponse',
          id: 'tool-1',
          toolResult: {
            status: 'error',
            error: 'permission denied',
          },
          metadata: {
            title: 'Read file',
            status: 'failed',
            rawOutput: 'permission denied',
          },
        });
      });

      it('preserves structured tool output from rawOutput', () => {
        const adapter = createAcpSessionNotificationAdapter();

        adapter.apply(
          acpUpdate({
            sessionUpdate: 'tool_call',
            toolCallId: 'tool-1',
            title: 'Inspect data',
            rawInput: {},
          })
        );

        const toolResponseStateChanges = adapter.apply(
          acpUpdate({
            sessionUpdate: 'tool_call_update',
            toolCallId: 'tool-1',
            status: 'completed',
            rawOutput: { ok: true, count: 2 },
          })
        );
        const messages = expectOnlyMessagesChange(adapter, toolResponseStateChanges);

        expect(messages).toHaveLength(2);
        expect(firstContent(messages[1])).toMatchObject({
          type: 'toolResponse',
          id: 'tool-1',
          toolResult: {
            status: 'success',
            value: {
              content: [],
              structuredContent: { ok: true, count: 2 },
              isError: false,
            },
          },
        });
      });

      it('uses failed tool response text content when raw output is absent', () => {
        const adapter = createAcpSessionNotificationAdapter();

        const failedToolStateChanges = adapter.apply(
          acpUpdate({
            sessionUpdate: 'tool_call_update',
            toolCallId: 'tool-1',
            status: 'failed',
            title: 'Read file',
            content: [
              {
                type: 'content',
                content: { type: 'text', text: 'file not found' },
              },
            ],
          })
        );
        const messages = expectOnlyMessagesChange(adapter, failedToolStateChanges);

        expect(firstContent(messages[0])).toMatchObject({
          type: 'toolResponse',
          id: 'tool-1',
          toolResult: {
            status: 'error',
            error: 'file not found',
          },
        });
      });

      it('maps in-progress tool message notifications', () => {
        const adapter = createAcpSessionNotificationAdapter();

        const notificationStateChanges = adapter.apply(
          acpUpdate({
            sessionUpdate: 'tool_call_update',
            toolCallId: 'tool-1',
            status: 'in_progress',
            _meta: {
              toolNotification: {
                type: 'message',
                params: {
                  level: 'info',
                  logger: 'subagent:session-1',
                  data: {
                    text: 'Running search...',
                  },
                },
              },
            },
          })
        );
        const notification = expectOnlyNotificationChange(notificationStateChanges);

        expect(notification).toMatchObject({
          type: 'Notification',
          request_id: 'tool-1',
          message: {
            method: 'notifications/message',
            params: {
              level: 'info',
              logger: 'subagent:session-1',
              data: {
                text: 'Running search...',
              },
            },
          },
        });
      });

      it('maps in-progress tool progress notifications', () => {
        const adapter = createAcpSessionNotificationAdapter();

        const notificationStateChanges = adapter.apply(
          acpUpdate({
            sessionUpdate: 'tool_call_update',
            toolCallId: 'tool-1',
            status: 'in_progress',
            _meta: {
              toolNotification: {
                type: 'progress',
                params: {
                  progressToken: 'scan-repo',
                  progress: 3,
                  total: 10,
                  message: 'Scanned 3 of 10 directories',
                },
              },
            },
          })
        );
        const notification = expectOnlyNotificationChange(notificationStateChanges);

        expect(notification).toMatchObject({
          type: 'Notification',
          request_id: 'tool-1',
          message: {
            method: 'notifications/progress',
            params: {
              progressToken: 'scan-repo',
              progress: 3,
              total: 10,
              message: 'Scanned 3 of 10 directories',
            },
          },
        });
      });
    });
  });

  describe('applyGosling', () => {
    it('maps usage updates into token state', () => {
      const adapter = createAcpSessionNotificationAdapter();

      expect(
        adapter.applyGosling(
          goslingUpdate({
            sessionUpdate: 'usage_update',
            used: 42,
            contextLimit: 200,
            accumulatedInputTokens: 10,
            accumulatedOutputTokens: 15,
            accumulatedCost: 0.12,
          })
        )
      ).toEqual([
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

    it('maps status messages and keeps later id-less chunks separate', () => {
      const adapter = createAcpSessionNotificationAdapter();

      const noticeStateChanges = adapter.applyGosling(
        goslingUpdate({
          sessionUpdate: 'status_message',
          status: { type: 'notice', message: 'Checking files' },
        })
      );
      let messages = expectOnlyMessagesChange(adapter, noticeStateChanges);

      expect(messages).toHaveLength(1);
      expect(messages[0].metadata).toMatchObject({ userVisible: true, agentVisible: false });
      expect(firstContent(messages[0])).toMatchObject({
        type: 'systemNotification',
        notificationType: 'inlineMessage',
        msg: 'Checking files',
      });

      const textStateChanges = adapter.apply(agentText('Result'));
      messages = expectOnlyMessagesChange(adapter, textStateChanges);

      expect(messages).toHaveLength(2);
      expect(firstContent(messages[1])).toMatchObject({ type: 'text', text: 'Result' });

      const progressStateChanges = adapter.applyGosling(
        goslingUpdate({
          sessionUpdate: 'status_message',
          status: { type: 'progress', message: 'Still working' },
        })
      );
      messages = expectOnlyMessagesChange(adapter, progressStateChanges);

      expect(firstContent(messages[2])).toMatchObject({
        type: 'systemNotification',
        notificationType: 'thinkingMessage',
        msg: 'Still working',
      });
    });
  });

  describe('applyPermissionRequest', () => {
    it('maps permission requests to action-required tool confirmations', () => {
      const adapter = createAcpSessionNotificationAdapter();

      const request: RequestPermissionRequest = {
        sessionId: SESSION_ID,
        options: [{ optionId: 'allow', name: 'Allow', kind: 'allow_once' }],
        toolCall: {
          toolCallId: 'tool-1',
          title: 'Edit file',
          rawInput: { path: 'README.md' },
          content: [
            {
              type: 'content',
              content: { type: 'text', text: 'Allow editing README.md?' },
            },
          ],
          _meta: {
            gosling: {
              toolCall: {
                toolName: 'edit_file',
              },
            },
          },
        },
      };

      const permissionStateChanges = adapter.applyPermissionRequest(request);
      const messages = expectOnlyMessagesChange(adapter, permissionStateChanges);

      expect(messages).toHaveLength(1);
      expect(messages[0].role).toBe('assistant');
      expect(firstContent(messages[0])).toMatchObject({
        type: 'actionRequired',
        data: {
          actionType: 'toolConfirmation',
          id: 'tool-1',
          toolName: 'edit_file',
          arguments: { path: 'README.md' },
          prompt: 'Allow editing README.md?',
        },
      });
    });
  });
});
