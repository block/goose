/**
 * Message types that match the Rust message structures
 * for direct serialization between client and server
 */

export type Role = 'user' | 'assistant';

export interface TextContent {
  type: 'text';
  text: string;
  annotations?: Record<string, unknown>;
}

export interface ImageContent {
  type: 'image';
  data: string;
  mimeType: string;
  annotations?: Record<string, unknown>;
}

export type Content = TextContent | ImageContent;

export interface ToolCall {
  name: string;
  arguments: Record<string, unknown>;
}

export interface ToolCallResult<T> {
  status: 'success' | 'error';
  value?: T;
  error?: string;
}

export interface ToolRequest {
  id: string;
  toolCall: ToolCallResult<ToolCall>;
}

export interface ToolResponse {
  id: string;
  toolResult: ToolCallResult<Content[]>;
}

export interface ToolConfirmationRequest {
  id: string;
  toolName: string;
  arguments: Record<string, unknown>;
  prompt?: string;
}

export interface ToolRequestMessageContent {
  type: 'toolRequest';
  id: string;
  toolCall: ToolCallResult<ToolCall>;
}

export interface ToolResponseMessageContent {
  type: 'toolResponse';
  id: string;
  toolResult: ToolCallResult<Content[]>;
}

export interface ToolConfirmationRequestMessageContent {
  type: 'toolConfirmationRequest';
  id: string;
  toolName: string;
  arguments: Record<string, unknown>;
  prompt?: string;
}

export interface EnableExtensionCall {
  name: string;
  arguments: Record<string, unknown>;
  extensionName: string;
}

export interface EnableExtensionCallResult<T> {
  status: 'success' | 'error';
  value?: T;
  error?: string;
}

export interface EnableExtensionRequest {
  id: string;
  extensionCall: EnableExtensionCallResult<EnableExtensionCall>;
}

export interface EnableExtensionConfirmationRequest {
  id: string;
  extensionName: string;
  arguments: Record<string, unknown>;
  prompt?: string;
}

export interface EnableExtensionRequestMessageContent {
  type: 'enableExtensionRequest';
  id: string;
  extensionCall: EnableExtensionCallResult<EnableExtensionCall>;
  extensionName: string;
}

export type MessageContent =
  | TextContent
  | ImageContent
  | ToolRequestMessageContent
  | ToolResponseMessageContent
  | ToolConfirmationRequestMessageContent
  | EnableExtensionRequestMessageContent;

export interface Message {
  id?: string;
  role: Role;
  created: number;
  content: MessageContent[];
}

// Helper functions to create messages
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

// Generate a unique ID for messages
function generateId(): string {
  return Math.random().toString(36).substring(2, 10);
}

// Helper functions to extract content from messages
export function getTextContent(message: Message): string {
  return message.content
    .filter((content): content is TextContent => content.type === 'text')
    .map((content) => content.text)
    .join('\n');
}

export function getToolRequests(message: Message): ToolRequestMessageContent[] {
  return message.content.filter(
    (content): content is ToolRequestMessageContent => content.type === 'toolRequest'
  );
}

export function getToolResponses(message: Message): ToolResponseMessageContent[] {
  return message.content.filter(
    (content): content is ToolResponseMessageContent => content.type === 'toolResponse'
  );
}

export function getEnableExtensionRequests(
  message: Message
): EnableExtensionRequestMessageContent[] {
  return message.content.filter(
    (content): content is EnableExtensionRequestMessageContent =>
      content.type === 'enableExtensionRequest'
  );
}

export function getToolConfirmationContent(
  message: Message
): ToolConfirmationRequestMessageContent {
  return message.content.find(
    (content): content is ToolConfirmationRequestMessageContent =>
      content.type === 'toolConfirmationRequest'
  );
}

export function getEnableExtensionContent(message: Message): EnableExtensionRequestMessageContent {
  return message.content.find(
    (content): content is EnableExtensionRequestMessageContent =>
      content.type === 'enableExtensionRequest'
  );
}

export function hasCompletedToolCalls(message: Message): boolean {
  const toolRequests = getToolRequests(message);
  if (toolRequests.length === 0) return false;

  // For now, we'll assume all tool calls are completed when this is checked
  // In a real implementation, you'd need to check if all tool requests have responses
  // by looking through subsequent messages
  return true;
}

export function hasCompletedEnableExtensionCalls(message: Message): boolean {
  const extensionRequests = getEnableExtensionRequests(message);
  if (extensionRequests.length === 0) return false;

  // For now, we'll assume all extension calls are completed when this is checked
  // In a real implementation, you'd need to check if all extension requests have responses
  // by looking through subsequent messages
  return true;
}
