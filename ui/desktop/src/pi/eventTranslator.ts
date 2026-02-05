/**
 * Translates Pi agent events to Goose-compatible message format.
 *
 * This allows the existing UI components to work with Pi as the backend,
 * maintaining a consistent user experience.
 */

import type { Message, MessageContent } from '../api';

// Pi event types (from @mariozechner/pi-ai and @mariozechner/pi-agent-core)

// Pi's content types within messages
export interface PiTextContent {
  type: 'text';
  text: string;
}

export interface PiThinkingContent {
  type: 'thinking';
  thinking: string;
}

export interface PiImageContent {
  type: 'image';
  data: string;
  mimeType: string;
}

export interface PiToolCall {
  type: 'toolCall';
  id: string;
  name: string;
  arguments: Record<string, unknown>;
}

export type PiAssistantContent = PiTextContent | PiThinkingContent | PiToolCall;
export type PiUserContent = string | (PiTextContent | PiImageContent)[];

// Pi message types
export interface PiUserMessage {
  role: 'user';
  content: PiUserContent;
  timestamp: number;
}

export interface PiAssistantMessage {
  role: 'assistant';
  content: PiAssistantContent[];
  timestamp: number;
  // Other fields we don't need: api, provider, model, usage, stopReason
}

export interface PiToolResultMessage {
  role: 'toolResult';
  toolCallId: string;
  toolName: string;
  content: (PiTextContent | PiImageContent)[];
  isError: boolean;
  timestamp: number;
}

export type PiMessage = PiUserMessage | PiAssistantMessage | PiToolResultMessage;

// Pi agent events
export interface PiAgentEvent {
  type: string;
  message?: PiMessage;
  toolCallId?: string;
  toolName?: string;
  args?: Record<string, unknown>;
  result?: { content: (PiTextContent | PiImageContent)[]; details?: unknown };
  partialResult?: { content: (PiTextContent | PiImageContent)[]; details?: unknown };
  isError?: boolean;
  [key: string]: unknown;
}

// Re-export Goose Message type for convenience
export type GooseMessage = Message;

// Tool notification type (matches Goose's Notification event structure)
export interface ToolNotification {
  type: 'Notification';
  request_id: string;
  message: {
    type: 'tool_start' | 'tool_progress' | 'tool_end';
    name?: string;
    args?: Record<string, unknown>;
    result?: unknown;
    isError?: boolean;
    progress?: string;
  };
}

/**
 * Translates a Pi message to Goose message format.
 */
export function translatePiMessage(piMessage: PiMessage): GooseMessage {
  const gooseContent: MessageContent[] = [];
  const messageId = `msg_${piMessage.timestamp}_${piMessage.role}`;

  if (piMessage.role === 'user') {
    // User message
    const content = piMessage.content;
    if (typeof content === 'string') {
      gooseContent.push({ type: 'text', text: content });
    } else if (Array.isArray(content)) {
      for (const item of content) {
        if (item.type === 'text') {
          gooseContent.push({ type: 'text', text: item.text });
        } else if (item.type === 'image') {
          gooseContent.push({
            type: 'image',
            data: item.data,
            mimeType: item.mimeType,
          });
        }
      }
    }
    return {
      id: messageId,
      role: 'user',
      created: Math.floor(piMessage.timestamp / 1000),
      content: gooseContent,
      metadata: { userVisible: true, agentVisible: true },
    };
  }

  if (piMessage.role === 'assistant') {
    // Assistant message with text, thinking, or tool calls
    for (const item of piMessage.content) {
      switch (item.type) {
        case 'text':
          gooseContent.push({ type: 'text', text: item.text });
          break;

        case 'thinking':
          gooseContent.push({
            type: 'thinking',
            thinking: item.thinking,
            signature: '',
          });
          break;

        case 'toolCall':
          gooseContent.push({
            type: 'toolRequest',
            id: item.id,
            toolCall: {
              name: item.name,
              arguments: item.arguments || {},
            },
          });
          break;
      }
    }
    return {
      id: messageId,
      role: 'assistant',
      created: Math.floor(piMessage.timestamp / 1000),
      content: gooseContent,
      metadata: { userVisible: true, agentVisible: true },
    };
  }

  if (piMessage.role === 'toolResult') {
    // Tool result message - translate to toolResponse content
    // Extract text from the tool result content
    const resultText = piMessage.content
      .filter((c): c is PiTextContent => c.type === 'text')
      .map((c) => c.text)
      .join('\n');

    gooseContent.push({
      type: 'toolResponse',
      id: piMessage.toolCallId,
      toolResult: {
        status: piMessage.isError ? 'error' : 'success',
        value: piMessage.isError
          ? undefined
          : {
              content: piMessage.content.map((c) =>
                c.type === 'text' ? { type: 'text' as const, text: c.text } : c
              ),
            },
        error: piMessage.isError ? resultText : undefined,
      },
    });
    return {
      id: messageId,
      role: 'user', // Tool results are typically shown as user role in Goose
      created: Math.floor(piMessage.timestamp / 1000),
      content: gooseContent,
      metadata: { userVisible: true, agentVisible: true },
    };
  }

  // Fallback - shouldn't happen
  return {
    id: messageId,
    role: 'assistant',
    created: Math.floor(Date.now() / 1000),
    content: gooseContent,
    metadata: { userVisible: true, agentVisible: true },
  };
}

function generateId(): string {
  return Math.random().toString(36).substring(2, 10);
}

/**
 * State machine for accumulating Pi events into Goose messages.
 */
export class PiEventAccumulator {
  private currentMessage: PiMessage | null = null;
  private messages: GooseMessage[] = [];
  private isStreaming = false;

  /**
   * Process a Pi event and return any completed Goose messages and notifications.
   */
  processEvent(event: PiAgentEvent): {
    message?: GooseMessage;
    notification?: ToolNotification;
    isComplete: boolean;
    isStreaming: boolean;
  } {
    switch (event.type) {
      case 'agent_start':
        this.isStreaming = true;
        this.messages = [];
        return { isComplete: false, isStreaming: true };

      case 'agent_end':
        this.isStreaming = false;
        return { isComplete: true, isStreaming: false };

      case 'message_start': {
        const msg = event.message;
        if (!msg) return { isComplete: false, isStreaming: true };
        this.currentMessage = msg;
        return {
          message: translatePiMessage(msg),
          isComplete: false,
          isStreaming: true,
        };
      }

      case 'message_update': {
        const msg = event.message;
        if (!msg) return { isComplete: false, isStreaming: true };
        this.currentMessage = msg;
        return {
          message: translatePiMessage(msg),
          isComplete: false,
          isStreaming: true,
        };
      }

      case 'message_end': {
        const msg = event.message;
        if (!msg) return { isComplete: false, isStreaming: true };
        this.currentMessage = null;
        const gooseMsg = translatePiMessage(msg);
        this.messages.push(gooseMsg);
        return {
          message: gooseMsg,
          isComplete: false,
          isStreaming: true,
        };
      }

      case 'turn_end': {
        // Turn end includes the final assistant message and tool results
        // The tool results are in event.toolResults
        const msg = event.message;
        if (msg) {
          const gooseMsg = translatePiMessage(msg);
          return {
            message: gooseMsg,
            isComplete: false,
            isStreaming: true,
          };
        }
        return { isComplete: false, isStreaming: true };
      }

      case 'tool_execution_start': {
        const notification: ToolNotification = {
          type: 'Notification',
          request_id: event.toolCallId || generateId(),
          message: {
            type: 'tool_start',
            name: event.toolName,
            args: event.args,
          },
        };
        return { notification, isComplete: false, isStreaming: true };
      }

      case 'tool_execution_update': {
        // Progress during tool execution
        const notification: ToolNotification = {
          type: 'Notification',
          request_id: event.toolCallId || generateId(),
          message: {
            type: 'tool_progress',
            name: event.toolName,
            progress: event.partialResult?.content
              ?.filter((c): c is PiTextContent => c.type === 'text')
              .map((c) => c.text)
              .join('\n'),
          },
        };
        return { notification, isComplete: false, isStreaming: true };
      }

      case 'tool_execution_end': {
        const notification: ToolNotification = {
          type: 'Notification',
          request_id: event.toolCallId || generateId(),
          message: {
            type: 'tool_end',
            name: event.toolName,
            result: event.result,
            isError: event.isError,
          },
        };
        return { notification, isComplete: false, isStreaming: true };
      }

      default:
        // Unknown event type, ignore
        return { isComplete: false, isStreaming: this.isStreaming };
    }
  }

  /**
   * Get all accumulated messages.
   */
  getMessages(): GooseMessage[] {
    return this.messages;
  }

  /**
   * Get the current streaming message (if any).
   */
  getCurrentMessage(): GooseMessage | null {
    return this.currentMessage ? translatePiMessage(this.currentMessage) : null;
  }

  /**
   * Reset the accumulator.
   */
  reset(): void {
    this.currentMessage = null;
    this.messages = [];
    this.isStreaming = false;
  }
}

/**
 * Create SSE-compatible event data from a Pi event.
 * This matches the format goosed uses for /reply SSE stream.
 */
export function piEventToSSE(event: PiAgentEvent): string | null {
  const result = new PiEventAccumulator().processEvent(event);

  if (result.message) {
    return JSON.stringify({
      type: 'message',
      data: result.message,
    });
  }

  if (result.isComplete) {
    return JSON.stringify({
      type: 'done',
    });
  }

  return null;
}
