import { Content, Message, ToolConfirmationRequest, ToolRequest, ToolResponse } from '../api';

export type ToolRequestMessageContent = ToolRequest & { type: 'toolRequest' };
export type ToolResponseMessageContent = ToolResponse & { type: 'toolResponse' };

export function createUserMessage(text: string): Message {
  return {
    id: generateId(),
    role: 'user',
    created: Math.floor(Date.now() / 1000),
    content: [{ type: 'text', text }],
  };
}

export function createAssistantMessage(text: string): Message {
  return {
    id: generateId(),
    role: 'assistant',
    created: Math.floor(Date.now() / 1000),
    content: [{ type: 'text', text }],
  };
}

export function createToolRequestMessage(
  id: string,
  toolName: string,
  args: Record<string, unknown>
): Message {
  return {
    id: generateId(),
    role: 'assistant',
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'toolRequest',
        id,
        toolCall: {
          status: 'success',
          value: {
            name: toolName,
            arguments: args,
          },
        },
      },
    ],
  };
}

export function createToolResponseMessage(id: string, result: Content[]): Message {
  return {
    id: generateId(),
    role: 'user',
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'toolResponse',
        id,
        toolResult: {
          status: 'success',
          value: result,
        },
      },
    ],
  };
}

export function createToolErrorResponseMessage(id: string, error: string): Message {
  return {
    id: generateId(),
    role: 'user',
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'toolResponse',
        id,
        toolResult: {
          status: 'error',
          error,
        },
      },
    ],
  };
}

function generateId(): string {
  return Math.random().toString(36).substring(2, 10);
}

export function getTextContent(message: Message): string {
  return message.content
    .map((content) => {
      if (content.type === 'text') return content.text;
      if (content.type === 'contextLengthExceeded') return content.msg;
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
): (ToolConfirmationRequest & { type: 'toolConfirmationRequest' }) | undefined {
  return message.content.find(
    (content): content is ToolConfirmationRequest & { type: 'toolConfirmationRequest' } =>
      content.type === 'toolConfirmationRequest'
  );
}

export function hasCompletedToolCalls(message: Message): boolean {
  const toolRequests = getToolRequests(message);
  return toolRequests.length > 0;
}
