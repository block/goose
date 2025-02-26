/**
 * Message types that match the Rust message structures
 * for direct serialization between client and server
 */

export type Role = 'user' | 'assistant';

export interface TextContent {
  text: string;
  annotations?: Record<string, unknown>;
}

export interface ImageContent {
  data: string;
  mime_type: string;
  annotations?: Record<string, unknown>;
}

export interface ToolCall {
  name: string;
  arguments: Record<string, unknown>;
}

export type Content = { Text: TextContent } | { Image: ImageContent };

export interface ToolRequest {
  id: string;
  tool_call: {
    Ok?: ToolCall;
    Err?: string;
  };
}

export interface ToolResponse {
  id: string;
  tool_result: {
    Ok?: Content[];
    Err?: string;
  };
}

export interface ToolConfirmationRequest {
  id: string;
  tool_name: string;
  arguments: Record<string, unknown>;
  prompt?: string;
}

export type MessageContent =
  | { Text: TextContent }
  | { Image: ImageContent }
  | { ToolRequest: ToolRequest }
  | { ToolResponse: ToolResponse }
  | { ToolConfirmationRequest: ToolConfirmationRequest };

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
    content: [{ Text: { text } }],
  };
}

export function createAssistantMessage(text: string): Message {
  return {
    id: generateId(),
    role: 'assistant',
    created: Math.floor(Date.now() / 1000),
    content: [{ Text: { text } }],
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
        ToolRequest: {
          id,
          tool_call: {
            Ok: {
              // Using Ok to match the server format
              name: toolName,
              arguments: args,
            },
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
        ToolResponse: {
          id,
          tool_result: {
            Ok: result, // Using Ok to match the server format
          },
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
        ToolResponse: {
          id,
          tool_result: {
            Err: error, // Using Err to match the server format
          },
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
    .filter((content): content is { Text: TextContent } => 'Text' in content)
    .map((content) => content.Text.text)
    .join('\n');
}

export function getToolRequests(message: Message): ToolRequest[] {
  // Try both casing variations
  return message.content
    .filter(
      (content): content is { ToolRequest: ToolRequest } | { toolRequest: ToolRequest } =>
        'ToolRequest' in content || 'toolRequest' in content
    )
    .map((content) => {
      if ('ToolRequest' in content) {
        return content.ToolRequest;
      } else {
        // Handle potential lowercase property name
        return (content as { toolRequest: ToolRequest }).toolRequest;
      }
    });
}

export function getToolResponses(message: Message): ToolResponse[] {
  // Try both casing variations
  return message.content
    .filter(
      (content): content is { ToolResponse: ToolResponse } | { toolResponse: ToolResponse } =>
        'ToolResponse' in content || 'toolResponse' in content
    )
    .map((content) => {
      if ('ToolResponse' in content) {
        return content.ToolResponse;
      } else {
        // Handle potential lowercase property name
        return (content as { toolResponse: ToolResponse }).toolResponse;
      }
    });
}

export function hasCompletedToolCalls(message: Message): boolean {
  const toolRequests = getToolRequests(message);
  if (toolRequests.length === 0) return false;

  // For now, we'll assume all tool calls are completed when this is checked
  // In a real implementation, you'd need to check if all tool requests have responses
  // by looking through subsequent messages
  return true;
}
