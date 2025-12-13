import { Message, MessageEvent, ActionRequired, ToolRequest, ToolResponse } from '../api';

export type ToolRequestMessageContent = ToolRequest & { type: 'toolRequest' };
export type ToolResponseMessageContent = ToolResponse & { type: 'toolResponse' };
export type NotificationEvent = Extract<MessageEvent, { type: 'Notification' }>;

// Compaction response message - must match backend constant
const COMPACTION_THINKING_TEXT = 'goose is compacting the conversation...';

export function createUserMessage(text: string): Message {
  return {
    id: generateMessageId(),
    role: 'user',
    created: Math.floor(Date.now() / 1000),
    content: [{ type: 'text', text }],
    metadata: { userVisible: true, agentVisible: true },
  };
}

export function createElicitationResponseMessage(
  elicitationId: string,
  userData: Record<string, unknown>
): Message {
  return {
    id: generateMessageId(),
    role: 'user',
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'actionRequired',
        data: {
          actionType: 'elicitationResponse',
          id: elicitationId,
          user_data: userData,
        },
      },
    ],
    metadata: { userVisible: false, agentVisible: true },
  };
}

export function generateMessageId(): string {
  return Math.random().toString(36).substring(2, 10);
}

export function getTextContent(message: Message): string {
  return message.content
    .map((content) => {
      if (content.type === 'text') return content.text;
      return '';
    })
    .join('');
}

export function getToolRequests(message: Message): (ToolRequest & { type: 'toolRequest' })[] {
  return message.content.filter(
    (content): content is ToolRequest & { type: 'toolRequest' } => content.type === 'toolRequest'
  );
}

export function getToolResponses(message: Message): (ToolResponse & { type: 'toolResponse' })[] {
  return message.content.filter(
    (content): content is ToolResponse & { type: 'toolResponse' } => content.type === 'toolResponse'
  );
}

export function getToolConfirmationContent(
  message: Message
): (ActionRequired & { type: 'actionRequired' }) | undefined {
  return message.content.find(
    (content): content is ActionRequired & { type: 'actionRequired' } =>
      content.type === 'actionRequired' && content.data.actionType === 'toolConfirmation'
  );
}

export function getElicitationContent(
  message: Message
): (ActionRequired & { type: 'actionRequired' }) | undefined {
  return message.content.find(
    (content): content is ActionRequired & { type: 'actionRequired' } =>
      content.type === 'actionRequired' && content.data.actionType === 'elicitation'
  );
}

export function hasCompletedToolCalls(message: Message): boolean {
  const toolRequests = getToolRequests(message);
  return toolRequests.length > 0;
}

export function getThinkingMessage(message: Message | undefined): string | undefined {
  if (!message || message.role !== 'assistant') {
    return undefined;
  }

  for (const content of message.content) {
    if (content.type === 'systemNotification' && content.notificationType === 'thinkingMessage') {
      return content.msg;
    }
  }

  return undefined;
}

export function getCompactingMessage(message: Message | undefined): string | undefined {
  if (!message || message.role !== 'assistant') {
    return undefined;
  }

  for (const content of message.content) {
    if (content.type === 'systemNotification' && content.notificationType === 'thinkingMessage') {
      if (content.msg === COMPACTION_THINKING_TEXT) {
        return content.msg;
      }
    }
  }

  return undefined;
}
