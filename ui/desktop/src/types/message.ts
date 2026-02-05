import { Message, MessageEvent, ActionRequired, ToolRequest, ToolResponse } from '../api';

export type ToolRequestMessageContent = ToolRequest & { type: 'toolRequest' };
export type ToolResponseMessageContent = ToolResponse & { type: 'toolResponse' };
export type NotificationEvent = Extract<MessageEvent, { type: 'Notification' }>;

// Compaction response message - must match backend constant
const COMPACTION_THINKING_TEXT = 'goose is compacting the conversation...';

export interface ImageData {
  data: string; // base64 encoded image data
  mimeType: string;
}

export interface UserInput {
  msg: string;
  images: ImageData[];
}

export function createUserMessage(text: string, images?: ImageData[]): Message {
  const content: Message['content'] = [];

  if (text.trim()) {
    content.push({ type: 'text', text });
  }

  if (images && images.length > 0) {
    images.forEach((img) => {
      content.push({
        type: 'image',
        data: img.data,
        mimeType: img.mimeType,
      });
    });
  }

  const message: Message = {
    id: generateMessageId(),
    role: 'user',
    created: Math.floor(Date.now() / 1000),
    content,
    metadata: { userVisible: true, agentVisible: true },
  };

  // DEBUG: Log user message creation
  console.log('[DEBUG createUserMessage] Created user message:', JSON.stringify({
    id: message.id,
    text: text.substring(0, 100),
    textLength: text.length,
    textTrimmedLength: text.trim().length,
    imageCount: images?.length ?? 0,
    contentTypes: content.map((c) => c.type),
    metadata: message.metadata,
  }, null, 2));

  return message;
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

export function getTextAndImageContent(message: Message): {
  textContent: string;
  imagePaths: string[];
} {
  let textContent = '';
  const imagePaths: string[] = [];

  for (const content of message.content) {
    if (content.type === 'text') {
      textContent += content.text;
    } else if (content.type === 'image') {
      imagePaths.push(`data:${content.mimeType};base64,${content.data}`);
    }
  }

  // DEBUG: Log text/image extraction
  console.log('[DEBUG getTextAndImageContent] Extracted content:', JSON.stringify({
    messageId: message.id,
    messageRole: message.role,
    inputContentTypes: message.content.map((c) => c.type),
    extractedTextLength: textContent.length,
    extractedTextTrimmedLength: textContent.trim().length,
    extractedTextPreview: textContent.substring(0, 100),
    extractedImageCount: imagePaths.length,
  }, null, 2));

  return { textContent, imagePaths };
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

export function getToolConfirmationId(
  content: ActionRequired & { type: 'actionRequired' }
): string | undefined {
  if (content.data.actionType === 'toolConfirmation') {
    return content.data.id;
  }
  return undefined;
}

export function getPendingToolConfirmationIds(messages: Message[]): Set<string> {
  const pendingIds = new Set<string>();
  const respondedIds = new Set<string>();

  for (const message of messages) {
    const responses = getToolResponses(message);
    for (const response of responses) {
      respondedIds.add(response.id);
    }
  }

  for (const message of messages) {
    const confirmation = getToolConfirmationContent(message);
    if (confirmation) {
      const confirmationId = getToolConfirmationId(confirmation);
      if (confirmationId && !respondedIds.has(confirmationId)) {
        pendingIds.add(confirmationId);
      }
    }
  }

  return pendingIds;
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
